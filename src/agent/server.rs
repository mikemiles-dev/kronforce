use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::State;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use tokio::sync::{Mutex, oneshot};
use uuid::Uuid;

use tracing::{error, info, warn};

use crate::agent::protocol::{
    CancelRequest, ExecutionResultReport, JobDispatchRequest, JobDispatchResponse,
};
use crate::executor::run_task;

/// Shared state for the agent HTTP server, holding identity, controller URL, and running executions.
#[derive(Clone)]
pub struct AgentState {
    pub agent_id: Uuid,
    pub controller_url: String,
    pub http_client: reqwest::Client,
    pub running: Arc<Mutex<HashMap<Uuid, oneshot::Sender<()>>>>,
    pub agent_key: Option<String>,
}

/// Builds the agent's HTTP router with execute, cancel, health, and shutdown routes.
pub fn router(state: AgentState) -> Router {
    Router::new()
        .route("/execute", post(execute_job))
        .route("/cancel", post(cancel_job))
        .route("/health", get(health))
        .route("/shutdown", post(shutdown))
        .with_state(state)
}

/// Validates that the request has the correct agent key. Returns error response if invalid.
fn validate_agent_auth(
    headers: &axum::http::HeaderMap,
    expected_key: &Option<String>,
) -> Result<(), (axum::http::StatusCode, Json<serde_json::Value>)> {
    let Some(key) = expected_key else {
        return Ok(()); // No key configured, skip auth
    };

    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));

    match auth {
        Some(token) if token == key => Ok(()),
        _ => Err((
            axum::http::StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error": "invalid or missing agent key"})),
        )),
    }
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status": "ok"}))
}

async fn execute_job(
    State(state): State<AgentState>,
    req: axum::extract::Request,
) -> Result<Json<JobDispatchResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    validate_agent_auth(req.headers(), &state.agent_key)?;
    let body = axum::body::to_bytes(req.into_body(), 1024 * 1024 * 10)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("bad body: {e}")})),
            )
        })?;
    let req: JobDispatchRequest = serde_json::from_slice(&body).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("bad json: {e}")})),
        )
    })?;
    Ok(execute_job_inner(state, req).await)
}

async fn execute_job_inner(
    state: AgentState,
    req: JobDispatchRequest,
) -> Json<JobDispatchResponse> {
    let exec_id = req.execution_id;

    // Create cancel channel
    let (cancel_tx, cancel_rx) = oneshot::channel::<()>();
    {
        let mut running = state.running.lock().await;
        running.insert(exec_id, cancel_tx);
    }

    let running = state.running.clone();
    let client = state.http_client.clone();
    let agent_id = state.agent_id;
    let agent_key = state.agent_key.clone();

    tokio::spawn(async move {
        let started_at = Utc::now();

        let result = run_task(
            &req.task,
            req.run_as.as_deref(),
            req.timeout_secs,
            None,
            cancel_rx,
        )
        .await;

        let finished_at = Utc::now();

        // Remove from running map
        running.lock().await.remove(&exec_id);

        // Build result report
        let report = ExecutionResultReport {
            execution_id: exec_id,
            job_id: req.job_id,
            agent_id,
            status: result.status,
            exit_code: result.exit_code,
            stdout: result.stdout.text,
            stderr: result.stderr.text,
            stdout_truncated: result.stdout.truncated,
            stderr_truncated: result.stderr.truncated,
            started_at,
            finished_at,
        };

        // POST result back to controller with exponential backoff
        let max_attempts = 5u32;
        for attempt in 1..=max_attempts {
            let mut cb_req = client.post(&req.callback_url).json(&report);
            if let Some(ref key) = agent_key {
                cb_req = cb_req.header("Authorization", format!("Bearer {}", key));
            }
            match cb_req.send().await {
                Ok(resp) if resp.status().is_success() => {
                    info!("reported result for execution {exec_id}");
                    break;
                }
                Ok(resp) => {
                    warn!(
                        "callback failed for {exec_id} (attempt {attempt}/{max_attempts}): status {}",
                        resp.status()
                    );
                }
                Err(e) => {
                    warn!("callback failed for {exec_id} (attempt {attempt}/{max_attempts}): {e}");
                }
            }
            if attempt >= max_attempts {
                error!(
                    "giving up on callback for execution {exec_id} after {max_attempts} attempts — result lost"
                );
                break;
            }
            // Backoff: 2s, 4s, 8s, 16s (capped)
            let delay = std::time::Duration::from_secs(2u64.pow(attempt).min(16));
            tokio::time::sleep(delay).await;
        }
    });

    Json(JobDispatchResponse {
        accepted: true,
        message: None,
    })
}

async fn cancel_job(
    State(state): State<AgentState>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    validate_agent_auth(req.headers(), &state.agent_key)?;
    let body = axum::body::to_bytes(req.into_body(), 1024 * 64)
        .await
        .map_err(|e| {
            (
                axum::http::StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error": format!("{e}")})),
            )
        })?;
    let cancel: CancelRequest = serde_json::from_slice(&body).map_err(|e| {
        (
            axum::http::StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": format!("{e}")})),
        )
    })?;
    let mut running = state.running.lock().await;
    if let Some(tx) = running.remove(&cancel.execution_id) {
        let _ = tx.send(());
        Ok(Json(serde_json::json!({"cancelled": true})))
    } else {
        Ok(Json(
            serde_json::json!({"cancelled": false, "message": "not running"}),
        ))
    }
}

async fn shutdown(
    State(state): State<AgentState>,
    req: axum::extract::Request,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    validate_agent_auth(req.headers(), &state.agent_key)?;
    info!("shutdown requested by controller, exiting...");
    tokio::spawn(async {
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        std::process::exit(0);
    });
    Ok(Json(serde_json::json!({"status": "shutting_down"})))
}
