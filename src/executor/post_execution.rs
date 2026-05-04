//! Post-execution processing: result persistence, output rules, notifications,
//! dependency cascading, and event logging.

use tracing::{error, info, warn};
use uuid::Uuid;

use crate::db::Db;
use crate::db::models::*;
use crate::error::AppError;
use crate::executor::notifications::notify_execution_complete;
use crate::executor::output_rules::process_post_execution;
use crate::scheduler::SchedulerCommand;

impl super::Executor {
    /// Handles all post-execution work: persists the result, runs output rules,
    /// sends notifications, and logs the completion event.
    pub(crate) async fn handle_execution_complete(
        exec_id: Uuid,
        updated: &ExecutionRecord,
        db: &Db,
        sched_tx: &tokio::sync::mpsc::Sender<SchedulerCommand>,
    ) {
        let db2 = db.clone();
        let updated2 = updated.clone();
        if let Err(e) = tokio::task::spawn_blocking(move || db2.update_execution(&updated2)).await {
            error!("failed to update execution {}: {e}", exec_id);
        }

        let output_events = Self::run_output_rules(db, updated, exec_id).await;
        for event in output_events {
            let _ = sched_tx.send(SchedulerCommand::EventOccurred(event)).await;
        }

        // Output forwarding
        Self::forward_output(db, updated).await;

        // Dependency cascade: if this execution succeeded, trigger on-demand jobs
        // that depend on this job and now have all dependencies satisfied
        if updated.status == ExecutionStatus::Succeeded {
            Self::cascade_dependencies(db, updated.job_id, sched_tx).await;
        }

        Self::send_execution_notifications(db, updated, exec_id).await;

        info!("execution {} finished: {:?}", exec_id, updated.status);
        let severity = match updated.status {
            ExecutionStatus::Succeeded => EventSeverity::Success,
            ExecutionStatus::Failed | ExecutionStatus::TimedOut => EventSeverity::Error,
            ExecutionStatus::Cancelled => EventSeverity::Warning,
            _ => EventSeverity::Info,
        };
        let event = Event {
            id: Uuid::new_v4(),
            kind: "execution.completed".to_string(),
            severity,
            message: {
                // Look up job name for richer event message
                let job_name = db
                    .get_job(updated.job_id)
                    .ok()
                    .flatten()
                    .map(|j| j.name)
                    .unwrap_or_else(|| updated.job_id.to_string());
                format!(
                    "Job '{}' execution {} finished: {:?}",
                    job_name, exec_id, updated.status
                )
            },
            job_id: Some(updated.job_id),
            agent_id: None,
            api_key_id: None,
            api_key_name: None,
            execution_id: Some(exec_id),
            details: None,
            timestamp: chrono::Utc::now(),
        };
        let db3 = db.clone();
        let event2 = event.clone();
        let _ = tokio::task::spawn_blocking(move || db3.insert_event(&event2)).await;
        let _ = sched_tx.send(SchedulerCommand::EventOccurred(event)).await;
    }

    pub(crate) async fn cascade_dependencies(
        db: &Db,
        completed_job_id: Uuid,
        sched_tx: &tokio::sync::mpsc::Sender<SchedulerCommand>,
    ) {
        let db2 = db.clone();
        let dependents: Vec<Job> = tokio::task::spawn_blocking(move || {
            // Get all jobs, find ones that depend on the completed job
            let all_jobs = db2.list_jobs(None, None, None, 1000, 0)?;
            Ok::<Vec<Job>, AppError>(
                all_jobs
                    .into_iter()
                    .filter(|j| j.depends_on.iter().any(|d| d.job_id == completed_job_id))
                    .collect(),
            )
        })
        .await
        .unwrap_or(Ok(vec![]))
        .unwrap_or_default();

        for job in dependents {
            // Only cascade to on-demand or scheduled jobs that aren't event-triggered
            let db3 = db.clone();
            let deps = job.depends_on.clone();
            let satisfied = tokio::task::spawn_blocking(move || {
                use crate::dag::DagResolver;
                let dag = DagResolver::new(db3);
                dag.deps_satisfied(&deps)
            })
            .await
            .unwrap_or(Ok(false));

            if let Ok(true) = satisfied {
                info!(
                    "dependency cascade: triggering '{}' after '{}' completed",
                    job.name, completed_job_id
                );
                let _ = sched_tx
                    .send(SchedulerCommand::TriggerNow {
                        job_id: job.id,
                        skip_deps: false,
                        params: None,
                    })
                    .await;
            }
        }

        // Check if all jobs in the completed job's group are now done
        Self::check_group_completed(db, completed_job_id, sched_tx).await;
    }

