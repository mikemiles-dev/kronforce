use tracing::{error, info};

use super::*;

use crate::executor::notifications::notify_execution_complete;
use crate::executor::output_rules::process_post_execution;
use crate::executor::scripts::ScriptStore;
use crate::scheduler::SchedulerCommand;

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;

use super::tasks;

impl super::Executor {
    /// Executes a job locally on the controller, spawning the task in a background tokio task.
    pub(crate) async fn execute_local(
        &self,
        job: &Job,
        trigger: TriggerSource,
    ) -> Result<Uuid, AppError> {
        let exec_id = Uuid::new_v4();
        let now = Utc::now();

        let mut rec = ExecutionRecord::new(exec_id, job.id, trigger.clone())
            .with_status(ExecutionStatus::Running)
            .with_task_snapshot(job.task.clone())
            .with_started_at(now);

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

        tokio::spawn(async move {
            let result = run_task(
                &task,
                run_as.as_deref(),
                timeout_secs,
                Some(&script_store),
                cancel_rx,
            )
            .await;
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

    /// Handles all post-execution work: persists the result, runs output rules,
    /// sends notifications, and logs the completion event.
    async fn handle_execution_complete(
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
            details: None,
            timestamp: chrono::Utc::now(),
        };
        let db3 = db.clone();
        let event2 = event.clone();
        let _ = tokio::task::spawn_blocking(move || db3.insert_event(&event2)).await;
        let _ = sched_tx.send(SchedulerCommand::EventOccurred(event)).await;
    }

    async fn run_output_rules(db: &Db, updated: &ExecutionRecord, exec_id: Uuid) -> Vec<Event> {
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

    async fn send_execution_notifications(db: &Db, updated: &ExecutionRecord, exec_id: Uuid) {
        let db_notif = db.clone();
        let job_id_notif = updated.job_id;
        let exec_status = updated.status;
        let exec_id_short = exec_id.to_string()[..8].to_string();
        let stderr_excerpt = updated.stderr.chars().take(500).collect::<String>();
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
}

/// Run a task based on its type.
pub async fn run_task(
    task: &TaskType,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    script_store: Option<&ScriptStore>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    match task {
        TaskType::Shell { command } => {
            tasks::run_shell_task(command, run_as, timeout_secs, cancel_rx).await
        }
        TaskType::Sql {
            driver,
            connection_string,
            query,
        } => {
            tasks::run_sql_task(
                driver,
                connection_string,
                query,
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
        TaskType::Ftp {
            protocol,
            host,
            port,
            username,
            password,
            direction,
            remote_path,
            local_path,
        } => {
            tasks::run_ftp_task(
                protocol,
                host,
                *port,
                username,
                password,
                direction,
                remote_path,
                local_path,
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
        TaskType::Http {
            method,
            url,
            headers,
            body,
            expect_status,
        } => {
            tasks::run_http_task(
                method,
                url,
                headers.as_ref(),
                body.as_deref(),
                *expect_status,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
        TaskType::Script { script_name } => {
            tasks::run_script_task(script_name, script_store, timeout_secs, cancel_rx).await
        }
        TaskType::Custom { .. } => CommandResult {
            status: ExecutionStatus::Failed,
            exit_code: None,
            stdout: CapturedOutput {
                text: String::new(),
                truncated: false,
            },
            stderr: CapturedOutput {
                text: "custom tasks require a custom agent — cannot run locally".to_string(),
                truncated: false,
            },
        },
        TaskType::FilePush {
            filename,
            destination,
            content_base64,
            permissions,
            overwrite,
        } => tasks::run_file_push_task(
            filename,
            destination,
            content_base64,
            permissions.as_deref(),
            *overwrite,
        ),
        TaskType::Kafka {
            broker,
            topic,
            message,
            key,
            properties,
        } => {
            tasks::run_kafka_task(
                broker,
                topic,
                message,
                key.as_deref(),
                properties.as_deref(),
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
        TaskType::Rabbitmq {
            url,
            exchange,
            routing_key,
            message,
            content_type,
        } => {
            tasks::run_rabbitmq_task(
                url,
                exchange,
                routing_key,
                message,
                content_type.as_deref(),
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
        TaskType::Mqtt {
            broker,
            topic,
            message,
            port,
            qos,
            username,
            password,
            client_id,
        } => {
            tasks::run_mqtt_task(
                broker,
                topic,
                message,
                *port,
                *qos,
                username.as_deref(),
                password.as_deref(),
                client_id.as_deref(),
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
        TaskType::Redis {
            url,
            channel,
            message,
        } => tasks::run_redis_task(url, channel, message, run_as, timeout_secs, cancel_rx).await,
        TaskType::Mcp {
            server_url,
            tool,
            arguments,
        } => {
            tasks::run_mcp_task(
                server_url,
                tool,
                arguments.as_ref(),
                timeout_secs,
                cancel_rx,
            )
            .await
        }
    }
}

pub(crate) fn shell_escape(s: &str) -> String {
    if cfg!(windows) {
        // Windows cmd.exe: wrap in double quotes, escape internal double quotes
        format!("\"{}\"", s.replace('"', "\\\""))
    } else {
        format!("'{}'", s.replace('\'', "'\\''"))
    }
}

pub(crate) fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    let hex = hex.replace(' ', "");
    if !hex.len().is_multiple_of(2) {
        return Err("odd length".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("{e}")))
        .collect()
}

pub(crate) fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Max bytes stored per stream (10MB). Output beyond this is truncated from the front (keeps tail).
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;
pub(crate) const DEFAULT_SCRIPT_TIMEOUT_SECS: u64 = 60;
pub(crate) const MAX_SCRIPT_OPERATIONS: u64 = 1_000_000;
pub(crate) const MAX_SCRIPT_STRING_SIZE: usize = 256 * 1024;

pub struct CapturedOutput {
    pub text: String,
    pub truncated: bool,
}

pub struct CommandResult {
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub stdout: CapturedOutput,
    pub stderr: CapturedOutput,
}

pub async fn run_command(
    command: &str,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let mut cmd = if let Some(user) = run_as {
        // run_as requires sudo (Unix only)
        if cfg!(windows) {
            return CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
                stderr: CapturedOutput {
                    text: "run_as is not supported on Windows".to_string(),
                    truncated: false,
                },
            };
        }
        // Validate run_as username: only allow alphanumeric, dash, underscore, dot
        if !user
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
                stderr: CapturedOutput {
                    text: format!("invalid run_as user: {user}"),
                    truncated: false,
                },
            };
        }
        let mut c = Command::new("sudo");
        c.args(["-n", "-u", user, "sh", "-c", command]);
        c
    } else if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", command]);
        c
    };
    let mut child = match cmd
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            return CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
                stderr: CapturedOutput {
                    text: format!("failed to spawn process: {e}"),
                    truncated: false,
                },
            };
        }
    };

    let timeout_duration = timeout_secs.map(Duration::from_secs);

    // Take stdout/stderr handles before waiting, so we can read them on timeout/cancel too
    let mut child_stdout = child.stdout.take();
    let mut child_stderr = child.stderr.take();

    tokio::select! {
        result = child.wait() => {
            let stdout = read_pipe_stdout(&mut child_stdout).await;
            let stderr = read_pipe_stderr(&mut child_stderr).await;
            match result {
                Ok(exit_status) => {
                    let code = exit_status.code();
                    let status = if exit_status.success() {
                        ExecutionStatus::Succeeded
                    } else {
                        ExecutionStatus::Failed
                    };
                    CommandResult {
                        status,
                        exit_code: code,
                        stdout,
                        stderr,
                    }
                }
                Err(e) => CommandResult {
                    status: ExecutionStatus::Failed,
                    exit_code: None,
                    stdout,
                    stderr: CapturedOutput { text: format!("process error: {e}"), truncated: false },
                },
            }
        }
        _ = async {
            match timeout_duration {
                Some(d) => tokio::time::sleep(d).await,
                None => std::future::pending().await,
            }
        } => {
            let _ = child.kill().await;
            CommandResult {
                status: ExecutionStatus::TimedOut,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: format!("job timed out after {}s", timeout_secs.unwrap()), truncated: false },
            }
        }
        _ = cancel_rx => {
            let _ = child.kill().await;
            CommandResult {
                status: ExecutionStatus::Cancelled,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: "job cancelled by user".to_string(), truncated: false },
            }
        }
    }
}

fn truncate_output(bytes: Vec<u8>) -> CapturedOutput {
    if bytes.len() <= MAX_OUTPUT_BYTES {
        CapturedOutput {
            text: String::from_utf8_lossy(&bytes).to_string(),
            truncated: false,
        }
    } else {
        // Keep the tail (most recent output)
        let start = bytes.len() - MAX_OUTPUT_BYTES;
        // Find the next valid char boundary to avoid splitting a UTF-8 sequence
        let trimmed = &bytes[start..];
        let text = String::from_utf8_lossy(trimmed).to_string();
        CapturedOutput {
            text: format!("[...truncated {} bytes...]\n{}", start, text),
            truncated: true,
        }
    }
}

async fn read_pipe_stdout(pipe: &mut Option<tokio::process::ChildStdout>) -> CapturedOutput {
    match pipe {
        Some(p) => {
            let mut buf = Vec::new();
            let _ = p.read_to_end(&mut buf).await;
            truncate_output(buf)
        }
        None => CapturedOutput {
            text: String::new(),
            truncated: false,
        },
    }
}

async fn read_pipe_stderr(pipe: &mut Option<tokio::process::ChildStderr>) -> CapturedOutput {
    match pipe {
        Some(p) => {
            let mut buf = Vec::new();
            let _ = p.read_to_end(&mut buf).await;
            truncate_output(buf)
        }
        None => CapturedOutput {
            text: String::new(),
            truncated: false,
        },
    }
}

/// Maximum retry delay cap (1 hour).
const MAX_RETRY_DELAY_SECS: u64 = 3600;

/// Calculates the retry delay for the given attempt, capped at MAX_RETRY_DELAY_SECS.
pub(crate) fn calculate_retry_delay(delay_secs: u64, backoff: f64, attempt: u32) -> u64 {
    let delay = (delay_secs as f64) * backoff.powi((attempt - 1) as i32);
    // Clamp before cast to prevent f64 overflow → u64 saturation
    if delay.is_nan() || delay.is_infinite() || delay > MAX_RETRY_DELAY_SECS as f64 {
        MAX_RETRY_DELAY_SECS
    } else {
        (delay as u64).min(MAX_RETRY_DELAY_SECS)
    }
}

/// Returns true if the execution should be retried based on job config and status.
pub(crate) fn should_retry(retry_max: u32, status: ExecutionStatus, attempt_number: u32) -> bool {
    if retry_max == 0 {
        return false;
    }
    if attempt_number > retry_max {
        return false;
    }
    matches!(status, ExecutionStatus::Failed | ExecutionStatus::TimedOut)
}
