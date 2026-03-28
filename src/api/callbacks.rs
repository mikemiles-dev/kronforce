use axum::Json;
use axum::extract::State;

use super::auth::AuthUser;
use super::{AppState, log_and_notify};
use crate::db::db_call;
use crate::error::AppError;
use crate::models::*;
use crate::protocol::ExecutionResultReport;
use crate::scheduler::SchedulerCommand;

pub(crate) async fn execution_result_callback(
    State(state): State<AppState>,
    Json(result): Json<ExecutionResultReport>,
) -> Result<Json<serde_json::Value>, AppError> {
    let exec_id = result.execution_id;

    // Get existing execution to preserve triggered_by
    let existing = db_call(&state.db, move |db| db.get_execution(exec_id)).await?;

    let task_snap = existing.as_ref().and_then(|e| e.task_snapshot.clone());
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
    };

    let stdout_for_rules = rec.stdout.clone();
    let stderr_for_rules = rec.stderr.clone();
    let stderr_for_notif = rec.stderr.clone();
    let job_id_for_rules = rec.job_id;
    let exec_id_for_rules = rec.id;

    let status = rec.status;
    db_call(&state.db, move |db| db.update_execution(&rec)).await?;

    // Run output rules (extraction + triggers) and notify scheduler
    let db_rules = state.db.clone();
    let sched_tx = state.scheduler_tx.clone();
    tokio::task::spawn(async move {
        let db_r = db_rules.clone();
        let stdout_r = stdout_for_rules;
        let stderr_r = stderr_for_rules;
        let output_events: Vec<Event> = tokio::task::spawn_blocking(move || {
            if let Ok(Some(job)) = db_r.get_job(job_id_for_rules) {
                crate::output_rules::process_post_execution(
                    &db_r,
                    &job,
                    exec_id_for_rules,
                    &stdout_r,
                    &stderr_r,
                    status,
                )
            } else {
                Vec::new()
            }
        })
        .await
        .unwrap_or_default();
        // Notify scheduler so event-triggered jobs can fire
        for event in output_events {
            let _ = sched_tx.send(SchedulerCommand::EventOccurred(event)).await;
        }
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
                crate::notifications::notify_execution_complete(
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

    tracing::info!(
        "received execution result {} from agent {}: {:?}",
        result.execution_id,
        result.agent_id,
        status
    );

    Ok(Json(serde_json::json!({"status": "ok"})))
}
