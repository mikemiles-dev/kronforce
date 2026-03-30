use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::oneshot;
use tracing::info;

use crate::db::models::{ExecutionStatus, McpTransport};

use super::super::{CapturedOutput, CommandResult};

// --- JSON-RPC Types ---

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    params: Option<Value>,
}

#[derive(Serialize)]
struct JsonRpcNotification {
    jsonrpc: &'static str,
    method: String,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<Value>,
    error: Option<JsonRpcError>,
}

#[derive(Deserialize)]
struct JsonRpcError {
    #[allow(dead_code)]
    code: i64,
    message: String,
}

// --- MCP Protocol ---

fn make_initialize_request() -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: "initialize".to_string(),
        params: Some(json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "kronforce",
                "version": "0.1.0"
            }
        })),
    }
}

fn make_initialized_notification() -> JsonRpcNotification {
    JsonRpcNotification {
        jsonrpc: "2.0",
        method: "notifications/initialized".to_string(),
    }
}

fn make_tools_list_request(id: u64) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0",
        id,
        method: "tools/list".to_string(),
        params: None,
    }
}

fn make_tool_call_request(id: u64, tool: &str, arguments: Option<&Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0",
        id,
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": tool,
            "arguments": arguments.unwrap_or(&json!({}))
        })),
    }
}

// --- Stdio Transport ---

struct StdioTransport {
    child: tokio::process::Child,
    reader: BufReader<tokio::process::ChildStdout>,
    stdin: tokio::process::ChildStdin,
    stderr: tokio::process::ChildStderr,
}

impl StdioTransport {
    fn spawn(server_command: &str) -> Result<Self, String> {
        let mut cmd = if cfg!(windows) {
            let mut c = tokio::process::Command::new("cmd");
            c.args(["/C", server_command]);
            c
        } else {
            let mut c = tokio::process::Command::new("sh");
            c.args(["-c", server_command]);
            c
        };

        let mut child = cmd
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn MCP server: {e}"))?;

        let stdin = child.stdin.take().ok_or("no stdin")?;
        let stdout = child.stdout.take().ok_or("no stdout")?;
        let stderr = child.stderr.take().ok_or("no stderr")?;
        let reader = BufReader::new(stdout);

        Ok(Self {
            child,
            reader,
            stdin,
            stderr,
        })
    }

    async fn send(&mut self, msg: &[u8]) -> Result<(), String> {
        self.stdin
            .write_all(msg)
            .await
            .map_err(|e| format!("write error: {e}"))?;
        self.stdin
            .write_all(b"\n")
            .await
            .map_err(|e| format!("write error: {e}"))?;
        self.stdin
            .flush()
            .await
            .map_err(|e| format!("flush error: {e}"))?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<JsonRpcResponse, String> {
        loop {
            let mut line = String::new();
            let n = self
                .reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("read error: {e}"))?;
            if n == 0 {
                // Server closed — capture stderr for diagnostics
                let mut stderr_buf = Vec::new();
                let _ = tokio::io::AsyncReadExt::read_to_end(&mut self.stderr, &mut stderr_buf).await;
                let stderr_text = String::from_utf8_lossy(&stderr_buf);
                if stderr_text.is_empty() {
                    return Err("server closed connection".to_string());
                }
                return Err(format!("server closed connection — stderr: {}", stderr_text.trim()));
            }
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            // Skip notifications (no "id" field or "id": null with "method")
            if let Ok(v) = serde_json::from_str::<Value>(line)
                && v.get("method").is_some()
                && v.get("id").is_none()
            {
                continue; // notification, skip
            }
            return serde_json::from_str(line)
                .map_err(|e| format!("JSON parse error: {e} — line: {line}"));
        }
    }

    async fn shutdown(&mut self) {
        let _ = self.child.kill().await;
    }
}

// --- HTTP Transport ---

struct HttpTransport {
    client: reqwest::Client,
    url: String,
}

impl HttpTransport {
    fn new(url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.to_string(),
        }
    }

    async fn send_request(&self, msg: &[u8]) -> Result<JsonRpcResponse, String> {
        let resp = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(msg.to_vec())
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        let body = resp.text().await.map_err(|e| format!("read error: {e}"))?;

        // May receive multiple JSON-RPC messages; find the response (has "id")
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(line)
                && v.get("id").is_some()
            {
                return serde_json::from_str(line).map_err(|e| format!("JSON parse error: {e}"));
            }
        }

        // Try parsing the whole body as a single response
        serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {e}"))
    }

    async fn send_notification(&self, msg: &[u8]) -> Result<(), String> {
        let _ = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .body(msg.to_vec())
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;
        Ok(())
    }
}

