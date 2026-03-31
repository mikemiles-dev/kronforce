use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::sync::oneshot;
use tracing::info;

use crate::db::models::ExecutionStatus;

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

// --- MCP Protocol Messages ---

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

fn make_tools_list_request() -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0",
        id: 2,
        method: "tools/list".to_string(),
        params: None,
    }
}

fn make_tool_call_request(tool: &str, arguments: Option<&Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0",
        id: 3,
        method: "tools/call".to_string(),
        params: Some(json!({
            "name": tool,
            "arguments": arguments.unwrap_or(&json!({}))
        })),
    }
}

// --- HTTP MCP Client (Streamable HTTP with SSE responses) ---

struct McpClient {
    client: reqwest::Client,
    url: String,
    session_id: Option<String>,
}

impl McpClient {
    fn new(url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.to_string(),
            session_id: None,
        }
    }

    async fn send(&mut self, msg: &Value) -> Result<JsonRpcResponse, String> {
        let body = serde_json::to_string(msg).map_err(|e| format!("serialize error: {e}"))?;
        let mut req = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        if let Some(ref sid) = self.session_id {
            req = req.header("Mcp-Session-Id", sid.clone());
        }

        let resp = req
            .body(body)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        // Capture session ID from response
        if let Some(sid) = resp.headers().get("mcp-session-id")
            && let Ok(s) = sid.to_str() {
                self.session_id = Some(s.to_string());
            }

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }

        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();

        let body = resp.text().await.map_err(|e| format!("read error: {e}"))?;

        // Parse based on content type
        if content_type.contains("text/event-stream") {
            // SSE format: lines like "event: message\ndata: {...}\n\n"
            parse_sse_response(&body)
        } else {
            // Plain JSON
            parse_json_response(&body)
        }
    }

    async fn notify(&mut self, msg: &Value) -> Result<(), String> {
        let body = serde_json::to_string(msg).map_err(|e| format!("serialize error: {e}"))?;
        let mut req = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .header("Accept", "application/json, text/event-stream");

        if let Some(ref sid) = self.session_id {
            req = req.header("Mcp-Session-Id", sid.clone());
        }

        let _ = req.body(body).send().await;
        Ok(())
    }

    async fn handshake(&mut self) -> Result<(), String> {
        let init = serde_json::to_value(make_initialize_request()).unwrap();
        let resp = self.send(&init).await?;
        if let Some(err) = resp.error {
            return Err(format!("handshake error: {}", err.message));
        }

        let notif = serde_json::to_value(make_initialized_notification()).unwrap();
        self.notify(&notif).await?;
        Ok(())
    }
}

/// Parse SSE (Server-Sent Events) body to extract the JSON-RPC response.
fn parse_sse_response(body: &str) -> Result<JsonRpcResponse, String> {
    for line in body.lines() {
        let line = line.trim();
        if let Some(data) = line.strip_prefix("data: ")
            && let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(data) {
                return Ok(resp);
            }
        // Also try parsing bare JSON lines (some servers don't use SSE framing)
        if line.starts_with('{')
            && let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(line)
                && resp.id.is_some() {
                    return Ok(resp);
                }
    }
    Err(format!("no JSON-RPC response found in SSE body: {}", &body[..body.len().min(200)]))
}

/// Parse plain JSON response body.
fn parse_json_response(body: &str) -> Result<JsonRpcResponse, String> {
    for line in body.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if let Ok(resp) = serde_json::from_str::<JsonRpcResponse>(line)
            && resp.id.is_some() {
                return Ok(resp);
            }
    }
    serde_json::from_str(body).map_err(|e| format!("JSON parse error: {e} — body: {}", &body[..body.len().min(200)]))
}

// --- Task Execution ---

pub async fn run_mcp_task(
    server_url: &str,
    tool: &str,
    arguments: Option<&Value>,
    timeout_secs: Option<u64>,
    cancel_rx: oneshot::Receiver<()>,
) -> CommandResult {
    let timeout_duration = timeout_secs.map(Duration::from_secs);

    let mcp_future = async {
        let mut client = McpClient::new(server_url);

        if let Err(e) = client.handshake().await {
            return make_error(-1, format!("MCP handshake failed: {e}"));
        }

        info!("MCP handshake complete, calling tool: {}", tool);

        let call = serde_json::to_value(make_tool_call_request(tool, arguments)).unwrap();
        let resp = match client.send(&call).await {
            Ok(r) => r,
            Err(e) => return make_error(1, format!("MCP tool call failed: {e}")),
        };

        map_response(resp)
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
                    let mime = item.get("mimeType").and_then(|m| m.as_str()).unwrap_or("image/*");
                    stdout_parts.push(format!("[image: {}]", mime));
                }
                "audio" => {
                    let mime = item.get("mimeType").and_then(|m| m.as_str()).unwrap_or("audio/*");
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
            stdout: CapturedOutput { text: String::new(), truncated: false },
            stderr: CapturedOutput { text: stdout, truncated: false },
        }
    } else {
        CommandResult {
            status: ExecutionStatus::Succeeded,
            exit_code: Some(0),
            stdout: CapturedOutput { text: stdout, truncated: false },
            stderr: CapturedOutput { text: String::new(), truncated: false },
        }
    }
}

fn make_error(exit_code: i32, message: String) -> CommandResult {
    CommandResult {
        status: ExecutionStatus::Failed,
        exit_code: Some(exit_code),
        stdout: CapturedOutput { text: String::new(), truncated: false },
        stderr: CapturedOutput { text: message, truncated: false },
    }
}

// --- Tool Discovery ---

pub async fn discover_tools(server_url: &str) -> Result<Vec<Value>, String> {
    let mut client = McpClient::new(server_url);
    client.handshake().await?;

    let list = serde_json::to_value(make_tools_list_request()).unwrap();
    let resp = client.send(&list).await?;

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
