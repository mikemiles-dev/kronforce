//! MCP (Model Context Protocol) server endpoint.
//!
//! Exposes Kronforce operations as MCP tools that AI assistants and MCP clients
//! can discover and invoke via the Streamable HTTP transport.

use axum::body::Body;
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::response::Response;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::api::AppState;
use crate::db::db_call;
use crate::db::models::*;
use crate::scheduler::SchedulerCommand;

// --- JSON-RPC Types ---

#[derive(Deserialize)]
struct JsonRpcMessage {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: Option<String>,
    params: Option<Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: &'static str,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl JsonRpcResponse {
    fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Value, code: i64, message: String) -> Self {
        Self {
            jsonrpc: "2.0",
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

// --- Tool Definition ---

struct McpTool {
    name: &'static str,
    description: &'static str,
    input_schema: Value,
    min_role: MinRole,
}

#[derive(Clone, Copy)]
enum MinRole {
    Viewer,
    Operator,
}

impl MinRole {
    fn allowed_for(self, role: &ApiKeyRole) -> bool {
        match self {
            MinRole::Viewer => true,
            MinRole::Operator => role.can_write(),
        }
    }
}

fn tool_definitions() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "list_jobs",
            description: "List jobs with optional filters. Returns job name, status, group, schedule, and last execution.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "group": {"type": "string", "description": "Filter by group name"},
                    "status": {"type": "string", "description": "Filter by status (scheduled, paused, unscheduled)"},
                    "search": {"type": "string", "description": "Search in job name and task"},
                    "limit": {"type": "integer", "description": "Max results (default 20)", "default": 20}
                }
            }),
            min_role: MinRole::Viewer,
        },
        McpTool {
            name: "get_job",
            description: "Get full details of a job by name or ID.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Job name (exact match)"},
                    "id": {"type": "string", "description": "Job UUID"}
                }
            }),
            min_role: MinRole::Viewer,
        },
        McpTool {
            name: "create_job",
            description: "Create a new job. Returns the new job ID.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Job name"},
                    "task": {"type": "object", "description": "Task definition (e.g. {\"type\":\"shell\",\"command\":\"echo hello\"})"},
                    "schedule": {"type": "object", "description": "Schedule (e.g. {\"type\":\"on_demand\"} or {\"type\":\"cron\",\"value\":\"0 * * * * *\"})"},
                    "group": {"type": "string", "description": "Job group (default: Default)"},
                    "description": {"type": "string", "description": "Job description"},
                    "timeout_secs": {"type": "integer", "description": "Timeout in seconds"}
                },
                "required": ["name", "task", "schedule"]
            }),
            min_role: MinRole::Operator,
        },
        McpTool {
            name: "trigger_job",
            description: "Trigger a job to run immediately. Returns execution info.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "name": {"type": "string", "description": "Job name"},
                    "id": {"type": "string", "description": "Job UUID"}
                }
            }),
            min_role: MinRole::Operator,
        },
        McpTool {
            name: "list_executions",
            description: "List recent job executions with status and output excerpt.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "status": {"type": "string", "description": "Filter by status (succeeded, failed, running, timed_out)"},
                    "limit": {"type": "integer", "description": "Max results (default 20)", "default": 20}
                }
            }),
            min_role: MinRole::Viewer,
        },
        McpTool {
            name: "get_execution",
            description: "Get full execution details including stdout, stderr, and exit code.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "id": {"type": "string", "description": "Execution UUID"}
                },
                "required": ["id"]
            }),
            min_role: MinRole::Viewer,
        },
        McpTool {
            name: "list_agents",
            description: "List all registered agents with status, type, and tags.",
            input_schema: json!({"type": "object", "properties": {}}),
            min_role: MinRole::Viewer,
        },
        McpTool {
            name: "list_groups",
            description: "List all job group names.",
            input_schema: json!({"type": "object", "properties": {}}),
            min_role: MinRole::Viewer,
        },
        McpTool {
            name: "list_events",
            description: "List recent system events.",
            input_schema: json!({
                "type": "object",
                "properties": {
                    "limit": {"type": "integer", "description": "Max results (default 20)", "default": 20}
                }
            }),
            min_role: MinRole::Viewer,
        },
        McpTool {
            name: "get_system_stats",
            description: "Get system overview: job counts, execution stats, agent status, group counts.",
            input_schema: json!({"type": "object", "properties": {}}),
            min_role: MinRole::Viewer,
        },
    ]
}

// --- SSE Response Helpers ---

