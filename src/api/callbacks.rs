use axum::Json;
use axum::extract::State;
use uuid::Uuid;

use super::auth::AuthUser;
use super::{AppState, log_and_notify};
use crate::error::AppError;
use crate::models::*;
use crate::protocol::ExecutionResultReport;
use crate::scheduler::SchedulerCommand;

pub(crate) async fn execution_result_callback(
    State(state): State<AppState>,
    Json(result): Json<ExecutionResultReport>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.clone();
    let exec_id = result.execution_id;

    // Get existing execution to preserve triggered_by
    let db2 = db.clone();
    let existing = tokio::task::spawn_blocking(move || db2.get_execution(exec_id))
        .await
        .unwrap()?;

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
    tokio::task::spawn_blocking(move || db.update_execution(&rec))
        .await
        .unwrap()?;

    // Run output rules (extraction + triggers) and notify scheduler
    let db_rules = state.db.clone();
    let sched_tx = state.scheduler_tx.clone();
    tokio::task::spawn(async move {
        let db_r = db_rules.clone();
        let stdout_r = stdout_for_rules;
        let stderr_r = stderr_for_rules;
        let output_events: Vec<Event> = tokio::task::spawn_blocking(move || {
            let mut events = Vec::new();
            if let Ok(Some(job)) = db_r.get_job(job_id_for_rules)
                && let Some(ref rules) = job.output_rules
            {
                if !rules.extractions.is_empty() {
                    let extracted =
                        crate::output_rules::run_extractions(&stdout_r, &rules.extractions);
                    if !extracted.is_empty() {
                        let _ = db_r.update_execution_extracted(
                            exec_id_for_rules,
                            &serde_json::json!(extracted),
                        );
                    }
                }
                // Assertions — only on successful executions
                if status == ExecutionStatus::Succeeded && !rules.assertions.is_empty() {
                    let failures =
                        crate::output_rules::run_assertions(&stdout_r, &rules.assertions);
                    if !failures.is_empty() {
                        let msg = failures.join("; ");
                        let _ = db_r.fail_execution_assertion(exec_id_for_rules, &msg);
                    }
                }
                let matches =
                    crate::output_rules::run_triggers(&stdout_r, &stderr_r, &rules.triggers);
                for (pattern, severity) in &matches {
                    let sev = match severity.as_str() {
                        "error" => EventSeverity::Error,
                        "warning" => EventSeverity::Warning,
                        "success" => EventSeverity::Success,
                        _ => EventSeverity::Info,
                    };
                    let event = Event {
                        id: Uuid::new_v4(),
                        kind: "output.matched".to_string(),
                        severity: sev,
                        message: format!(
                            "Output pattern matched: '{}' in job '{}'",
                            pattern, job.name
                        ),
                        job_id: Some(job_id_for_rules),
                        agent_id: None,
                        api_key_id: None,
                        api_key_name: None,
                        details: None,
                        timestamp: chrono::Utc::now(),
                    };
                    let _ = db_r.insert_event(&event);
                    events.push(event);
                }
            }
            events
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
            let notif = match &job.notifications {
                Some(n) => n,
                None => return,
            };
            let should_notify = match exec_status {
                ExecutionStatus::Failed | ExecutionStatus::TimedOut => {
                    notif.on_failure || notif.on_assertion_failure
                }
                ExecutionStatus::Succeeded => notif.on_success,
                _ => false,
            };
            if !should_notify {
                return;
            }
            let subject = format!(
                "[Kronforce] Job '{}' {}",
                job.name,
                match exec_status {
                    ExecutionStatus::Succeeded => "succeeded",
                    ExecutionStatus::Failed => "failed",
                    ExecutionStatus::TimedOut => "timed out",
                    _ => "completed",
                }
            );
            let body = format!(
                "Job: {}\nStatus: {:?}\nExecution: {}\nTime: {}\n{}",
                job.name,
                exec_status,
                exec_id_short,
                chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC"),
                if !stderr_excerpt.is_empty() {
                    format!("\nError output:\n{}", stderr_excerpt)
                } else {
                    String::new()
                }
            );
            let recipients =
                notif
                    .recipients
                    .as_ref()
                    .map(|r| crate::notifications::NotificationRecipients {
                        emails: r.emails.clone(),
                        phones: r.phones.clone(),
                    });
            crate::notifications::send_notification(
                &db_notif,
                &subject,
                &body,
                recipients.as_ref(),
            )
            .await;
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
    let db_q = state.db.clone();
    let eid = result.execution_id;
    let _ = tokio::task::spawn_blocking(move || db_q.complete_queue_item(eid)).await;

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
