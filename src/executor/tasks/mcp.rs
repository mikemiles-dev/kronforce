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

// --- HTTP MCP Client ---

struct McpClient {
    client: reqwest::Client,
    url: String,
}

impl McpClient {
    fn new(url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            url: url.to_string(),
        }
    }

    async fn send(&self, msg: &Value) -> Result<JsonRpcResponse, String> {
        let resp = self
            .client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .json(msg)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(format!("HTTP {status}: {body}"));
        }

        let body = resp.text().await.map_err(|e| format!("read error: {e}"))?;

        // Response may contain multiple JSON-RPC messages; find the one with an id
        for line in body.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(parsed) = serde_json::from_str::<JsonRpcResponse>(line) {
                if parsed.id.is_some() {
                    return Ok(parsed);
                }
            }
        }

        // Try the whole body as a single response
        serde_json::from_str(&body).map_err(|e| format!("JSON parse error: {e} — body: {body}"))
    }

    async fn notify(&self, msg: &Value) -> Result<(), String> {
        self.client
            .post(&self.url)
            .header("Content-Type", "application/json")
            .json(msg)
            .send()
            .await
            .map_err(|e| format!("HTTP error: {e}"))?;
        Ok(())
    }

    async fn handshake(&self) -> Result<(), String> {
        let init = serde_json::to_value(&make_initialize_request()).unwrap();
        let resp = self.send(&init).await?;
        if let Some(err) = resp.error {
            return Err(format!("handshake error: {}", err.message));
        }

        let notif = serde_json::to_value(&make_initialized_notification()).unwrap();
        self.notify(&notif).await?;
        Ok(())
    }
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
        let client = McpClient::new(server_url);

        // 1. Handshake
        if let Err(e) = client.handshake().await {
            return make_error(-1, format!("MCP handshake failed: {e}"));
        }

        info!("MCP handshake complete, calling tool: {}", tool);

        // 2. Call tool
        let call = serde_json::to_value(&make_tool_call_request(tool, arguments)).unwrap();
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
    let client = McpClient::new(server_url);
    client.handshake().await?;

    let list = serde_json::to_value(&make_tools_list_request()).unwrap();
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