fn sse_response(body: &JsonRpcResponse, session_id: &str) -> Response {
    let json = serde_json::to_string(body).unwrap_or_default();
    let sse_body = format!("event: message\ndata: {}\n\n", json);

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache, no-transform")
        .header("Mcp-Session-Id", session_id)
        .body(Body::from(sse_body))
        .unwrap()
}

fn accepted_response() -> Response {
    Response::builder()
        .status(StatusCode::ACCEPTED)
        .body(Body::empty())
        .unwrap()
}

fn error_response(status: StatusCode, message: &str) -> Response {
    let body = json!({"jsonrpc": "2.0", "id": "server-error", "error": {"code": -32600, "message": message}});
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

// --- Main Handler ---

pub async fn mcp_handler(State(state): State<AppState>, req: Request) -> Response {
    // Validate Accept header
    let accept = req
        .headers()
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !accept.contains("application/json") || !accept.contains("text/event-stream") {
        return error_response(
            StatusCode::NOT_ACCEPTABLE,
            "Not Acceptable: Client must accept both application/json and text/event-stream",
        );
    }

    // Get API key for role checking
    let api_key = req.extensions().get::<ApiKey>().cloned();
    let role = api_key
        .as_ref()
        .map(|k| k.role)
        .unwrap_or(ApiKeyRole::Admin); // no keys = full access

    // Generate or extract session ID
    let session_id = req
        .headers()
        .get("mcp-session-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Parse JSON-RPC message
    let body_bytes = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
        Ok(b) => b,
        Err(e) => {
            return error_response(StatusCode::BAD_REQUEST, &format!("invalid body: {e}"));
        }
    };

    let msg: JsonRpcMessage = match serde_json::from_slice(&body_bytes) {
        Ok(m) => m,
        Err(e) => {
            return error_response(StatusCode::BAD_REQUEST, &format!("invalid JSON-RPC: {e}"));
        }
    };

    let method = msg.method.as_deref().unwrap_or("");
    let id = msg.id.clone().unwrap_or(Value::Null);

    // Notifications (no id) get 202
    if msg.id.is_none() {
        return accepted_response();
    }

    match method {
        "initialize" => {
            let resp = JsonRpcResponse::success(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {"listChanged": false}
                    },
                    "serverInfo": {
                        "name": "kronforce",
                        "version": "0.1.0"
                    }
                }),
            );
            sse_response(&resp, &session_id)
        }
        "tools/list" => {
            let tools: Vec<Value> = tool_definitions()
                .into_iter()
                .filter(|t| t.min_role.allowed_for(&role))
                .map(|t| {
                    json!({
                        "name": t.name,
                        "description": t.description,
                        "inputSchema": t.input_schema,
                    })
                })
                .collect();

            let resp = JsonRpcResponse::success(id, json!({"tools": tools}));
            sse_response(&resp, &session_id)
        }
        "tools/call" => {
            let params = msg.params.unwrap_or(json!({}));
            let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

            // Check tool exists and role is sufficient
            let tool_def = tool_definitions().into_iter().find(|t| t.name == tool_name);
            let Some(tool) = tool_def else {
                let resp = JsonRpcResponse::success(
                    id,
                    json!({"content": [{"type": "text", "text": format!("Unknown tool: {}", tool_name)}], "isError": true}),
                );
                return sse_response(&resp, &session_id);
            };

            if !tool.min_role.allowed_for(&role) {
                let resp = JsonRpcResponse::success(
                    id,
                    json!({"content": [{"type": "text", "text": "Insufficient permissions for this tool"}], "isError": true}),
                );
                return sse_response(&resp, &session_id);
            }

            let result = execute_tool(tool_name, &arguments, &state).await;
            let resp = match result {
                Ok(text) => JsonRpcResponse::success(
                    id,
                    json!({"content": [{"type": "text", "text": text}], "isError": false}),
                ),
                Err(err) => JsonRpcResponse::success(
                    id,
                    json!({"content": [{"type": "text", "text": err}], "isError": true}),
                ),
            };
            sse_response(&resp, &session_id)
        }
        "ping" => {
            let resp = JsonRpcResponse::success(id, json!({}));
            sse_response(&resp, &session_id)
        }
        _ => {
            let resp = JsonRpcResponse::error(id, -32601, format!("Method not found: {method}"));
            sse_response(&resp, &session_id)
        }
    }
}

// --- Tool Execution ---