// --- Tool Execution ---

pub async fn run_mcp_task(
    server: &str,
    transport: &McpTransport,
    tool: &str,
    arguments: Option<&Value>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let timeout_duration = timeout_secs.map(Duration::from_secs);

    let mcp_future = async {
        match transport {
            McpTransport::Stdio => run_stdio(server, tool, arguments).await,
            McpTransport::Http => run_http(server, tool, arguments).await,
        }
    };

    tokio::select! {
        result = mcp_future => result,
        _ = async {
            match timeout_duration {
                Some(d) => tokio::time::sleep(d).await,
                None => std::future::pending().await,
            }
        } => {
            CommandResult {
                status: ExecutionStatus::TimedOut,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: format!("MCP task timed out after {}s", timeout_secs.unwrap_or(0)), truncated: false },
            }
        }
        _ = cancel_rx => {
            CommandResult {
                status: ExecutionStatus::Cancelled,
                exit_code: None,
                stdout: CapturedOutput { text: String::new(), truncated: false },
                stderr: CapturedOutput { text: "MCP task cancelled by user".to_string(), truncated: false },
            }
        }
    }
}

async fn run_stdio(server: &str, tool: &str, arguments: Option<&Value>) -> CommandResult {
    let mut transport = match StdioTransport::spawn(server) {
        Ok(t) => t,
        Err(e) => return make_error(-1, e),
    };

    // 1. Initialize handshake
    let init_msg = serde_json::to_vec(&make_initialize_request()).unwrap();
    if let Err(e) = transport.send(&init_msg).await {
        transport.shutdown().await;
        return make_error(-1, format!("handshake send failed: {e}"));
    }

    let init_resp = match transport.receive().await {
        Ok(r) => r,
        Err(e) => {
            transport.shutdown().await;
            return make_error(-1, format!("handshake receive failed: {e}"));
        }
    };

    if let Some(err) = init_resp.error {
        transport.shutdown().await;
        return make_error(-1, format!("handshake error: {}", err.message));
    }

    // 2. Send initialized notification
    let notif_msg = serde_json::to_vec(&make_initialized_notification()).unwrap();
    if let Err(e) = transport.send(&notif_msg).await {
        transport.shutdown().await;
        return make_error(-1, format!("initialized notification failed: {e}"));
    }

    info!("MCP handshake complete (stdio), calling tool: {}", tool);

    // 3. Call tool
    let call_msg = serde_json::to_vec(&make_tool_call_request(2, tool, arguments)).unwrap();
    if let Err(e) = transport.send(&call_msg).await {
        transport.shutdown().await;
        return make_error(1, format!("tool call send failed: {e}"));
    }

    let call_resp = match transport.receive().await {
        Ok(r) => r,
        Err(e) => {
            transport.shutdown().await;
            return make_error(1, format!("tool call receive failed: {e}"));
        }
    };

    transport.shutdown().await;
    map_response(call_resp)
}

async fn run_http(server: &str, tool: &str, arguments: Option<&Value>) -> CommandResult {
    let transport = HttpTransport::new(server);

    // 1. Initialize handshake
    let init_msg = serde_json::to_vec(&make_initialize_request()).unwrap();
    let init_resp = match transport.send_request(&init_msg).await {
        Ok(r) => r,
        Err(e) => return make_error(-1, format!("handshake failed: {e}")),
    };

    if let Some(err) = init_resp.error {
        return make_error(-1, format!("handshake error: {}", err.message));
    }

    // 2. Send initialized notification
    let notif_msg = serde_json::to_vec(&make_initialized_notification()).unwrap();
    let _ = transport.send_notification(&notif_msg).await;

    info!("MCP handshake complete (HTTP), calling tool: {}", tool);

    // 3. Call tool
    let call_msg = serde_json::to_vec(&make_tool_call_request(2, tool, arguments)).unwrap();
    let call_resp = match transport.send_request(&call_msg).await {
        Ok(r) => r,
        Err(e) => return make_error(1, format!("tool call failed: {e}")),
    };

    map_response(call_resp)
}