    pub(crate) async fn check_group_completed(
        db: &Db,
        job_id: Uuid,
        sched_tx: &tokio::sync::mpsc::Sender<SchedulerCommand>,
    ) {
        let db2 = db.clone();
        let result = tokio::task::spawn_blocking(move || {
            let job = db2.get_job(job_id)?;
            let Some(job) = job else {
                return Ok::<Option<(String, bool)>, AppError>(None);
            };
            let group = job.group.clone().unwrap_or_else(|| "Default".to_string());
            // Get all jobs in this group
            let group_jobs = db2.list_jobs(None, None, Some(&group), 1000, 0)?;
            if group_jobs.len() <= 1 {
                return Ok(None);
            }
            // Check if all have a recent successful execution
            let all_succeeded = group_jobs.iter().all(|j| {
                db2.get_latest_execution_for_job(j.id)
                    .ok()
                    .flatten()
                    .is_some_and(|e| e.status == ExecutionStatus::Succeeded)
            });
            Ok(Some((group, all_succeeded)))
        })
        .await
        .unwrap_or(Ok(None));

        if let Ok(Some((group, true))) = result {
            let event = Event {
                id: Uuid::new_v4(),
                kind: "group.completed".to_string(),
                severity: EventSeverity::Success,
                message: format!("All jobs in group '{}' have succeeded", group),
                job_id: Some(job_id),
                agent_id: None,
                api_key_id: None,
                api_key_name: None,
                execution_id: None,
                details: Some(group),
                timestamp: chrono::Utc::now(),
            };
            let db3 = db.clone();
            let event2 = event.clone();
            let _ = tokio::task::spawn_blocking(move || db3.insert_event(&event2)).await;
            let _ = sched_tx.send(SchedulerCommand::EventOccurred(event)).await;
        }
    }

    pub(crate) async fn forward_output(db: &Db, updated: &ExecutionRecord) {
        let db_fwd = db.clone();
        let job_id = updated.job_id;
        let forward_url = tokio::task::spawn_blocking(move || {
            db_fwd
                .get_job(job_id)
                .ok()
                .flatten()
                .and_then(|j| j.output_rules)
                .and_then(|r| r.forward_url)
        })
        .await
        .unwrap_or(None);

        if let Some(url) = forward_url {
            let payload = serde_json::json!({
                "job_id": updated.job_id,
                "execution_id": updated.id,
                "status": updated.status,
                "exit_code": updated.exit_code,
                "stdout": &updated.stdout[..updated.stdout.len().min(100_000)],
                "stderr": &updated.stderr[..updated.stderr.len().min(100_000)],
                "started_at": updated.started_at,
                "finished_at": updated.finished_at,
            });
            let client = reqwest::Client::new();
            if let Err(e) = client
                .post(&url)
                .json(&payload)
                .timeout(std::time::Duration::from_secs(10))
                .send()
                .await
            {
                warn!("output forwarding to {} failed: {e}", url);
            }
        }
    }

    pub(crate) async fn run_output_rules(
        db: &Db,
        updated: &ExecutionRecord,
        exec_id: Uuid,
    ) -> Vec<Event> {
        let db_rules = db.clone();
        let stdout_clone = updated.stdout.clone();
        let stderr_clone = updated.stderr.clone();
        let job_id = updated.job_id;
        let exec_status = updated.status;
        tokio::task::spawn_blocking(move || {
            if let Ok(Some(job)) = db_rules.get_job(job_id) {
                process_post_execution(
                    &db_rules,
                    &job,
                    exec_id,
                    &stdout_clone,
                    &stderr_clone,
                    exec_status,
                )
            } else {
                Vec::new()
            }
        })
        .await
        .unwrap_or_default()
    }

    pub(crate) async fn send_execution_notifications(
        db: &Db,
        updated: &ExecutionRecord,
        exec_id: Uuid,
    ) {
        let db_notif = db.clone();
        let job_id_notif = updated.job_id;
        let exec_status = updated.status;
        let exec_id_short = exec_id.to_string()[..8].to_string();
        let stderr_excerpt = updated.stderr.chars().take(500).collect::<String>();
        let stdout_full = updated.stdout.clone();
        let stderr_full = updated.stderr.clone();
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
                    &stdout_full,
                    &stderr_full,
                )
                .await;
            }
        });
    }
}
