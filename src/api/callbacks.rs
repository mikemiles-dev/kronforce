use axum::Json;
use axum::extract::State;
use tracing::info;
use uuid::Uuid;

use super::auth::AuthUser;
use super::{AppState, log_and_notify};
use crate::agent::protocol::ExecutionResultReport;
use crate::db::db_call;
use crate::db::models::*;
use crate::error::AppError;
use crate::executor::notifications::notify_execution_complete;
use crate::executor::output_rules::process_post_execution;
use crate::scheduler::SchedulerCommand;

/// Receives execution results from agents, updates the execution record,
/// runs output rules, sends notifications, and logs the completion event.
pub(crate) async fn execution_result_callback(
    State(state): State<AppState>,
    Json(result): Json<ExecutionResultReport>,
) -> Result<Json<serde_json::Value>, AppError> {
    let exec_id = result.execution_id;

    // Get existing execution to preserve triggered_by
    let existing = db_call(&state.db, move |db| db.get_execution(exec_id)).await?;

    let task_snap = existing.as_ref().and_then(|e| e.task_snapshot.clone());
    let retry_of = existing.as_ref().and_then(|e| e.retry_of);
    let attempt_number = existing.as_ref().map(|e| e.attempt_number).unwrap_or(1);
    let triggered_by = existing
        .map(|e| e.triggered_by)
        .unwrap_or(TriggerSource::Scheduler);

    let rec = ExecutionRecord {
        id: result.execution_id,
        job_id: result.job_id,
        agent_id: Some(result.agent_id),
        task_snapshot: task_snap,
        status: result.status,
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
        stdout_truncated: result.stdout_truncated,
        stderr_truncated: result.stderr_truncated,
        started_at: Some(result.started_at),
        finished_at: Some(result.finished_at),
        triggered_by,
        extracted: None,
        retry_of,
        attempt_number,
        params: None,
    };

    let stdout_for_rules = rec.stdout.clone();
    let stderr_for_rules = rec.stderr.clone();
    let stderr_for_notif = rec.stderr.clone();
    let job_id_for_rules = rec.job_id;
    let exec_id_for_rules = rec.id;

    let status = rec.status;
    let result_job_id = result.job_id;
    let result_exec_id = result.execution_id;
    let result_agent_id = result.agent_id;
    db_call(&state.db, move |db| db.update_execution(&rec)).await?;

    // Run output rules (extraction + triggers), log event, and notify scheduler
    let db_rules = state.db.clone();
    let sched_tx = state.scheduler_tx.clone();
    tokio::task::spawn(async move {
        let db_r = db_rules.clone();
        let stdout_r = stdout_for_rules;
        let stderr_r = stderr_for_rules;
        let output_events: Vec<Event> = tokio::task::spawn_blocking(move || {
            if let Ok(Some(job)) = db_r.get_job(job_id_for_rules) {
                process_post_execution(&db_r, &job, exec_id_for_rules, &stdout_r, &stderr_r, status)
            } else {
                Vec::new()
            }
        })
        .await
        .unwrap_or_default();
        for event in output_events {
            let _ = sched_tx.send(SchedulerCommand::EventOccurred(event)).await;
        }

        // Log execution.completed event (same as local execution path)
        let severity = match status {
            ExecutionStatus::Succeeded => EventSeverity::Success,
            ExecutionStatus::Failed | ExecutionStatus::TimedOut => EventSeverity::Error,
            ExecutionStatus::Cancelled => EventSeverity::Warning,
            _ => EventSeverity::Info,
        };
        let job_name = db_rules
            .get_job(result_job_id)
            .ok()
            .flatten()
            .map(|j| j.name)
            .unwrap_or_else(|| result_job_id.to_string());
        let event = Event {
            id: Uuid::new_v4(),
            kind: "execution.completed".to_string(),
            severity,
            message: format!(
                "Job '{}' execution {} finished: {:?}",
                job_name, result_exec_id, status
            ),
            job_id: Some(result_job_id),
            agent_id: Some(result_agent_id),
            api_key_id: None,
            api_key_name: None,
            details: None,
            timestamp: chrono::Utc::now(),
        };
        let db_ev = db_rules.clone();
        let event2 = event.clone();
        let _ = tokio::task::spawn_blocking(move || db_ev.insert_event(&event2)).await;
        let _ = sched_tx.send(SchedulerCommand::EventOccurred(event)).await;
    });

    // Send notifications based on job config
    {
        let db_notif = state.db.clone();
        let job_id_notif = result.job_id;
        let exec_status = status;
        let exec_id_short = result.execution_id.to_string()[..8].to_string();
        let stderr_excerpt: String = stderr_for_notif.chars().take(500).collect();
        tokio::spawn(async move {
            let job = match tokio::task::spawn_blocking({
                let db = db_notif.clone();
                move || db.get_job(job_id_notif)
            })
            .await
            {
                Ok(Ok(Some(j))) => j,
                _ => return,
            };
            if let Some(ref notif) = job.notifications {
                notify_execution_complete(
                    &db_notif,
                    notif,
                    &job.name,
                    &exec_id_short,
                    exec_status,
                    &stderr_excerpt,
                )
                .await;
            }
        });
    }

    let severity = match status {
        ExecutionStatus::Succeeded => EventSeverity::Success,
        ExecutionStatus::Failed | ExecutionStatus::TimedOut => EventSeverity::Error,
        _ => EventSeverity::Info,
    };
    // Log event and notify scheduler for event-triggered jobs
    let no_auth = AuthUser(None);
    let msg = format!("Execution {} finished: {:?}", result.execution_id, status);
    // Mark queue item complete (for custom agents)
    let eid = result.execution_id;
    let _ = db_call(&state.db, move |db| db.complete_queue_item(eid)).await;

    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "execution.completed",
        severity,
        &msg,
        Some(result.job_id),
        Some(result.agent_id),
        &no_auth,
        None,
    )
    .await;

    info!(
        "received execution result {} from agent {}: {:?}",
        result.execution_id, result.agent_id, status
    );

    // Schedule retry for agent-dispatched jobs if applicable
    {
        use crate::executor::{calculate_retry_delay, should_retry};
        let job_id = result.job_id;
        let exec_id = result.execution_id;
        let db_retry = state.db.clone();
        let sched_retry = state.scheduler_tx.clone();
        tokio::spawn(async move {
            let job = match tokio::task::spawn_blocking({
                let db = db_retry.clone();
                move || db.get_job(job_id)
            })
            .await
            {
                Ok(Ok(Some(j))) => j,
                _ => return,
            };
            if should_retry(job.retry_max, status, attempt_number) {
                let next_attempt = attempt_number + 1;
                let delay =
                    calculate_retry_delay(job.retry_delay_secs, job.retry_backoff, next_attempt);
                let original_id = retry_of.unwrap_or(exec_id);
                if delay > 0 {
                    tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                }
                let _ = sched_retry
                    .send(SchedulerCommand::RetryExecution {
                        job_id,
                        original_execution_id: original_id,
                        attempt: next_attempt,
                    })
                    .await;
            }
        });
    }

    Ok(Json(serde_json::json!({"status": "ok"})))
}
