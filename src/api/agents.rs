use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use uuid::Uuid;

use super::AppState;
use crate::error::AppError;
use crate::models::*;
use crate::protocol::{AgentHeartbeat, AgentRegistration, AgentRegistrationResponse};

pub(crate) async fn register_agent(
    State(state): State<AppState>,
    Json(req): Json<AgentRegistration>,
) -> Result<Json<AgentRegistrationResponse>, AppError> {
    let db = state.db.clone();
    let name = req.name.clone();

    // Check if agent with same name exists (re-registration)
    let existing = tokio::task::spawn_blocking({
        let db = db.clone();
        let name = name.clone();
        move || db.get_agent_by_name(&name)
    })
    .await
    .unwrap()?;

    let agent_id = existing.as_ref().map(|a| a.id).unwrap_or_else(Uuid::new_v4);
    let now = Utc::now();

    let agent_type = req.agent_type.as_deref().map(AgentType::from_str).unwrap_or(AgentType::Standard);

    // Preserve existing UI-managed task types on re-registration
    let task_types = existing.as_ref().map(|a| a.task_types.clone()).unwrap_or_default();

    let agent = Agent {
        id: agent_id,
        name: req.name,
        tags: req.tags,
        hostname: req.hostname,
        address: req.address,
        port: req.port,
        agent_type,
        status: AgentStatus::Online,
        last_heartbeat: Some(now),
        registered_at: existing.map(|a| a.registered_at).unwrap_or(now),
        task_types,
    };

    let db2 = db.clone();
    let agent2 = agent.clone();
    tokio::task::spawn_blocking(move || db2.upsert_agent(&agent2))
        .await
        .unwrap()?;

    tracing::info!("agent registered: {} ({})", agent.name, agent.id);

    let db_log = state.db.clone();
    let agent_name = agent.name.clone();
    let agent_id_log = agent.id;
    let _ = tokio::task::spawn_blocking(move || {
        db_log.log_event("agent.registered", EventSeverity::Success, &format!("Agent '{}' registered", agent_name), None, Some(agent_id_log))
    }).await;

    Ok(Json(AgentRegistrationResponse {
        agent_id: agent.id,
        heartbeat_interval_secs: 10,
    }))
}

pub(crate) async fn agent_heartbeat(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(_hb): Json<AgentHeartbeat>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.clone();
    let now = Utc::now();
    tokio::task::spawn_blocking(move || db.update_agent_heartbeat(id, now))
        .await
        .unwrap()?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

pub(crate) async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<Vec<Agent>>, AppError> {
    let db = state.db.clone();
    let agents = tokio::task::spawn_blocking(move || db.list_agents())
        .await
        .unwrap()?;
    Ok(Json(agents))
}

pub(crate) async fn get_agent_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Agent>, AppError> {
    let db = state.db.clone();
    let agent = tokio::task::spawn_blocking(move || db.get_agent(id))
        .await
        .unwrap()?
        .ok_or_else(|| AppError::NotFound(format!("agent {id} not found")))?;
    Ok(Json(agent))
}

pub(crate) async fn deregister_agent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    // Look up agent to get address before deleting
    let db = state.db.clone();
    let agent = tokio::task::spawn_blocking(move || db.get_agent(id))
        .await
        .unwrap()?;

    // Send shutdown signal to agent (best-effort)
    if let Some(ref a) = agent {
        let _ = state.agent_client.shutdown_agent(&a.address, a.port).await;
        tracing::info!("sent shutdown to agent {} ({})", a.name, a.id);
    }

    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.delete_agent(id))
        .await
        .unwrap()?;

    if let Some(a) = agent {
        let db_log = state.db.clone();
        let name = a.name.clone();
        let _ = tokio::task::spawn_blocking(move || {
            db_log.log_event("agent.unpaired", EventSeverity::Warning, &format!("Agent '{}' unpaired and shut down", name), None, Some(id))
        }).await;
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub(crate) async fn get_agent_task_types(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<crate::models::TaskTypeDefinition>>, AppError> {
    let db = state.db.clone();
    let agent = tokio::task::spawn_blocking(move || db.get_agent(id))
        .await
        .unwrap()?;
    match agent {
        Some(a) => Ok(Json(a.task_types)),
        None => Err(AppError::NotFound("agent not found".into())),
    }
}

pub(crate) async fn update_agent_task_types(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let task_types: Vec<crate::models::TaskTypeDefinition> = serde_json::from_value(
        body.get("task_types").cloned().unwrap_or(serde_json::Value::Array(vec![]))
    ).map_err(|e| AppError::BadRequest(format!("invalid task_types: {e}")))?;

    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.update_agent_task_types(id, &task_types))
        .await
        .unwrap()?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

pub(crate) async fn poll_agent_queue(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<Response, AppError> {
    // Also update heartbeat
    let db = state.db.clone();
    let now = Utc::now();
    let aid = agent_id;
    let _ = tokio::task::spawn_blocking(move || db.update_agent_heartbeat(aid, now)).await;

    let db = state.db.clone();
    let job = tokio::task::spawn_blocking(move || db.dequeue_job(agent_id))
        .await
        .unwrap()?;

    match job {
        Some(j) => Ok(Json(j).into_response()),
        None => Ok(axum::http::StatusCode::NO_CONTENT.into_response()),
    }
}
