use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

use crate::agent_client::AgentClient;
use crate::db::Db;
use crate::error::AppError;
use crate::models::*;
use crate::protocol::JobDispatchRequest;

use reqwest;

struct RunningJob {
    cancel_tx: oneshot::Sender<()>,
}

#[derive(Clone)]
pub struct Executor {
    db: Db,
    agent_client: AgentClient,
    scheduler_tx: tokio::sync::mpsc::Sender<crate::scheduler::SchedulerCommand>,
    running: Arc<Mutex<HashMap<Uuid, RunningJob>>>,
}

impl Executor {
    pub fn new(db: Db, agent_client: AgentClient, scheduler_tx: tokio::sync::mpsc::Sender<crate::scheduler::SchedulerCommand>) -> Self {
        Self {
            db,
            agent_client,
            scheduler_tx,
            running: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn execute(
        &self,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        match &job.target {
            None | Some(AgentTarget::Local) => {
                self.execute_local(job, trigger).await
            }
            Some(AgentTarget::Agent { agent_id }) => {
                self.dispatch_to_agent(*agent_id, job, trigger, callback_base_url).await
            }
            Some(AgentTarget::Tagged { tag }) => {
                self.dispatch_to_tagged(tag, job, trigger, callback_base_url).await
            }
            Some(AgentTarget::Any) => {
                self.dispatch_to_any(job, trigger, callback_base_url).await
            }
            Some(AgentTarget::All) => {
                self.dispatch_to_all(job, trigger, callback_base_url).await
            }
        }
    }

    async fn execute_local(
        &self,
        job: &Job,
        trigger: TriggerSource,
    ) -> Result<Uuid, AppError> {
        let exec_id = Uuid::new_v4();
        let now = Utc::now();

        let rec = ExecutionRecord {
            id: exec_id,
            job_id: job.id,
            agent_id: None,
            task_snapshot: Some(job.task.clone()),
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

        let db = self.db.clone();
        let rec_clone = rec.clone();
        tokio::task::spawn_blocking(move || db.insert_execution(&rec_clone)).await.unwrap()?;

        let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
        {
            let mut running = self.running.lock().await;
            running.insert(exec_id, RunningJob { cancel_tx });
        }

        let task = job.task.clone();
        let run_as = job.run_as.clone();
        let timeout_secs = job.timeout_secs;
        let db = self.db.clone();
        let running = self.running.clone();
        let sched_tx = self.scheduler_tx.clone();

        tokio::spawn(async move {
            let result = run_task(&task, run_as.as_deref(), timeout_secs, cancel_rx).await;
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
            };

            let db2 = db.clone();
            let updated2 = updated.clone();
            if let Err(e) =
                tokio::task::spawn_blocking(move || db2.update_execution(&updated2)).await
            {
                tracing::error!("failed to update execution {}: {e}", exec_id);
            }
            running.lock().await.remove(&exec_id);
            tracing::info!("execution {} finished: {:?}", exec_id, updated.status);

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
                message: format!("Execution {} finished: {:?}", exec_id, updated.status),
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
            let _ = sched_tx.send(crate::scheduler::SchedulerCommand::EventOccurred(event)).await;
        });

        Ok(exec_id)
    }

    async fn dispatch_to_agent(
        &self,
        agent_id: Uuid,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let db = self.db.clone();
        let agent = tokio::task::spawn_blocking(move || db.get_agent(agent_id))
            .await
            .unwrap()?
            .ok_or_else(|| AppError::AgentUnavailable(format!("agent {agent_id} not found")))?;

        if agent.status != AgentStatus::Online {
            return Err(AppError::AgentUnavailable(format!(
                "agent {} is {}",
                agent.name,
                agent.status.as_str()
            )));
        }

        self.dispatch_to_specific_agent(&agent, job, trigger, callback_base_url).await
    }

    async fn dispatch_to_tagged(
        &self,
        tag: &str,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let db = self.db.clone();
        let tag_owned = tag.to_string();
        let tag_for_err = tag_owned.clone();
        let agents = tokio::task::spawn_blocking(move || db.get_online_agents_by_tag(&tag_owned))
            .await
            .unwrap()?;

        if agents.is_empty() {
            return Err(AppError::AgentUnavailable(format!(
                "no online agents with tag '{}'",
                tag_for_err
            )));
        }

        // Pick random agent
        let idx = (Utc::now().timestamp_nanos_opt().unwrap_or(0) as usize) % agents.len();
        let agent = &agents[idx];

        self.dispatch_to_specific_agent(agent, job, trigger, callback_base_url).await
    }

    async fn dispatch_to_any(
        &self,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let db = self.db.clone();
        let agents = tokio::task::spawn_blocking(move || db.get_online_agents())
            .await
            .unwrap()?;

        if agents.is_empty() {
            return Err(AppError::AgentUnavailable(
                "no online agents available".to_string(),
            ));
        }

        let idx = (Utc::now().timestamp_nanos_opt().unwrap_or(0) as usize) % agents.len();
        let agent = &agents[idx];

        self.dispatch_to_specific_agent(agent, job, trigger, callback_base_url).await
    }

    async fn dispatch_to_all(
        &self,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let db = self.db.clone();
        let agents = tokio::task::spawn_blocking(move || db.get_online_agents())
            .await
            .unwrap()?;

        if agents.is_empty() {
            return Err(AppError::AgentUnavailable(
                "no online agents available".to_string(),
            ));
        }

        let mut first_exec_id = None;
        for agent in &agents {
            match self
                .dispatch_to_specific_agent(agent, job, trigger.clone(), callback_base_url)
                .await
            {
                Ok(exec_id) => {
                    if first_exec_id.is_none() {
                        first_exec_id = Some(exec_id);
                    }
                }
                Err(e) => {
                    tracing::error!(
                        "failed to dispatch to agent {} ({}): {e}",
                        agent.name,
                        agent.id
                    );
                }
            }
        }

        first_exec_id.ok_or_else(|| {
            AppError::AgentError("failed to dispatch to any agent".to_string())
        })
    }

    async fn dispatch_to_specific_agent(
        &self,
        agent: &Agent,
        job: &Job,
        trigger: TriggerSource,
        callback_base_url: &str,
    ) -> Result<Uuid, AppError> {
        let exec_id = Uuid::new_v4();
        let now = Utc::now();

        let rec = ExecutionRecord {
            id: exec_id,
            job_id: job.id,
            agent_id: Some(agent.id),
            task_snapshot: Some(job.task.clone()),
            status: ExecutionStatus::Pending,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            started_at: Some(now),
            finished_at: None,
            triggered_by: trigger,
        };

        let db = self.db.clone();
        let rec_clone = rec.clone();
        tokio::task::spawn_blocking(move || db.insert_execution(&rec_clone)).await.unwrap()?;

        let dispatch = JobDispatchRequest {
            execution_id: exec_id,
            job_id: job.id,
            task: job.task.clone(),
            run_as: job.run_as.clone(),
            timeout_secs: job.timeout_secs,
            callback_url: format!("{}/api/callbacks/execution-result", callback_base_url),
        };

        match self.agent_client.dispatch_job(&agent.address, agent.port, &dispatch).await {
            Ok(resp) if resp.accepted => {
                // Update to Running
                let db = self.db.clone();
                let mut running_rec = rec;
                running_rec.status = ExecutionStatus::Running;
                let _ = tokio::task::spawn_blocking(move || db.update_execution(&running_rec)).await;
                tracing::info!(
                    "dispatched job {} to agent {} -> execution {}",
                    job.name, agent.name, exec_id
                );
                Ok(exec_id)
            }
            Ok(resp) => {
                let msg = resp.message.unwrap_or_else(|| "rejected".into());
                // Mark as failed
                let db = self.db.clone();
                let mut failed_rec = rec;
                failed_rec.status = ExecutionStatus::Failed;
                failed_rec.stderr = format!("agent rejected: {msg}");
                failed_rec.finished_at = Some(Utc::now());
                let _ = tokio::task::spawn_blocking(move || db.update_execution(&failed_rec)).await;
                Err(AppError::AgentError(msg))
            }
            Err(e) => {
                let db = self.db.clone();
                let mut failed_rec = rec;
                failed_rec.status = ExecutionStatus::Failed;
                failed_rec.stderr = format!("dispatch failed: {e}");
                failed_rec.finished_at = Some(Utc::now());
                let _ = tokio::task::spawn_blocking(move || db.update_execution(&failed_rec)).await;
                Err(e)
            }
        }
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
}

/// Run a task based on its type.
pub async fn run_task(
    task: &TaskType,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    match task {
        TaskType::Shell { command } => {
            run_command(command, run_as, timeout_secs, cancel_rx).await
        }
        TaskType::Sql { driver, connection_string, query } => {
            let cmd = match driver {
                SqlDriver::Postgres => format!("psql {} -c {}", shell_escape(connection_string), shell_escape(query)),
                SqlDriver::Mysql => format!("mysql {} -e {}", shell_escape(connection_string), shell_escape(query)),
                SqlDriver::Sqlite => format!("sqlite3 {} {}", shell_escape(connection_string), shell_escape(query)),
            };
            run_command(&cmd, run_as, timeout_secs, cancel_rx).await
        }
        TaskType::Ftp { protocol, host, port, username, password, direction, remote_path, local_path } => {
            let port_part = port.map(|p| format!(":{}", p)).unwrap_or_default();
            let proto = match protocol {
                FtpProtocol::Ftp => "ftp",
                FtpProtocol::Ftps => "ftps",
                FtpProtocol::Sftp => "sftp",
            };
            let url = format!("{}://{}{}{}",proto, host, port_part, remote_path);
            let cmd = match direction {
                TransferDirection::Download => format!("curl -u {}:{} {} -o {}", shell_escape(username), shell_escape(password), shell_escape(&url), shell_escape(local_path)),
                TransferDirection::Upload => format!("curl -u {}:{} -T {} {}", shell_escape(username), shell_escape(password), shell_escape(local_path), shell_escape(&url)),
            };
            run_command(&cmd, run_as, timeout_secs, cancel_rx).await
        }
        TaskType::Http { method, url, headers, body, expect_status } => {
            run_http(method, url, headers.as_ref(), body.as_deref(), *expect_status, timeout_secs, cancel_rx).await
        }
    }
}

fn shell_escape(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

async fn run_http(
    method: &HttpMethod,
    url: &str,
    headers: Option<&std::collections::HashMap<String, String>>,
    body: Option<&str>,
    expect_status: Option<u16>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let client = reqwest::Client::builder()
        .timeout(timeout_secs.map(std::time::Duration::from_secs).unwrap_or(std::time::Duration::from_secs(30)))
        .build()
        .unwrap();

    let mut req = match method {
        HttpMethod::Get => client.get(url),
        HttpMethod::Post => client.post(url),
        HttpMethod::Put => client.put(url),
        HttpMethod::Delete => client.delete(url),
    };

    if let Some(hdrs) = headers {
        for (k, v) in hdrs {
            req = req.header(k.as_str(), v.as_str());
        }
    }

    if let Some(b) = body {
        req = req.body(b.to_string());
    }

    let http_future = async {
        match req.send().await {
            Ok(resp) => {
                let status_code = resp.status().as_u16();
                let resp_body = resp.text().await.unwrap_or_default();

                if let Some(expected) = expect_status {
                    if status_code != expected {
                        return CommandResult {
                            status: ExecutionStatus::Failed,
                            exit_code: Some(status_code as i32),
                            stdout: CapturedOutput { text: resp_body, truncated: false },
                            stderr: CapturedOutput { text: format!("expected status {}, got {}", expected, status_code), truncated: false },
                        };
                    }
                }

                CommandResult {
                    status: if (200..300).contains(&status_code) { ExecutionStatus::Succeeded } else { ExecutionStatus::Failed },
                    exit_code: Some(status_code as i32),
                    stdout: CapturedOutput { text: resp_body, truncated: false },
                    stderr: CapturedOutput { text: String::new(), truncated: false },
                }
            }
            Err(e) => CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: format!("HTTP request failed: {e}"), truncated: false },
            },
        }
    };

    tokio::select! {
        result = http_future => result,
        _ = cancel_rx => {
            CommandResult {
                status: ExecutionStatus::Cancelled,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: "job cancelled by user".to_string(), truncated: false },
            }
        }
    }
}

/// Max bytes stored per stream (256KB). Output beyond this is truncated from the front (keeps tail).
const MAX_OUTPUT_BYTES: usize = 256 * 1024;

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
        let mut c = Command::new("sudo");
        c.args(["-n", "-u", user, "sh", "-c", command]);
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
