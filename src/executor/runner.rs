//! Task execution and command running: `run_task`, `run_command`, streaming variants,
//! and output capture/truncation helpers.

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio::sync::oneshot;

use crate::db::models::*;
use crate::executor::scripts::ScriptStore;
use crate::executor::tasks;
use crate::executor::utils::shell_escape;

/// Max bytes stored per stream (10MB). Output beyond this is truncated from the front (keeps tail).
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024;

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

/// Run a task based on its type.
pub async fn run_task(
    task: &TaskType,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    script_store: Option<&ScriptStore>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    match task {
        TaskType::Shell {
            command,
            working_dir,
        } => {
            let cmd = match working_dir {
                Some(dir) => format!("cd {} && {}", shell_escape(dir), command),
                None => command.clone(),
            };
            tasks::run_shell_task(&cmd, run_as, timeout_secs, cancel_rx).await
        }
        TaskType::Sql {
            driver,
            connection_string,
            query,
            connection: _,
        } => {
            let conn_str = connection_string.as_deref().unwrap_or("");
            tasks::run_sql_task(driver, conn_str, query, run_as, timeout_secs, cancel_rx).await
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
            connection: _,
        } => {
            tasks::run_ftp_task(
                protocol,
                host.as_deref().unwrap_or(""),
                *port,
                username.as_deref().unwrap_or(""),
                password.as_deref().unwrap_or(""),
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
            connection: _,
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
        TaskType::DockerBuild {
            script_name,
            image_tag,
            run_after_build,
            build_args,
        } => {
            tasks::run_docker_build_task(
                script_name,
                image_tag.as_deref(),
                *run_after_build,
                build_args.as_deref(),
                script_store,
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
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
            connection: _,
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
            connection: _,
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
            connection: _,
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
            connection: _,
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
        TaskType::KafkaConsume {
            broker,
            topic,
            group_id,
            max_messages,
            offset,
            connection: _,
        } => {
            tasks::run_kafka_consume_task(
                broker,
                topic,
                group_id.as_deref(),
                *max_messages,
                offset.as_deref(),
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
        TaskType::MqttSubscribe {
            broker,
            topic,
            port,
            max_messages,
            username,
            password,
            client_id,
            qos,
            connection: _,
        } => {
            tasks::run_mqtt_subscribe_task(
                broker,
                topic,
                *port,
                *max_messages,
                username.as_deref(),
                password.as_deref(),
                client_id.as_deref(),
                *qos,
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
        TaskType::RabbitmqConsume {
            url,
            queue,
            max_messages,
            connection: _,
        } => {
            tasks::run_rabbitmq_consume_task(
                url,
                queue,
                *max_messages,
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
        TaskType::RedisRead {
            url,
            key,
            mode,
            count,
            connection: _,
        } => {
            tasks::run_redis_read_task(
                url,
                key,
                mode.as_deref(),
                *count,
                run_as,
                timeout_secs,
                cancel_rx,
            )
            .await
        }
    }
}

/// Like `run_task` but broadcasts stdout/stderr lines via `live_tx` for SSE streaming.
/// Only shell tasks get true line-by-line streaming; other task types broadcast output at the end.
pub async fn run_task_streaming(
    task: &TaskType,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    script_store: Option<&ScriptStore>,
    cancel_rx: oneshot::Receiver<()>,
    live_tx: Option<&tokio::sync::broadcast::Sender<String>>,
) -> CommandResult {
    if let Some(tx) = live_tx {
        // For shell-based tasks, stream output line-by-line
        let shell_cmd = match task {
            TaskType::Shell {
                command,
                working_dir,
            } => Some(match working_dir {
                Some(dir) => format!("cd {} && {}", shell_escape(dir), command),
                None => command.clone(),
            }),
            TaskType::DockerBuild {
                script_name,
                image_tag,
                run_after_build,
                build_args,
            } => {
                // Write Dockerfile to temp dir using Rust, then just run docker build
                if let Some(store) = script_store {
                    if let Ok(code) = store.read_code(script_name) {
                        let tag = image_tag.as_deref().unwrap_or(script_name);
                        let tmp = std::env::temp_dir()
                            .join(format!("kf-docker-{}", uuid::Uuid::new_v4()));
                        if std::fs::create_dir_all(&tmp).is_ok()
                            && std::fs::write(tmp.join("Dockerfile"), &code).is_ok()
                        {
                            let tmp_path = tmp.display().to_string();
                            let mut cmd = format!(
                                "docker build --progress=plain -t {} {} -f {}/Dockerfile {}",
                                shell_escape(tag),
                                build_args.as_deref().unwrap_or(""),
                                shell_escape(&tmp_path),
                                shell_escape(&tmp_path),
                            );
                            if *run_after_build {
                                cmd.push_str(&format!(" && docker run --rm {}", shell_escape(tag)));
                            }
                            cmd.push_str(&format!(
                                " ; RET=$?; rm -rf {}; exit $RET",
                                shell_escape(&tmp_path)
                            ));
                            Some(cmd)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(cmd) = shell_cmd {
            return run_command_streaming(&cmd, run_as, timeout_secs, cancel_rx, tx).await;
        }
    }
    // For non-streamable tasks or when no live_tx, use standard run_task
    // and broadcast the final output
    let result = run_task(task, run_as, timeout_secs, script_store, cancel_rx).await;
    if let Some(tx) = live_tx {
        for line in result.stdout.text.lines() {
            let _ = tx.send(line.to_string());
        }
        for line in result.stderr.text.lines() {
            let _ = tx.send(format!("[stderr] {}", line));
        }
    }
    result
}

pub async fn run_command(
    command: &str,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    run_command_inner(command, run_as, timeout_secs, cancel_rx, None).await
}

pub async fn run_command_streaming(
    command: &str,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
    live_tx: &tokio::sync::broadcast::Sender<String>,
) -> CommandResult {
    run_command_inner(command, run_as, timeout_secs, cancel_rx, Some(live_tx)).await
}

async fn run_command_inner(
    command: &str,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    mut cancel_rx: oneshot::Receiver<()>,
    live_tx: Option<&tokio::sync::broadcast::Sender<String>>,
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
    let child_stdout = child.stdout.take();
    let child_stderr = child.stderr.take();

    // If streaming, read lines and broadcast; otherwise read all at once
    if let Some(tx) = live_tx {
        use tokio::io::{AsyncBufReadExt, BufReader};
        let mut stdout_lines = child_stdout.map(|s| BufReader::new(s).lines());
        let mut stderr_lines = child_stderr.map(|s| BufReader::new(s).lines());
        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();
        let mut stdout_done = stdout_lines.is_none();
        let mut stderr_done = stderr_lines.is_none();

        loop {
            // Break when both pipes are closed (process exited)
            if stdout_done && stderr_done {
                break;
            }

            tokio::select! {
                line = async {
                    match stdout_lines.as_mut() {
                        Some(r) => r.next_line().await,
                        None => std::future::pending().await,
                    }
                }, if !stdout_done => {
                    match line {
                        Ok(Some(l)) => { let _ = tx.send(l.clone()); stdout_buf.push(l); }
                        _ => { stdout_done = true; }
                    }
                }
                line = async {
                    match stderr_lines.as_mut() {
                        Some(r) => r.next_line().await,
                        None => std::future::pending().await,
                    }
                }, if !stderr_done => {
                    match line {
                        Ok(Some(l)) => { let _ = tx.send(format!("[stderr] {}", l)); stderr_buf.push(l); }
                        _ => { stderr_done = true; }
                    }
                }
                _ = async {
                    match timeout_duration {
                        Some(d) => tokio::time::sleep(d).await,
                        None => std::future::pending().await,
                    }
                } => {
                    let _ = child.kill().await;
                    return CommandResult {
                        status: ExecutionStatus::TimedOut,
                        exit_code: None,
                        stdout: truncate_output(stdout_buf.join("\n").into_bytes()),
                        stderr: CapturedOutput { text: format!("job timed out after {}s", timeout_secs.unwrap_or(0)), truncated: false },
                    };
                }
                _ = &mut cancel_rx => {
                    let _ = child.kill().await;
                    return CommandResult {
                        status: ExecutionStatus::Cancelled,
                        exit_code: None,
                        stdout: truncate_output(stdout_buf.join("\n").into_bytes()),
                        stderr: CapturedOutput { text: "job cancelled by user".to_string(), truncated: false },
                    };
                }
            }
        }

        let exit_status = child.wait().await;
        match exit_status {
            Ok(es) => {
                let status = if es.success() {
                    ExecutionStatus::Succeeded
                } else {
                    ExecutionStatus::Failed
                };
                CommandResult {
                    status,
                    exit_code: es.code(),
                    stdout: truncate_output(stdout_buf.join("\n").into_bytes()),
                    stderr: truncate_output(stderr_buf.join("\n").into_bytes()),
                }
            }
            Err(e) => CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: truncate_output(stdout_buf.join("\n").into_bytes()),
                stderr: CapturedOutput {
                    text: format!("process error: {e}"),
                    truncated: false,
                },
            },
        }
    } else {
        // Non-streaming path (original behavior)
        let mut child_stdout = child_stdout;
        let mut child_stderr = child_stderr;

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
                    stderr: CapturedOutput { text: format!("job timed out after {}s", timeout_secs.unwrap_or(0)), truncated: false },
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
    } // end else (non-streaming path)
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