async fn execute_tool(name: &str, args: &Value, state: &AppState) -> Result<String, String> {
    match name {
        "list_jobs" => tool_list_jobs(args, state).await,
        "get_job" => tool_get_job(args, state).await,
        "create_job" => tool_create_job(args, state).await,
        "trigger_job" => tool_trigger_job(args, state).await,
        "list_executions" => tool_list_executions(args, state).await,
        "get_execution" => tool_get_execution(args, state).await,
        "list_agents" => tool_list_agents(state).await,
        "list_groups" => tool_list_groups(state).await,
        "list_events" => tool_list_events(args, state).await,
        "get_system_stats" => tool_get_system_stats(state).await,
        _ => Err(format!("Unknown tool: {name}")),
    }
}

async fn tool_list_jobs(args: &Value, state: &AppState) -> Result<String, String> {
    let group = args.get("group").and_then(|v| v.as_str()).map(String::from);
    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .map(String::from);
    let search = args
        .get("search")
        .and_then(|v| v.as_str())
        .map(String::from);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as u32;

    let jobs = db_call(&state.db, move |db| {
        db.list_jobs(
            status.as_deref(),
            search.as_deref(),
            group.as_deref(),
            limit,
            0,
        )
    })
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let summaries: Vec<Value> = jobs
        .iter()
        .map(|j| {
            json!({
                "id": j.id.to_string(),
                "name": j.name,
                "status": j.status.as_str(),
                "group": j.group,
                "schedule": format!("{:?}", j.schedule),
            })
        })
        .collect();

    serde_json::to_string_pretty(&summaries).map_err(|e| format!("serialize error: {e}"))
}

async fn tool_get_job(args: &Value, state: &AppState) -> Result<String, String> {
    let job = if let Some(id_str) = args.get("id").and_then(|v| v.as_str()) {
        let id = Uuid::parse_str(id_str).map_err(|e| format!("invalid UUID: {e}"))?;
        db_call(&state.db, move |db| db.get_job(id))
            .await
            .map_err(|e| format!("DB error: {e}"))?
    } else if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
        let name = name.to_string();
        let jobs = db_call(&state.db, move |db| {
            db.list_jobs(None, Some(&name), None, 1, 0)
        })
        .await
        .map_err(|e| format!("DB error: {e}"))?;
        jobs.into_iter().next()
    } else {
        return Err("provide 'name' or 'id' argument".to_string());
    };

    match job {
        Some(j) => serde_json::to_string_pretty(&j).map_err(|e| format!("serialize error: {e}")),
        None => Err("job not found".to_string()),
    }
}

async fn tool_create_job(args: &Value, state: &AppState) -> Result<String, String> {
    let name = args
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or("'name' is required")?
        .to_string();
    let task: TaskType =
        serde_json::from_value(args.get("task").cloned().ok_or("'task' is required")?)
            .map_err(|e| format!("invalid task: {e}"))?;
    let schedule: ScheduleKind = serde_json::from_value(
        args.get("schedule")
            .cloned()
            .ok_or("'schedule' is required")?,
    )
    .map_err(|e| format!("invalid schedule: {e}"))?;

    let group = args
        .get("group")
        .and_then(|v| v.as_str())
        .unwrap_or("Default")
        .to_string();
    let description = args
        .get("description")
        .and_then(|v| v.as_str())
        .map(String::from);
    let timeout_secs = args.get("timeout_secs").and_then(|v| v.as_u64());

    let job_id = Uuid::new_v4();
    let now = chrono::Utc::now();
    let job = Job {
        id: job_id,
        name: name.clone(),
        description,
        task,
        run_as: None,
        schedule,
        status: JobStatus::Scheduled,
        timeout_secs,
        depends_on: vec![],
        target: None,
        created_by: None,
        created_at: now,
        updated_at: now,
        output_rules: None,
        notifications: None,
        group: Some(group),
        retry_max: 0,
        retry_delay_secs: 0,
        retry_backoff: 1.0,
    };

    let job_clone = job.clone();
    db_call(&state.db, move |db| db.insert_job(&job_clone))
        .await
        .map_err(|e| format!("create failed: {e}"))?;

    let _ = state.scheduler_tx.send(SchedulerCommand::Reload).await;

    Ok(format!("Job '{}' created with ID {}", name, job_id))
}

async fn tool_trigger_job(args: &Value, state: &AppState) -> Result<String, String> {
    let job = if let Some(id_str) = args.get("id").and_then(|v| v.as_str()) {
        let id = Uuid::parse_str(id_str).map_err(|e| format!("invalid UUID: {e}"))?;
        db_call(&state.db, move |db| db.get_job(id))
            .await
            .map_err(|e| format!("DB error: {e}"))?
    } else if let Some(name) = args.get("name").and_then(|v| v.as_str()) {
        let name = name.to_string();
        let jobs = db_call(&state.db, move |db| {
            db.list_jobs(None, Some(&name), None, 1, 0)
        })
        .await
        .map_err(|e| format!("DB error: {e}"))?;
        jobs.into_iter().next()
    } else {
        return Err("provide 'name' or 'id' argument".to_string());
    };

    let job = job.ok_or("job not found")?;

    state
        .scheduler_tx
        .send(SchedulerCommand::TriggerNow(job.id))
        .await
        .map_err(|_| "scheduler unavailable".to_string())?;

    Ok(format!("Job '{}' triggered", job.name))
}

