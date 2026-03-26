pub mod auth;
mod jobs;
mod executions;
mod agents;
mod events;
mod settings;
mod scripts;
mod callbacks;

use axum::middleware;
use axum::routing::{get, post, put, delete};
use axum::response::Html;
use axum::{Json, Router};
use serde::Serialize;
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::agent::AgentClient;
use crate::dag::DagResolver;
use crate::db::Db;
use crate::models::*;
use crate::scheduler::SchedulerCommand;

pub use auth::{hash_api_key, generate_api_key};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub dag: DagResolver,
    pub scheduler_tx: mpsc::Sender<SchedulerCommand>,
    pub agent_client: AgentClient,
    pub callback_base_url: String,
    pub script_store: crate::scripts::ScriptStore,
}

const DASHBOARD_HTML: &str = include_str!("../dashboard.html");

#[derive(Serialize)]
pub(crate) struct PaginatedResponse<T: serde::Serialize> {
    data: T,
    total: u32,
    page: u32,
    per_page: u32,
    total_pages: u32,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

pub fn router(state: AppState) -> Router {
    // Routes that require auth
    let authed = Router::new()
        .route("/api/jobs", get(jobs::list_jobs).post(jobs::create_job))
        .route(
            "/api/jobs/{id}",
            get(jobs::get_job_handler).put(jobs::update_job).delete(jobs::delete_job),
        )
        .route("/api/jobs/{id}/trigger", post(jobs::trigger_job))
        .route("/api/jobs/{id}/executions", get(executions::list_executions))
        .route("/api/executions", get(executions::list_all_executions))
        .route("/api/executions/{id}", get(executions::get_execution))
        .route("/api/executions/{id}/cancel", post(executions::cancel_execution))
        .route("/api/agents", get(agents::list_agents))
        .route("/api/agents/{id}", get(agents::get_agent_handler).delete(agents::deregister_agent))
        .route("/api/agents/{id}/task-types", put(agents::update_agent_task_types))
        .route("/api/events", get(events::list_events))
        .route("/api/timeline", get(events::get_timeline))
        .route("/api/timeline/{job_id}", get(events::get_job_timeline))
        .route("/api/timeline-detail/{bucket}", get(events::get_timeline_detail))
        .route("/api/keys", get(auth::list_api_keys).post(auth::create_api_key))
        .route("/api/keys/{id}", delete(auth::revoke_api_key))
        .route("/api/auth/me", get(auth::auth_me))
        .route("/api/scripts", get(scripts::list_scripts))
        .route("/api/scripts/{name}", get(scripts::get_script).put(scripts::save_script).delete(scripts::delete_script))
        .route("/api/settings", get(settings::get_settings).put(settings::update_settings))
        .route("/api/notifications/test", post(settings::test_notification))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::auth_middleware))
        .with_state(state.clone());

    // Agent endpoints — require agent key when KRONFORCE_AGENT_KEY is set
    let agent_authed = Router::new()
        .route("/api/agents/register", post(agents::register_agent))
        .route("/api/agent-queue/{agent_id}/next", get(agents::poll_agent_queue))
        .route("/api/agents/{id}/heartbeat", post(agents::agent_heartbeat))
        .route("/api/callbacks/execution-result", post(callbacks::execution_result_callback))
        .route("/api/agents/{id}/task-types", get(agents::get_agent_task_types))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth::agent_auth_middleware))
        .with_state(state.clone());

    // Routes exempt from all auth
    let public = Router::new()
        .route("/", get(dashboard))
        .route("/api/health", get(health))
        .with_state(state);

    public.merge(authed).merge(agent_authed)
}

async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

/// Log an event and notify the scheduler for event-triggered jobs
pub(crate) async fn log_and_notify(
    db: &Db,
    scheduler_tx: &mpsc::Sender<SchedulerCommand>,
    kind: &str,
    severity: EventSeverity,
    message: &str,
    job_id: Option<Uuid>,
    agent_id: Option<Uuid>,
    auth: &auth::AuthUser,
    details: Option<String>,
) {
    let event = Event {
        id: Uuid::new_v4(),
        kind: kind.to_string(),
        severity,
        message: message.to_string(),
        job_id,
        agent_id,
        api_key_id: auth.0.as_ref().map(|k| k.id),
        api_key_name: auth.0.as_ref().map(|k| k.name.clone()),
        details,
        timestamp: chrono::Utc::now(),
    };
    let db2 = db.clone();
    let event2 = event.clone();
    let _ = tokio::task::spawn_blocking(move || db2.insert_event(&event2)).await;
    let _ = scheduler_tx.send(SchedulerCommand::EventOccurred(event)).await;
}
