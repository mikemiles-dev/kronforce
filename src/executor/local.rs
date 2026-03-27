use super::*;

use std::time::Duration;

use tokio::io::AsyncReadExt;
use tokio::process::Command;

use reqwest;

impl super::Executor {
    pub(crate) async fn execute_local(
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
            extracted: None,
        };

        let db = self.db.clone();
        let rec_clone = rec.clone();
        tokio::task::spawn_blocking(move || db.insert_execution(&rec_clone))
            .await
            .unwrap()?;

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
            };

            let db2 = db.clone();
            let updated2 = updated.clone();
            if let Err(e) =
                tokio::task::spawn_blocking(move || db2.update_execution(&updated2)).await
            {
                tracing::error!("failed to update execution {}: {e}", exec_id);
            }
            // Run output rules (extraction + triggers)
            {
                let db_rules = db.clone();
                let stdout_clone = updated.stdout.clone();
                let stderr_clone = updated.stderr.clone();
                let job_id = updated.job_id;
                let exec_id_rules = exec_id;
                let exec_status = updated.status;
                let output_events: Vec<Event> = tokio::task::spawn_blocking(move || {
                    let mut events = Vec::new();
                    if let Ok(Some(job)) = db_rules.get_job(job_id)
                        && let Some(ref rules) = job.output_rules
                    {
                        // Extractions
                        if !rules.extractions.is_empty() {
                            let extracted = crate::output_rules::run_extractions(
                                &stdout_clone,
                                &rules.extractions,
                            );
                            if !extracted.is_empty() {
                                let _ = db_rules.update_execution_extracted(
                                    exec_id_rules,
                                    &serde_json::json!(extracted),
                                );
                                // Write-back: update global variables for rules with write_to_variable
                                for rule in &rules.extractions {
                                    if let Some(ref var_name) = rule.write_to_variable {
                                        if let Some(value) = extracted.get(&rule.name) {
                                            if let Err(e) =
                                                db_rules.upsert_variable(var_name, value)
                                            {
                                                tracing::error!(
                                                    "failed to write variable {}: {}",
                                                    var_name,
                                                    e
                                                );
                                            } else {
                                                tracing::info!(
                                                    "variable {} updated from extraction",
                                                    var_name
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        // Assertions — only on successful executions
                        if exec_status == ExecutionStatus::Succeeded && !rules.assertions.is_empty()
                        {
                            let failures = crate::output_rules::run_assertions(
                                &stdout_clone,
                                &rules.assertions,
                            );
                            if !failures.is_empty() {
                                let msg = failures.join("; ");
                                let _ = db_rules.fail_execution_assertion(exec_id_rules, &msg);
                                tracing::warn!(
                                    "execution {} failed assertion: {}",
                                    exec_id_rules,
                                    msg
                                );
                            }
                        }
                        // Triggers
                        let matches = crate::output_rules::run_triggers(
                            &stdout_clone,
                            &stderr_clone,
                            &rules.triggers,
                        );
                        for (pattern, severity) in &matches {
                            let sev = match severity.as_str() {
                                "error" => crate::models::EventSeverity::Error,
                                "warning" => crate::models::EventSeverity::Warning,
                                "success" => crate::models::EventSeverity::Success,
                                _ => crate::models::EventSeverity::Info,
                            };
                            let event = Event {
                                id: Uuid::new_v4(),
                                kind: "output.matched".to_string(),
                                severity: sev,
                                message: format!(
                                    "Output pattern matched: '{}' in job '{}'",
                                    pattern, job.name
                                ),
                                job_id: Some(job_id),
                                agent_id: None,
                                api_key_id: None,
                                api_key_name: None,
                                details: None,
                                timestamp: chrono::Utc::now(),
                            };
                            let _ = db_rules.insert_event(&event);
                            events.push(event);
                        }
                    }
                    events
                })
                .await
                .unwrap_or_default();
                // Notify scheduler of output.matched events so event-triggered jobs can fire
                for event in output_events {
                    let _ = sched_tx
                        .send(crate::scheduler::SchedulerCommand::EventOccurred(event))
                        .await;
                }
            }

            // Send notifications based on job config
            {
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
                    let recipients = notif.recipients.as_ref().map(|r| {
                        crate::notifications::NotificationRecipients {
                            emails: r.emails.clone(),
                            phones: r.phones.clone(),
                        }
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
            let _ = sched_tx
                .send(crate::scheduler::SchedulerCommand::EventOccurred(event))
                .await;
        });

        Ok(exec_id)
    }
}

/// Run a task based on its type.
pub async fn run_task(
    task: &TaskType,
    run_as: Option<&str>,
    timeout_secs: Option<u64>,
    script_store: Option<&crate::scripts::ScriptStore>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    match task {
        TaskType::Shell { command } => run_command(command, run_as, timeout_secs, cancel_rx).await,
        TaskType::Sql {
            driver,
            connection_string,
            query,
        } => {
            let cmd = match driver {
                SqlDriver::Postgres => format!(
                    "psql {} -c {}",
                    shell_escape(connection_string),
                    shell_escape(query)
                ),
                SqlDriver::Mysql => format!(
                    "mysql {} -e {}",
                    shell_escape(connection_string),
                    shell_escape(query)
                ),
                SqlDriver::Sqlite => format!(
                    "sqlite3 {} {}",
                    shell_escape(connection_string),
                    shell_escape(query)
                ),
            };
            run_command(&cmd, run_as, timeout_secs, cancel_rx).await
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
            let port_part = port.map(|p| format!(":{}", p)).unwrap_or_default();
            let proto = match protocol {
                FtpProtocol::Ftp => "ftp",
                FtpProtocol::Ftps => "ftps",
                FtpProtocol::Sftp => "sftp",
            };
            let url = format!("{}://{}{}{}", proto, host, port_part, remote_path);
            let cmd = match direction {
                TransferDirection::Download => format!(
                    "curl -u {}:{} {} -o {}",
                    shell_escape(username),
                    shell_escape(password),
                    shell_escape(&url),
                    shell_escape(local_path)
                ),
                TransferDirection::Upload => format!(
                    "curl -u {}:{} -T {} {}",
                    shell_escape(username),
                    shell_escape(password),
                    shell_escape(local_path),
                    shell_escape(&url)
                ),
            };
            run_command(&cmd, run_as, timeout_secs, cancel_rx).await
        }
        TaskType::Http {
            method,
            url,
            headers,
            body,
            expect_status,
        } => {
            run_http(
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
            let store = match script_store {
                Some(s) => s,
                None => {
                    return CommandResult {
                        status: ExecutionStatus::Failed,
                        exit_code: None,
                        stdout: CapturedOutput {
                            text: String::new(),
                            truncated: false,
                        },
                        stderr: CapturedOutput {
                            text: "script store not available on agent".to_string(),
                            truncated: false,
                        },
                    };
                }
            };
            let code = match store.read_code(script_name) {
                Ok(c) => c,
                Err(e) => {
                    return CommandResult {
                        status: ExecutionStatus::Failed,
                        exit_code: None,
                        stdout: CapturedOutput {
                            text: String::new(),
                            truncated: false,
                        },
                        stderr: CapturedOutput {
                            text: format!("script error: {e}"),
                            truncated: false,
                        },
                    };
                }
            };
            run_script(&code, timeout_secs, cancel_rx).await
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
        } => {
            use base64::Engine;
            let decoded = match base64::engine::general_purpose::STANDARD.decode(content_base64) {
                Ok(bytes) => bytes,
                Err(e) => {
                    return CommandResult {
                        status: ExecutionStatus::Failed,
                        exit_code: Some(1),
                        stdout: CapturedOutput {
                            text: String::new(),
                            truncated: false,
                        },
                        stderr: CapturedOutput {
                            text: format!("base64 decode error: {e}"),
                            truncated: false,
                        },
                    };
                }
            };

            let dest = std::path::Path::new(destination);

            // Check overwrite
            if !overwrite && dest.exists() {
                return CommandResult {
                    status: ExecutionStatus::Failed,
                    exit_code: Some(1),
                    stdout: CapturedOutput {
                        text: String::new(),
                        truncated: false,
                    },
                    stderr: CapturedOutput {
                        text: format!("file already exists: {} (overwrite=false)", destination),
                        truncated: false,
                    },
                };
            }

            // Create parent dirs
            if let Some(parent) = dest.parent()
                && let Err(e) = std::fs::create_dir_all(parent)
            {
                return CommandResult {
                    status: ExecutionStatus::Failed,
                    exit_code: Some(1),
                    stdout: CapturedOutput {
                        text: String::new(),
                        truncated: false,
                    },
                    stderr: CapturedOutput {
                        text: format!("failed to create directory {}: {e}", parent.display()),
                        truncated: false,
                    },
                };
            }

            // Write file
            let size = decoded.len();
            if let Err(e) = std::fs::write(dest, &decoded) {
                return CommandResult {
                    status: ExecutionStatus::Failed,
                    exit_code: Some(1),
                    stdout: CapturedOutput {
                        text: String::new(),
                        truncated: false,
                    },
                    stderr: CapturedOutput {
                        text: format!("failed to write file: {e}"),
                        truncated: false,
                    },
                };
            }

            // Set permissions (Unix only)
            #[cfg(unix)]
            if let Some(perm_str) = permissions
                && let Ok(mode) = u32::from_str_radix(perm_str, 8)
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = std::fs::Permissions::from_mode(mode);
                let _ = std::fs::set_permissions(dest, perms);
            }

            CommandResult {
                status: ExecutionStatus::Succeeded,
                exit_code: Some(0),
                stdout: CapturedOutput {
                    text: format!(
                        "File '{}' written to {} ({} bytes)",
                        filename, destination, size
                    ),
                    truncated: false,
                },
                stderr: CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
            }
        }
        TaskType::Kafka {
            broker,
            topic,
            message,
            key,
            properties,
        } => {
            let mut cmd = format!(
                "echo {} | kafka-console-producer --broker-list {} --topic {}",
                shell_escape(message),
                shell_escape(broker),
                shell_escape(topic)
            );
            if let Some(k) = key {
                cmd = format!(
                    "echo {}:{} | kafka-console-producer --broker-list {} --topic {} --property parse.key=true --property key.separator=:",
                    shell_escape(k),
                    shell_escape(message),
                    shell_escape(broker),
                    shell_escape(topic)
                );
            }
            if let Some(props) = properties {
                cmd.push(' ');
                cmd.push_str(props);
            }
            run_command(&cmd, run_as, timeout_secs, cancel_rx).await
        }
        TaskType::Rabbitmq {
            url,
            exchange,
            routing_key,
            message,
            content_type,
        } => {
            let mut cmd = format!(
                "amqp-publish --url {} --exchange {} --routing-key {} --body {}",
                shell_escape(url),
                shell_escape(exchange),
                shell_escape(routing_key),
                shell_escape(message)
            );
            if let Some(ct) = content_type {
                cmd.push_str(&format!(" --content-type {}", shell_escape(ct)));
            }
            run_command(&cmd, run_as, timeout_secs, cancel_rx).await
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
            let p = port.unwrap_or(1883);
            let mut cmd = format!(
                "mosquitto_pub -h {} -p {} -t {} -m {}",
                shell_escape(broker),
                p,
                shell_escape(topic),
                shell_escape(message)
            );
            if let Some(q) = qos {
                cmd.push_str(&format!(" -q {}", q));
            }
            if let Some(u) = username {
                cmd.push_str(&format!(" -u {}", shell_escape(u)));
            }
            if let Some(pw) = password {
                cmd.push_str(&format!(" -P {}", shell_escape(pw)));
            }
            if let Some(cid) = client_id {
                cmd.push_str(&format!(" -i {}", shell_escape(cid)));
            }
            run_command(&cmd, run_as, timeout_secs, cancel_rx).await
        }
        TaskType::Redis {
            url,
            channel,
            message,
        } => {
            let cmd = format!(
                "redis-cli -u {} PUBLISH {} {}",
                shell_escape(url),
                shell_escape(channel),
                shell_escape(message)
            );
            run_command(&cmd, run_as, timeout_secs, cancel_rx).await
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
        .timeout(
            timeout_secs
                .map(std::time::Duration::from_secs)
                .unwrap_or(std::time::Duration::from_secs(30)),
        )
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

                if let Some(expected) = expect_status
                    && status_code != expected
                {
                    return CommandResult {
                        status: ExecutionStatus::Failed,
                        exit_code: Some(status_code as i32),
                        stdout: CapturedOutput {
                            text: resp_body,
                            truncated: false,
                        },
                        stderr: CapturedOutput {
                            text: format!("expected status {}, got {}", expected, status_code),
                            truncated: false,
                        },
                    };
                }

                CommandResult {
                    status: if (200..300).contains(&status_code) {
                        ExecutionStatus::Succeeded
                    } else {
                        ExecutionStatus::Failed
                    },
                    exit_code: Some(status_code as i32),
                    stdout: CapturedOutput {
                        text: resp_body,
                        truncated: false,
                    },
                    stderr: CapturedOutput {
                        text: String::new(),
                        truncated: false,
                    },
                }
            }
            Err(e) => CommandResult {
                status: ExecutionStatus::Failed,
                exit_code: None,
                stdout: CapturedOutput {
                    text: String::new(),
                    truncated: false,
                },
                stderr: CapturedOutput {
                    text: format!("HTTP request failed: {e}"),
                    truncated: false,
                },
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

fn hex_to_bytes(hex: &str) -> Result<Vec<u8>, String> {
    let hex = hex.replace(' ', "");
    if !hex.len().is_multiple_of(2) {
        return Err("odd length".into());
    }
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).map_err(|e| format!("{e}")))
        .collect()
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

async fn run_script(
    code: &str,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    use rhai::{Engine, Scope};
    use std::sync::{Arc as StdArc, Mutex as StdMutex};

    let code = code.to_string();
    let timeout = timeout_secs
        .map(std::time::Duration::from_secs)
        .unwrap_or(std::time::Duration::from_secs(60));

    let script_future = tokio::task::spawn_blocking(move || {
        let mut engine = Engine::new();
        let output = StdArc::new(StdMutex::new(Vec::<String>::new()));
        let errors = StdArc::new(StdMutex::new(Vec::<String>::new()));

        // Limit execution
        engine.set_max_operations(1_000_000);
        engine.set_max_string_size(256 * 1024);

        // print() -> captures to output
        let out = output.clone();
        engine.on_print(move |s| {
            out.lock().unwrap().push(s.to_string());
        });

        // debug() -> captures to errors
        let err = errors.clone();
        engine.on_debug(move |s, _, _| {
            err.lock().unwrap().push(s.to_string());
        });

        // Register http_get(url) -> #{status, body}
        engine.register_fn("http_get", |url: &str| -> rhai::Dynamic {
            let url = url.to_string();
            let rt = tokio::runtime::Handle::try_current();
            let result = if let Ok(handle) = rt {
                let u = url.clone();
                std::thread::spawn(move || handle.block_on(async { reqwest::get(&u).await }))
                    .join()
                    .ok()
                    .and_then(|r| r.ok())
            } else {
                None
            };
            match result {
                Some(resp) => {
                    let status = resp.status().as_u16() as i64;
                    let body_result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        std::thread::spawn(move || handle.block_on(resp.text()))
                            .join()
                            .ok()
                            .and_then(|r| r.ok())
                    } else {
                        None
                    };
                    let body = body_result.unwrap_or_default();
                    let mut map = rhai::Map::new();
                    map.insert("status".into(), rhai::Dynamic::from(status));
                    map.insert("body".into(), rhai::Dynamic::from(body));
                    rhai::Dynamic::from(map)
                }
                None => {
                    let mut map = rhai::Map::new();
                    map.insert("status".into(), rhai::Dynamic::from(0_i64));
                    map.insert(
                        "body".into(),
                        rhai::Dynamic::from("request failed".to_string()),
                    );
                    rhai::Dynamic::from(map)
                }
            }
        });

        // Register http_post(url, body) -> #{status, body}
        engine.register_fn("http_post", |url: &str, body: &str| -> rhai::Dynamic {
            let url = url.to_string();
            let body = body.to_string();
            let rt = tokio::runtime::Handle::try_current();
            let result = if let Ok(handle) = rt {
                let u = url.clone();
                let b = body.clone();
                std::thread::spawn(move || {
                    handle.block_on(async { reqwest::Client::new().post(&u).body(b).send().await })
                })
                .join()
                .ok()
                .and_then(|r| r.ok())
            } else {
                None
            };
            match result {
                Some(resp) => {
                    let status = resp.status().as_u16() as i64;
                    let body_result = if let Ok(handle) = tokio::runtime::Handle::try_current() {
                        std::thread::spawn(move || handle.block_on(resp.text()))
                            .join()
                            .ok()
                            .and_then(|r| r.ok())
                    } else {
                        None
                    };
                    let resp_body = body_result.unwrap_or_default();
                    let mut map = rhai::Map::new();
                    map.insert("status".into(), rhai::Dynamic::from(status));
                    map.insert("body".into(), rhai::Dynamic::from(resp_body));
                    rhai::Dynamic::from(map)
                }
                None => {
                    let mut map = rhai::Map::new();
                    map.insert("status".into(), rhai::Dynamic::from(0_i64));
                    map.insert(
                        "body".into(),
                        rhai::Dynamic::from("request failed".to_string()),
                    );
                    rhai::Dynamic::from(map)
                }
            }
        });

        // Register shell_exec(cmd) -> #{exit_code, stdout, stderr}
        engine.register_fn("shell_exec", |cmd: &str| -> rhai::Dynamic {
            let output = std::process::Command::new("sh").arg("-c").arg(cmd).output();
            match output {
                Ok(out) => {
                    let mut map = rhai::Map::new();
                    map.insert(
                        "exit_code".into(),
                        rhai::Dynamic::from(out.status.code().unwrap_or(-1) as i64),
                    );
                    map.insert(
                        "stdout".into(),
                        rhai::Dynamic::from(String::from_utf8_lossy(&out.stdout).to_string()),
                    );
                    map.insert(
                        "stderr".into(),
                        rhai::Dynamic::from(String::from_utf8_lossy(&out.stderr).to_string()),
                    );
                    rhai::Dynamic::from(map)
                }
                Err(e) => {
                    let mut map = rhai::Map::new();
                    map.insert("exit_code".into(), rhai::Dynamic::from(-1_i64));
                    map.insert("stdout".into(), rhai::Dynamic::from("".to_string()));
                    map.insert(
                        "stderr".into(),
                        rhai::Dynamic::from(format!("exec error: {e}")),
                    );
                    rhai::Dynamic::from(map)
                }
            }
        });

        // Register sleep_ms(ms)
        engine.register_fn("sleep_ms", |ms: i64| {
            std::thread::sleep(std::time::Duration::from_millis(ms as u64));
        });

        // Register env_var(name) -> string
        engine.register_fn("env_var", |name: &str| -> String {
            std::env::var(name).unwrap_or_default()
        });

        // Register udp_send(addr, data) -> #{sent, error}
        engine.register_fn("udp_send", |addr: &str, data: &str| -> rhai::Dynamic {
            use std::net::UdpSocket;
            let mut map = rhai::Map::new();
            match UdpSocket::bind("0.0.0.0:0") {
                Ok(socket) => {
                    let _ = socket.set_write_timeout(Some(std::time::Duration::from_secs(5)));
                    match socket.send_to(data.as_bytes(), addr) {
                        Ok(n) => {
                            map.insert("sent".into(), rhai::Dynamic::from(n as i64));
                            map.insert("error".into(), rhai::Dynamic::from("".to_string()));
                        }
                        Err(e) => {
                            map.insert("sent".into(), rhai::Dynamic::from(0_i64));
                            map.insert("error".into(), rhai::Dynamic::from(format!("{e}")));
                        }
                    }
                }
                Err(e) => {
                    map.insert("sent".into(), rhai::Dynamic::from(0_i64));
                    map.insert("error".into(), rhai::Dynamic::from(format!("{e}")));
                }
            }
            rhai::Dynamic::from(map)
        });

        // Register tcp_send(addr, data) -> #{response, error}
        engine.register_fn("tcp_send", |addr: &str, data: &str| -> rhai::Dynamic {
            use std::io::{Read, Write};
            use std::net::TcpStream;
            let mut map = rhai::Map::new();
            match TcpStream::connect_timeout(
                &addr
                    .parse()
                    .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 0))),
                std::time::Duration::from_secs(5),
            ) {
                Ok(mut stream) => {
                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(5)));
                    match stream.write_all(data.as_bytes()) {
                        Ok(_) => {
                            let _ = stream.shutdown(std::net::Shutdown::Write);
                            let mut buf = Vec::new();
                            let _ = stream.read_to_end(&mut buf);
                            map.insert(
                                "response".into(),
                                rhai::Dynamic::from(String::from_utf8_lossy(&buf).to_string()),
                            );
                            map.insert("error".into(), rhai::Dynamic::from("".to_string()));
                        }
                        Err(e) => {
                            map.insert("response".into(), rhai::Dynamic::from("".to_string()));
                            map.insert("error".into(), rhai::Dynamic::from(format!("{e}")));
                        }
                    }
                }
                Err(e) => {
                    map.insert("response".into(), rhai::Dynamic::from("".to_string()));
                    map.insert("error".into(), rhai::Dynamic::from(format!("{e}")));
                }
            }
            rhai::Dynamic::from(map)
        });

        // Register udp_send_hex(addr, hex_string) -> #{sent, error}
        engine.register_fn("udp_send_hex", |addr: &str, hex: &str| -> rhai::Dynamic {
            use std::net::UdpSocket;
            let mut map = rhai::Map::new();
            let bytes = match hex_to_bytes(hex) {
                Ok(b) => b,
                Err(e) => {
                    map.insert("sent".into(), rhai::Dynamic::from(0_i64));
                    map.insert("error".into(), rhai::Dynamic::from(format!("bad hex: {e}")));
                    return rhai::Dynamic::from(map);
                }
            };
            match UdpSocket::bind("0.0.0.0:0") {
                Ok(socket) => {
                    let _ = socket.set_write_timeout(Some(std::time::Duration::from_secs(5)));
                    match socket.send_to(&bytes, addr) {
                        Ok(n) => {
                            map.insert("sent".into(), rhai::Dynamic::from(n as i64));
                            map.insert("error".into(), rhai::Dynamic::from("".to_string()));
                        }
                        Err(e) => {
                            map.insert("sent".into(), rhai::Dynamic::from(0_i64));
                            map.insert("error".into(), rhai::Dynamic::from(format!("{e}")));
                        }
                    }
                }
                Err(e) => {
                    map.insert("sent".into(), rhai::Dynamic::from(0_i64));
                    map.insert("error".into(), rhai::Dynamic::from(format!("{e}")));
                }
            }
            rhai::Dynamic::from(map)
        });

        // Register tcp_send_hex(addr, hex_string) -> #{response_hex, response, error}
        engine.register_fn("tcp_send_hex", |addr: &str, hex: &str| -> rhai::Dynamic {
            use std::io::{Read, Write};
            use std::net::TcpStream;
            let mut map = rhai::Map::new();
            let bytes = match hex_to_bytes(hex) {
                Ok(b) => b,
                Err(e) => {
                    map.insert("response_hex".into(), rhai::Dynamic::from("".to_string()));
                    map.insert("response".into(), rhai::Dynamic::from("".to_string()));
                    map.insert("error".into(), rhai::Dynamic::from(format!("bad hex: {e}")));
                    return rhai::Dynamic::from(map);
                }
            };
            match TcpStream::connect_timeout(
                &addr
                    .parse()
                    .unwrap_or_else(|_| std::net::SocketAddr::from(([127, 0, 0, 1], 0))),
                std::time::Duration::from_secs(5),
            ) {
                Ok(mut stream) => {
                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                    let _ = stream.set_write_timeout(Some(std::time::Duration::from_secs(5)));
                    match stream.write_all(&bytes) {
                        Ok(_) => {
                            let _ = stream.shutdown(std::net::Shutdown::Write);
                            let mut buf = Vec::new();
                            let _ = stream.read_to_end(&mut buf);
                            map.insert(
                                "response_hex".into(),
                                rhai::Dynamic::from(bytes_to_hex(&buf)),
                            );
                            map.insert(
                                "response".into(),
                                rhai::Dynamic::from(String::from_utf8_lossy(&buf).to_string()),
                            );
                            map.insert("error".into(), rhai::Dynamic::from("".to_string()));
                        }
                        Err(e) => {
                            map.insert("response_hex".into(), rhai::Dynamic::from("".to_string()));
                            map.insert("response".into(), rhai::Dynamic::from("".to_string()));
                            map.insert("error".into(), rhai::Dynamic::from(format!("{e}")));
                        }
                    }
                }
                Err(e) => {
                    map.insert("response_hex".into(), rhai::Dynamic::from("".to_string()));
                    map.insert("response".into(), rhai::Dynamic::from("".to_string()));
                    map.insert("error".into(), rhai::Dynamic::from(format!("{e}")));
                }
            }
            rhai::Dynamic::from(map)
        });

        // Register hex_encode(string) -> hex_string
        engine.register_fn("hex_encode", |data: &str| -> String {
            bytes_to_hex(data.as_bytes())
        });

        // Register hex_decode(hex_string) -> string
        engine.register_fn("hex_decode", |hex: &str| -> String {
            match hex_to_bytes(hex) {
                Ok(bytes) => String::from_utf8_lossy(&bytes).to_string(),
                Err(_) => String::new(),
            }
        });

        // Register fail(message) - marks execution as failed
        let fail_flag = StdArc::new(StdMutex::new(None::<String>));
        let ff = fail_flag.clone();
        engine.register_fn("fail", move |msg: &str| {
            *ff.lock().unwrap() = Some(msg.to_string());
        });

        let mut scope = Scope::new();
        let result = engine.eval_with_scope::<rhai::Dynamic>(&mut scope, &code);

        let stdout_lines = output.lock().unwrap().join("\n");
        let stderr_lines = errors.lock().unwrap().join("\n");
        let failed = fail_flag.lock().unwrap().clone();

        match result {
            Ok(val) => {
                let final_stdout = if stdout_lines.is_empty() {
                    format!("{}", val)
                } else {
                    format!("{}\n{}", stdout_lines, val)
                };
                if let Some(fail_msg) = failed {
                    (
                        ExecutionStatus::Failed,
                        None,
                        final_stdout,
                        format!("{}\n{}", stderr_lines, fail_msg).trim().to_string(),
                    )
                } else {
                    (
                        ExecutionStatus::Succeeded,
                        Some(0),
                        final_stdout,
                        stderr_lines,
                    )
                }
            }
            Err(e) => {
                let err_msg = format!("{}\n{}", stderr_lines, e).trim().to_string();
                (ExecutionStatus::Failed, Some(1), stdout_lines, err_msg)
            }
        }
    });

    tokio::select! {
        result = script_future => {
            match result {
                Ok((status, exit_code, stdout, stderr)) => CommandResult {
                    status,
                    exit_code,
                    stdout: CapturedOutput { text: stdout, truncated: false },
                    stderr: CapturedOutput { text: stderr, truncated: false },
                },
                Err(e) => CommandResult {
                    status: ExecutionStatus::Failed,
                    exit_code: None,
                    stdout: CapturedOutput { text: String::new(), truncated: false },
                    stderr: CapturedOutput { text: format!("script task panicked: {e}"), truncated: false },
                },
            }
        }
        _ = async {
            tokio::time::sleep(timeout).await
        } => {
            CommandResult {
                status: ExecutionStatus::TimedOut,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: format!("script timed out after {}s", timeout_secs.unwrap_or(60)), truncated: false },
            }
        }
        _ = cancel_rx => {
            CommandResult {
                status: ExecutionStatus::Cancelled,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: "script cancelled".to_string(), truncated: false },
            }
        }
    }
}

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
