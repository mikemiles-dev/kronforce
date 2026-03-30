use std::time::Duration;

use axum::Json;
use axum::extract::{Query, State};
use serde::Deserialize;

use super::AppState;
use crate::error::AppError;
use crate::executor::tasks::mcp::discover_tools;

#[derive(Deserialize)]
pub(crate) struct DiscoverToolsQuery {
    server_url: String,
}

pub(crate) async fn mcp_discover_tools(
    State(_state): State<AppState>,
    Query(query): Query<DiscoverToolsQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    // 15 second timeout — MCP servers can be slow to respond
    let result = tokio::time::timeout(
        Duration::from_secs(15),
        discover_tools(&query.server_url),
    )
    .await
    .map_err(|_| AppError::BadRequest("MCP discovery timed out after 15 seconds".to_string()))?
    .map_err(|e| AppError::BadRequest(format!("MCP discovery failed: {e}")))?;

    Ok(Json(serde_json::json!({ "tools": result })))
}
