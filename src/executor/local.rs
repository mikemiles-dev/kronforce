//! Local job execution: spawns tasks on the controller node.

use tracing::info;

use chrono::Utc;
use tokio::sync::oneshot;
use uuid::Uuid;

use crate::db::models::*;
use crate::error::AppError;
use crate::executor::runner::run_task_streaming;
use crate::executor::utils::{calculate_retry_delay, should_retry};
use crate::scheduler::SchedulerCommand;

impl super::Executor {
    /// Executes a job locally on the controller, spawning the task in a background tokio task.
    pub(crate) async fn execute_local(
        &self,
        job: &Job,
        trigger: TriggerSource,
        params: Option<serde_json::Value>,
    ) -> Result<Uuid, AppError> {
        let exec_id = Uuid::new_v4();
        let now = Utc::now();

        let mut rec = ExecutionRecord::new(exec_id, job.id, trigger.clone())
            .with_status(ExecutionStatus::Running)
            .with_task_snapshot(job.task.clone())
            .with_started_at(now);
        rec.params = params;

        // Set retry tracking fields from trigger source
        if let TriggerSource::Retry {
            original_execution_id,
            attempt,
        } = &trigger
        {
            rec.retry_of = Some(*original_execution_id);
            rec.attempt_number = *attempt;
        }

        let db = self.db.clone();
        let rec_clone = rec.clone();
        tokio::task::spawn_blocking(move || db.insert_execution(&rec_clone))
            .await
            .map_err(|e| AppError::Internal(e.to_string()))??;

        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        {
            let mut running = self.running.lock().await;
            running.insert(exec_id, super::RunningJob { cancel_tx });
        }

        let task = job.task.clone();
        let run_as = job.run_as.clone();
        let timeout_secs = job.timeout_secs;
        let db = self.db.clone();
        let running = self.running.clone();
        let sched_tx = self.scheduler_tx.clone();
        let script_store = self.script_store.clone();
        let job_clone = job.clone();
        let live_output = self.live_output.clone();

        // Create broadcast channel BEFORE spawn so SSE can connect immediately
        let (tx, _) = tokio::sync::broadcast::channel::<String>(1024);
        self.live_output.insert(exec_id, tx.clone());

        tokio::spawn(async move {
            let result = run_task_streaming(
                &task,
                run_as.as_deref(),
                timeout_secs,
                Some(&script_store),
                cancel_rx,
                Some(&tx),
            )
            .await;

            // Signal completion and remove channel
            let _ = tx.send("[done]".to_string());
            live_output.remove(&exec_id);
            let finished_at = Utc::now();
            let updated = ExecutionRecord {
                id: exec_id,
                job_id: rec.job_id,
                agent_id: None,
                task_snapshot: rec.task_snapshot.clone(),
                status: result.status,
                exit_code: result.exit_code,
                stdout: result.stdout.text,
                stderr: result.stderr.text,
                stdout_truncated: result.stdout.truncated,
                stderr_truncated: result.stderr.truncated,
                started_at: rec.started_at,
                finished_at: Some(finished_at),
                triggered_by: rec.triggered_by,
                extracted: None,
                retry_of: rec.retry_of,
                attempt_number: rec.attempt_number,
                params: rec.params.clone(),
            };

            Self::handle_execution_complete(exec_id, &updated, &db, &sched_tx).await;
            running.lock().await.remove(&exec_id);

            // Schedule retry if applicable — use scheduler channel with delay
            if should_retry(job_clone.retry_max, updated.status, updated.attempt_number) {
                let next_attempt = updated.attempt_number + 1;
                let delay = calculate_retry_delay(
                    job_clone.retry_delay_secs,
                    job_clone.retry_backoff,
                    next_attempt,
                );
                let original_id = updated.retry_of.unwrap_or(exec_id);
                info!(
                    "scheduling retry {}/{} for job {} (delay: {}s)",
                    next_attempt,
                    job_clone.retry_max + 1,
                    job_clone.name,
                    delay
                );
                let sched_retry = sched_tx.clone();
                let job_id = job_clone.id;
                tokio::spawn(async move {
                    if delay > 0 {
                        tokio::time::sleep(std::time::Duration::from_secs(delay)).await;
                    }
                    if let Err(e) = sched_retry
                        .send(SchedulerCommand::RetryExecution {
                            job_id,
                            original_execution_id: original_id,
                            attempt: next_attempt,
                        })
                        .await
                    {
                        tracing::error!("failed to schedule retry for job {}: {e}", job_id);
                    }
                });
            }
        });

        Ok(exec_id)
    }
}