async fn tool_list_executions(args: &Value, state: &AppState) -> Result<String, String> {
    let status = args
        .get("status")
        .and_then(|v| v.as_str())
        .map(String::from);
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as u32;

    let execs = db_call(&state.db, move |db| {
        db.list_all_executions(status.as_deref(), None, None, limit, 0)
    })
    .await
    .map_err(|e| format!("DB error: {e}"))?;

    let summaries: Vec<Value> = execs
        .iter()
        .map(|e| {
            json!({
                "id": e.id.to_string(),
                "job_id": e.job_id.to_string(),
                "status": e.status.as_str(),
                "exit_code": e.exit_code,
                "started_at": e.started_at.map(|t| t.to_rfc3339()),
                "finished_at": e.finished_at.map(|t| t.to_rfc3339()),
                "stdout_excerpt": e.stdout.chars().take(200).collect::<String>(),
            })
        })
        .collect();

    serde_json::to_string_pretty(&summaries).map_err(|e| format!("serialize error: {e}"))
}

async fn tool_get_execution(args: &Value, state: &AppState) -> Result<String, String> {
    let id_str = args
        .get("id")
        .and_then(|v| v.as_str())
        .ok_or("'id' is required")?;
    let id = Uuid::parse_str(id_str).map_err(|e| format!("invalid UUID: {e}"))?;

    let exec = db_call(&state.db, move |db| db.get_execution(id))
        .await
        .map_err(|e| format!("DB error: {e}"))?
        .ok_or("execution not found")?;

    serde_json::to_string_pretty(&exec).map_err(|e| format!("serialize error: {e}"))
}

async fn tool_list_agents(state: &AppState) -> Result<String, String> {
    let agents = db_call(&state.db, |db| db.list_agents())
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let summaries: Vec<Value> = agents
        .iter()
        .map(|a| {
            json!({
                "id": a.id.to_string(),
                "name": a.name,
                "status": a.status.as_str(),
                "agent_type": a.agent_type.as_str(),
                "tags": a.tags,
                "hostname": a.hostname,
            })
        })
        .collect();

    serde_json::to_string_pretty(&summaries).map_err(|e| format!("serialize error: {e}"))
}

async fn tool_list_groups(state: &AppState) -> Result<String, String> {
    let groups = db_call(&state.db, |db| db.get_distinct_groups())
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    serde_json::to_string_pretty(&groups).map_err(|e| format!("serialize error: {e}"))
}

async fn tool_list_events(args: &Value, state: &AppState) -> Result<String, String> {
    let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(20) as u32;

    let events = db_call(&state.db, move |db| db.list_events(None, limit, 0))
        .await
        .map_err(|e| format!("DB error: {e}"))?;

    let summaries: Vec<Value> = events
        .iter()
        .map(|e| {
            json!({
                "kind": e.kind,
                "severity": e.severity.as_str(),
                "message": e.message,
                "timestamp": e.timestamp.to_rfc3339(),
            })
        })
        .collect();

    serde_json::to_string_pretty(&summaries).map_err(|e| format!("serialize error: {e}"))
}

async fn tool_get_system_stats(state: &AppState) -> Result<String, String> {
    let db1 = state.db.clone();
    let db2 = state.db.clone();
    let db3 = state.db.clone();
    let db4 = state.db.clone();

    let job_count = db_call(&db1, |db| db.count_jobs(None, None, None))
        .await
        .unwrap_or(0);
    let outcomes = db_call(&db2, |db| db.get_execution_outcome_counts())
        .await
        .unwrap_or_default();
    let agents = db_call(&db3, |db| db.list_agents())
        .await
        .unwrap_or_default();
    let groups = db_call(&db4, |db| db.get_distinct_groups())
        .await
        .unwrap_or_default();

    let online_agents = agents
        .iter()
        .filter(|a| a.status == AgentStatus::Online)
        .count();

    let stats = json!({
        "total_jobs": job_count,
        "total_agents": agents.len(),
        "online_agents": online_agents,
        "total_groups": groups.len(),
        "execution_outcomes": outcomes,
    });

    serde_json::to_string_pretty(&stats).map_err(|e| format!("serialize error: {e}"))
}
