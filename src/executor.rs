use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

use crate::db::Db;
use crate::models::*;

struct RunningJob {
    cancel_tx: oneshot::Sender<()>,
}

#[derive(Clone)]
pub struct Executor {
    db: Db,
    running: Arc<Mutex<HashMap<Uuid, RunningJob>>>,
}

impl Executor {
    pub fn new(db: Db) -> Self {
        Self {
            db,
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Execute a job. Returns the execution ID immediately; the job runs in the background.
    pub async fn execute(
        &self,
        job: &Job,
        trigger: TriggerSource,
    ) -> Result<Uuid, crate::error::AppError> {
        let exec_id = Uuid::new_v4();
        let now = Utc::now();

        let rec = ExecutionRecord {
            id: exec_id,
            job_id: job.id,
            status: ExecutionStatus::Running,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            started_at: Some(now),
            finished_at: None,
            triggered_by: trigger,
        };

        // Insert into DB on blocking thread
        let db = self.db.clone();
        let rec_clone = rec.clone();
        tokio::task::spawn_blocking(move || db.insert_execution(&rec_clone)).await.unwrap()?;

        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();

        {
            let mut running = self.running.lock().await;
            running.insert(exec_id, RunningJob { cancel_tx });
        }

        let command = job.command.clone();
        let timeout_secs = job.timeout_secs;
        let db = self.db.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            let result = run_command(&command, timeout_secs, cancel_rx).await;

            let finished_at = Utc::now();
            let updated = ExecutionRecord {
                id: exec_id,
                job_id: rec.job_id,
                status: result.status,
                exit_code: result.exit_code,
                stdout: result.stdout.text,
                stderr: result.stderr.text,
                stdout_truncated: result.stdout.truncated,
                stderr_truncated: result.stderr.truncated,
                started_at: rec.started_at,
                finished_at: Some(finished_at),
                triggered_by: rec.triggered_by,
            };

            let db2 = db.clone();
            let updated2 = updated.clone();
            if let Err(e) =
                tokio::task::spawn_blocking(move || db2.update_execution(&updated2)).await
            {
                tracing::error!("failed to update execution {}: {e}", exec_id);
            }

            running.lock().await.remove(&exec_id);

            tracing::info!(
                "execution {} finished: {:?} (exit_code: {:?})",
                exec_id,
                updated.status,
                updated.exit_code
            );
        });

        Ok(exec_id)
    }

    pub async fn cancel(&self, execution_id: Uuid) -> bool {
        let mut running = self.running.lock().await;
        if let Some(job) = running.remove(&execution_id) {
            let _ = job.cancel_tx.send(());
            true
        } else {
            false
        }
    }

    pub async fn is_running(&self, execution_id: Uuid) -> bool {
        self.running.lock().await.contains_key(&execution_id)
    }
}

/// Max bytes stored per stream (256KB). Output beyond this is truncated from the front (keeps tail).
const MAX_OUTPUT_BYTES: usize = 256 * 1024;

struct CapturedOutput {
    text: String,
    truncated: bool,
}

struct CommandResult {
    status: ExecutionStatus,
    exit_code: Option<i32>,
    stdout: CapturedOutput,
    stderr: CapturedOutput,
}

async fn run_command(
    command: &str,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let mut child = match Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(e) => {
            return CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: format!("failed to spawn process: {e}"), truncated: false },
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
        None => CapturedOutput { text: String::new(), truncated: false },
    }
}

async fn read_pipe_stderr(pipe: &mut Option<tokio::process::ChildStderr>) -> CapturedOutput {
    match pipe {
        Some(p) => {
            let mut buf = Vec::new();
            let _ = p.read_to_end(&mut buf).await;
            truncate_output(buf)
        }
        None => CapturedOutput { text: String::new(), truncated: false },
    }
}
