use axum::Json;
use axum::extract::{Path, State};
use axum::response::{IntoResponse, Response};
use chrono::Utc;
use uuid::Uuid;

use super::AppState;
use super::auth::AuthUser;
use crate::db::db_call;
use crate::db::models::*;
use crate::error::AppError;
use tracing::info;

use crate::agent::protocol::{AgentHeartbeat, AgentRegistration, AgentRegistrationResponse};

/// Handles agent registration or re-registration, upserting the agent record.
pub(crate) async fn register_agent(
    State(state): State<AppState>,
    Json(req): Json<AgentRegistration>,
) -> Result<Json<AgentRegistrationResponse>, AppError> {
    let name = req.name.clone();

    // Check if agent with same name exists (re-registration)
    let name_clone = name.clone();
    let existing = db_call(&state.db, move |db| db.get_agent_by_name(&name_clone)).await?;

    let agent_id = existing.as_ref().map(|a| a.id).unwrap_or_else(Uuid::new_v4);
    let now = Utc::now();

    let agent_type = req
        .agent_type
        .as_deref()
        .map(AgentType::from_str)
        .unwrap_or(AgentType::Standard);

    // Preserve existing UI-managed task types on re-registration
    let task_types = existing
        .as_ref()
        .map(|a| a.task_types.clone())
        .unwrap_or_default();

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

    let agent2 = agent.clone();
    db_call(&state.db, move |db| db.upsert_agent(&agent2)).await?;

    info!("agent registered: {} ({})", agent.name, agent.id);

    let agent_name = agent.name.clone();
    let agent_id_log = agent.id;
    let _ = db_call(&state.db, move |db| {
        db.log_event(
            "agent.registered",
            EventSeverity::Success,
            &format!("Agent '{}' registered", agent_name),
            None,
            Some(agent_id_log),
        )
    })
    .await;

    Ok(Json(AgentRegistrationResponse {
        agent_id: agent.id,
        heartbeat_interval_secs: 10,
    }))
}

/// Processes an agent heartbeat, updating its last-seen timestamp.
pub(crate) async fn agent_heartbeat(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(_hb): Json<AgentHeartbeat>,
) -> Result<Json<serde_json::Value>, AppError> {
    let now = Utc::now();
    db_call(&state.db, move |db| db.update_agent_heartbeat(id, now)).await?;
    Ok(Json(serde_json::json!({"status": "ok"})))
}

/// Returns all registered agents.
pub(crate) async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<Vec<Agent>>, AppError> {
    let agents = db_call(&state.db, move |db| db.list_agents()).await?;
    Ok(Json(agents))
}

/// Returns a single agent by ID.
pub(crate) async fn get_agent_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Agent>, AppError> {
    let agent = db_call(&state.db, move |db| db.get_agent(id))
        .await?
        .ok_or_else(|| AppError::NotFound(format!("agent {id} not found")))?;
    Ok(Json(agent))
}

/// Deregisters an agent, sends a shutdown signal, and deletes the record.
pub(crate) async fn deregister_agent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, AppError> {
    // Look up agent to get address before deleting
    let agent = db_call(&state.db, move |db| db.get_agent(id)).await?;

    // Send shutdown signal to agent (best-effort)
    if let Some(ref a) = agent {
        let _ = state.agent_client.shutdown_agent(&a.address, a.port).await;
        info!("sent shutdown to agent {} ({})", a.name, a.id);
    }

    db_call(&state.db, move |db| db.delete_agent(id)).await?;

    if let Some(a) = agent {
        let name = a.name.clone();
        let _ = db_call(&state.db, move |db| {
            db.log_event(
                "agent.unpaired",
                EventSeverity::Warning,
                &format!("Agent '{}' unpaired and shut down", name),
                None,
                Some(id),
            )
        })
        .await;
    }

    let actor_key_id = auth.0.as_ref().map(|k| k.id);
    let actor_key_name = auth.0.as_ref().map(|k| k.name.clone());
    let id_str = id.to_string();
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "agent.deregistered",
            "agent",
            Some(&id_str),
            actor_key_id,
            actor_key_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Returns the task type definitions for a specific agent.
pub(crate) async fn get_agent_task_types(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<TaskTypeDefinition>>, AppError> {
    let agent = db_call(&state.db, move |db| db.get_agent(id)).await?;
    match agent {
        Some(a) => Ok(Json(a.task_types)),
        None => Err(AppError::NotFound("agent not found".into())),
    }
}

/// Replaces the task type definitions for an agent.
pub(crate) async fn update_agent_task_types(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let task_types: Vec<TaskTypeDefinition> = serde_json::from_value(
        body.get("task_types")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )
    .map_err(|e| AppError::BadRequest(format!("invalid task_types: {e}")))?;

    db_call(&state.db, move |db| {
        db.update_agent_task_types(id, &task_types)
    })
    .await?;

    Ok(Json(serde_json::json!({ "status": "ok" })))
}

/// Polls the job queue for the next pending item for this agent. Also updates heartbeat.
pub(crate) async fn poll_agent_queue(
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> Result<Response, AppError> {
    // Also update heartbeat
    let now = Utc::now();
    let aid = agent_id;
    let _ = db_call(&state.db, move |db| db.update_agent_heartbeat(aid, now)).await;

    let job = db_call(&state.db, move |db| db.dequeue_job(agent_id)).await?;

    match job {
        Some(j) => Ok(Json(j).into_response()),
        None => Ok(axum::http::StatusCode::NO_CONTENT.into_response()),
    }
}
