use tokio::sync::oneshot;

use crate::db::models::ExecutionStatus;
use crate::executor::scripts::ScriptStore;

use super::super::{
    CapturedOutput, CommandResult, DEFAULT_SCRIPT_TIMEOUT_SECS, MAX_SCRIPT_OPERATIONS,
    MAX_SCRIPT_STRING_SIZE, bytes_to_hex, hex_to_bytes,
};

pub async fn run_script_task(
    script_name: &str,
    script_store: Option<&ScriptStore>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
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
        .unwrap_or(std::time::Duration::from_secs(DEFAULT_SCRIPT_TIMEOUT_SECS));

    let script_future = tokio::task::spawn_blocking(move || {
        let mut engine = Engine::new();
        let output = StdArc::new(StdMutex::new(Vec::<String>::new()));
        let errors = StdArc::new(StdMutex::new(Vec::<String>::new()));

        // Limit execution
        engine.set_max_operations(MAX_SCRIPT_OPERATIONS);
        engine.set_max_string_size(MAX_SCRIPT_STRING_SIZE);

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
                stderr: CapturedOutput { text: format!("script timed out after {}s", timeout_secs.unwrap_or(DEFAULT_SCRIPT_TIMEOUT_SECS)), truncated: false },
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