fn map_response(resp: JsonRpcResponse) -> CommandResult {
    if let Some(err) = resp.error {
        return make_error(1, format!("tool error: {}", err.message));
    }

    let result = match resp.result {
        Some(r) => r,
        None => return make_error(1, "empty tool response".to_string()),
    };

    let is_error = result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    // Extract text from content array
    let mut stdout_parts = Vec::new();
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        for item in content {
            let content_type = item.get("type").and_then(|t| t.as_str()).unwrap_or("");
            match content_type {
                "text" => {
                    if let Some(text) = item.get("text").and_then(|t| t.as_str()) {
                        stdout_parts.push(text.to_string());
                    }
                }
                "image" => {
                    let mime = item
                        .get("mimeType")
                        .and_then(|m| m.as_str())
                        .unwrap_or("image/*");
                    stdout_parts.push(format!("[image: {}]", mime));
                }
                "audio" => {
                    let mime = item
                        .get("mimeType")
                        .and_then(|m| m.as_str())
                        .unwrap_or("audio/*");
                    stdout_parts.push(format!("[audio: {}]", mime));
                }
                "resource" | "resource_link" => {
                    let uri = item
                        .get("uri")
                        .or_else(|| item.get("resource").and_then(|r| r.get("uri")))
                        .and_then(|u| u.as_str())
                        .unwrap_or("unknown");
                    stdout_parts.push(format!("[resource: {}]", uri));
                }
                _ => {
                    stdout_parts.push(format!("[{}: ...]", content_type));
                }
            }
        }
    }

    let stdout = stdout_parts.join("\n");

    if is_error {
        CommandResult {
            status: ExecutionStatus::Failed,
            exit_code: Some(1),
            stdout: CapturedOutput {
                text: String::new(),
                truncated: false,
            },
            stderr: CapturedOutput {
                text: stdout,
                truncated: false,
            },
        }
    } else {
        CommandResult {
            status: ExecutionStatus::Succeeded,
            exit_code: Some(0),
            stdout: CapturedOutput {
                text: stdout,
                truncated: false,
            },
            stderr: CapturedOutput {
                text: String::new(),
                truncated: false,
            },
        }
    }
}

fn make_error(exit_code: i32, message: String) -> CommandResult {
    CommandResult {
        status: ExecutionStatus::Failed,
        exit_code: Some(exit_code),
        stdout: CapturedOutput {
            text: String::new(),
            truncated: false,
        },
        stderr: CapturedOutput {
            text: message,
            truncated: false,
        },
    }
}

// --- Tool Discovery ---

pub async fn discover_tools(server: &str, transport: &McpTransport) -> Result<Vec<Value>, String> {
    match transport {
        McpTransport::Stdio => discover_stdio(server).await,
        McpTransport::Http => discover_http(server).await,
    }
}

async fn discover_stdio(server: &str) -> Result<Vec<Value>, String> {
    let mut transport = StdioTransport::spawn(server)?;

    // Handshake
    let init_msg = serde_json::to_vec(&make_initialize_request()).unwrap();
    transport.send(&init_msg).await?;
    let init_resp = transport.receive().await?;
    if let Some(err) = init_resp.error {
        transport.shutdown().await;
        return Err(format!("handshake error: {}", err.message));
    }
    let notif_msg = serde_json::to_vec(&make_initialized_notification()).unwrap();
    transport.send(&notif_msg).await?;

    // List tools
    let list_msg = serde_json::to_vec(&make_tools_list_request(2)).unwrap();
    transport.send(&list_msg).await?;
    let list_resp = transport.receive().await?;
    transport.shutdown().await;

    extract_tools(list_resp)
}

async fn discover_http(server: &str) -> Result<Vec<Value>, String> {
    let transport = HttpTransport::new(server);

    // Handshake
    let init_msg = serde_json::to_vec(&make_initialize_request()).unwrap();
    let init_resp = transport.send_request(&init_msg).await?;
    if let Some(err) = init_resp.error {
        return Err(format!("handshake error: {}", err.message));
    }
    let notif_msg = serde_json::to_vec(&make_initialized_notification()).unwrap();
    let _ = transport.send_notification(&notif_msg).await;

    // List tools
    let list_msg = serde_json::to_vec(&make_tools_list_request(2)).unwrap();
    let list_resp = transport.send_request(&list_msg).await?;

    extract_tools(list_resp)
}

fn extract_tools(resp: JsonRpcResponse) -> Result<Vec<Value>, String> {
    if let Some(err) = resp.error {
        return Err(format!("tools/list error: {}", err.message));
    }
    let result = resp.result.ok_or("empty tools/list response")?;
    let tools = result
        .get("tools")
        .and_then(|t| t.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(tools)
}
