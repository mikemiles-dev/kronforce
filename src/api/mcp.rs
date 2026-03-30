use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use super::AppState;
use crate::db::models::McpTransport;
use crate::error::AppError;
use crate::executor::tasks::mcp::discover_tools;

#[derive(Deserialize)]
pub(crate) struct DiscoverToolsQuery {
    server: String,
    transport: Option<String>,
}

pub(crate) async fn mcp_discover_tools(
    State(_state): State<AppState>,
    Query(query): Query<DiscoverToolsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let transport = match query.transport.as_deref() {
        Some("http") => McpTransport::Http,
        _ => McpTransport::Stdio,
    };

    let tools = discover_tools(&query.server, &transport)
        .await
        .map_err(|e| AppError::BadRequest(format!("MCP discovery failed: {e}")))?;

    Ok(Json(serde_json::json!({ "tools": tools })))
}
