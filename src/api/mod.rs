//! REST API layer for the Kronforce controller.
//!
//! Defines all HTTP routes, request/response types, authentication middleware,
//! and the shared application state used by handlers.

mod agents;
mod ai;
mod audit;
pub mod auth;
mod callbacks;
mod connections;
mod data;
mod events;
mod executions;
mod jobs;
mod mcp;
pub mod oidc;
pub mod rate_limit;
mod scripts;
mod settings;
mod stats;
mod templates;
mod variables;

use axum::extract::State;
use axum::middleware;
use axum::response::Html;
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use serde::Serialize;
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use uuid::Uuid;

use std::sync::Arc;

use crate::agent::AgentClient;
use crate::dag::DagResolver;
use crate::db::Db;
use crate::db::models::*;
use crate::executor::scripts::ScriptStore;
use crate::scheduler::SchedulerCommand;

pub use auth::{generate_api_key, hash_api_key};

/// Shared application state passed to all API route handlers.
#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub dag: DagResolver,
    pub scheduler_tx: mpsc::Sender<SchedulerCommand>,
    pub agent_client: AgentClient,
    pub callback_base_url: String,
    pub script_store: ScriptStore,
    pub oidc: Option<Arc<oidc::OidcState>>,
    pub demo_mode: bool,
    pub live_output: Arc<dashmap::DashMap<Uuid, tokio::sync::broadcast::Sender<String>>>,
    /// Per-agent notification channels for instant job dispatch to custom agents.
    pub agent_notify: Arc<dashmap::DashMap<Uuid, Arc<tokio::sync::Notify>>>,
    pub ai_api_key: Option<String>,
    pub ai_provider: String,
    pub ai_model: Option<String>,
    /// True when the controller is bound to a loopback address. Auth-disabled mode
    /// (no API keys configured) is restricted to loopback so a non-loopback deploy
    /// without keys does not silently expose every endpoint.
    pub bind_is_loopback: bool,
    /// One-shot tickets for SSE streaming endpoints. Tickets are minted via
    /// POST /api/executions/{id}/stream-ticket (authenticated) and consumed by
    /// GET /api/executions/{id}/stream?ticket=... so the raw API key never lands
    /// on a URL (and therefore in proxy/access logs and Referer headers).
    pub stream_tickets: Arc<dashmap::DashMap<String, StreamTicket>>,
}

/// One-shot SSE auth ticket. Bound to a single execution and a near-future expiry.
#[derive(Clone, Debug)]
pub struct StreamTicket {
    pub execution_id: Uuid,
    pub expires_at: chrono::DateTime<chrono::Utc>,
}

const DASHBOARD_HTML: &str = include_str!(concat!(env!("OUT_DIR"), "/dashboard.html"));

/// Generic paginated API response wrapper.
#[derive(Serialize)]
pub(crate) struct PaginatedResponse<T: serde::Serialize> {
    pub(crate) data: T,
    pub(crate) total: u32,
    pub(crate) page: u32,
    pub(crate) per_page: u32,
    pub(crate) total_pages: u32,
}

/// Parse page/per_page from query params with defaults and clamping.
pub(crate) fn paginate(page: Option<u32>, per_page: Option<u32>) -> (u32, u32, u32) {
    let page = page.unwrap_or(1).max(1);
    let per_page = per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;
    (page, per_page, offset)
}

/// Build a PaginatedResponse from data, total count, page, and per_page.
pub(crate) fn paginated_response<T: serde::Serialize>(
    data: T,
    total: u32,
    page: u32,
    per_page: u32,
) -> PaginatedResponse<T> {
    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(per_page)
    };
    PaginatedResponse {
        data,
        total,
        page,
        per_page,
        total_pages,
    }
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    db: Option<DbHealth>,
}

#[derive(Serialize)]
pub struct DbHealth {
    pub ok: bool,
    pub size_bytes: Option<u64>,
    pub wal_size_bytes: Option<u64>,
    pub pool_size: u32,
}

/// Builds the complete Axum router with all API, agent, and public routes.
pub fn router(
    state: AppState,
    rate_limiters: rate_limit::RateLimiters,
    mcp_enabled: bool,
) -> Router {
    // Routes that require auth
    let authed = Router::new()
        .route("/api/jobs", get(jobs::list_jobs).post(jobs::create_job))
        .route(
            "/api/jobs/{id}",
            get(jobs::get_job_handler)
                .put(jobs::update_job)
                .delete(jobs::delete_job),
        )
        .route("/api/jobs/{id}/trigger", post(jobs::trigger_job))
        .route("/api/jobs/{id}/versions", get(jobs::list_job_versions))
        .route(
            "/api/jobs/{id}/webhook",
            post(jobs::generate_webhook).delete(jobs::delete_webhook),
        )
        .route(
            "/api/jobs/groups",
            get(jobs::list_groups).post(jobs::create_group),
        )
        .route("/api/jobs/bulk-group", put(jobs::bulk_set_group))
        .route("/api/jobs/rename-group", put(jobs::rename_group))
        .route(
            "/api/jobs/pipeline-schedule/{group}",
            get(jobs::get_pipeline_schedule)
                .put(jobs::set_pipeline_schedule)
                .delete(jobs::delete_pipeline_schedule),
        )
        .route(
            "/api/jobs/{id}/executions",
            get(executions::list_executions),
        )
        .route("/api/executions", get(executions::list_all_executions))
        .route(
            "/api/executions/recent-statuses",
            get(executions::recent_statuses),
        )
        .route("/api/executions/{id}", get(executions::get_execution))
        .route(
            "/api/executions/{id}/stream",
            get(executions::stream_execution),
        )
        .route(
            "/api/executions/{id}/stream-ticket",
            post(executions::create_stream_ticket),
        )
        .route(
            "/api/executions/{id}/cancel",
            post(executions::cancel_execution),
        )
        .route(
            "/api/executions/{id}/approve",
            post(jobs::approve_execution),
        )
        .route("/api/agents", get(agents::list_agents))
        .route(
            "/api/agents/{id}",
            get(agents::get_agent_handler).delete(agents::deregister_agent),
        )
        .route(
            "/api/agents/{id}/task-types",
            put(agents::update_agent_task_types),
        )
        .route("/api/events", get(events::list_events))
        .route("/api/timeline", get(events::get_timeline))
        .route("/api/timeline/{job_id}", get(events::get_job_timeline))
        .route(
            "/api/timeline-detail/{bucket}",
            get(events::get_timeline_detail),
        )
        .route(
            "/api/keys",
            get(auth::list_api_keys).post(auth::create_api_key),
        )
        .route("/api/keys/{id}", delete(auth::revoke_api_key))
        .route("/api/auth/me", get(auth::auth_me))
        .route("/api/scripts", get(scripts::list_scripts))
        .route(
            "/api/scripts/{name}",
            get(scripts::get_script)
                .put(scripts::save_script)
                .delete(scripts::delete_script),
        )
        .route(
            "/api/settings",
            get(settings::get_settings).put(settings::update_settings),
        )
        .route("/api/notifications/test", post(settings::test_notification))
        .route("/api/stats/charts", get(stats::chart_stats))
        .route("/api/mcp/tools", get(mcp::mcp_discover_tools))
        .route("/api/audit-log", get(audit::list_audit_log))
        .route("/api/ai/generate-job", post(ai::ai_generate_job))
        .route("/api/ai/models", get(ai::ai_list_models))
        .route("/api/data/export", get(data::export_data))
        .route("/api/data/import", post(data::import_data))
        .route("/api/data/delete", delete(data::delete_all_data))
        .route(
            "/api/templates",
            get(templates::list_templates).post(templates::save_template),
        )
        .route(
            "/api/templates/{name}",
            get(templates::get_template).delete(templates::delete_template),
        )
        .route(
            "/api/variables",
            get(variables::list_variables).post(variables::create_variable),
        )
        .route(
            "/api/variables/{name}",
            get(variables::get_variable)
                .put(variables::update_variable)
                .delete(variables::delete_variable),
        )
        .route(
            "/api/connections",
            get(connections::list_connections).post(connections::create_connection),
        )
        .route(
            "/api/connections/{name}",
            get(connections::get_connection)
                .put(connections::update_connection)
                .delete(connections::delete_connection),
        )
        .route(
            "/api/connections/{name}/test",
            post(connections::test_connection),
        )
        .route_layer(middleware::from_fn(
            rate_limit::rate_limit_authed_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::auth_middleware,
        ))
        .with_state(state.clone());

    // Agent endpoints — require agent key when KRONFORCE_AGENT_KEY is set
    let agent_authed = Router::new()
        .route("/api/agents/register", post(agents::register_agent))
        .route(
            "/api/agent-queue/{agent_id}/next",
            get(agents::poll_agent_queue),
        )
        .route("/api/agents/{id}/heartbeat", post(agents::agent_heartbeat))
        .route(
            "/api/callbacks/execution-result",
            post(callbacks::execution_result_callback),
        )
        .route(
            "/api/agents/{id}/task-types",
            get(agents::get_agent_task_types),
        )
        .route_layer(middleware::from_fn(rate_limit::rate_limit_agent_middleware))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth::agent_auth_middleware,
        ))
        .with_state(state.clone());

    // Routes exempt from all auth
    let public = Router::new()
        .route("/", get(dashboard))
        .route("/api/health", get(health))
        .route("/api/config", get(public_config))
        .route("/metrics", get(stats::prometheus_metrics))
        .route("/api/auth/oidc/config", get(oidc::oidc_config))
        .route("/api/auth/oidc/login", get(oidc::oidc_login))
        .route("/api/auth/oidc/callback", get(oidc::oidc_callback))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/webhooks/{token}", post(jobs::webhook_trigger))
        .route_layer(middleware::from_fn(
            rate_limit::rate_limit_public_middleware,
        ))
        .with_state(state.clone());

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let security_headers = axum::middleware::from_fn(add_security_headers);

    let mut app = public.merge(authed).merge(agent_authed);

    if mcp_enabled {
        let mcp_route = Router::new()
            .route("/mcp", post(crate::mcp_server::mcp_handler))
            .route_layer(middleware::from_fn_with_state(
                state.clone(),
                auth::auth_middleware,
            ))
            .with_state(state);
        app = app.merge(mcp_route);
    }

    app.layer(axum::Extension(rate_limiters))
        .layer(cors)
        .layer(security_headers)
}

async fn add_security_headers(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> axum::response::Response {
    let mut resp = next.run(req).await;
    let headers = resp.headers_mut();
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );
    resp
}

async fn dashboard() -> impl axum::response::IntoResponse {
    (
        [(
            axum::http::header::CACHE_CONTROL,
            "no-cache, no-store, must-revalidate",
        )],
        Html(DASHBOARD_HTML),
    )
}

async fn public_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    // Check if AI key is configured via DB settings or env var
    let db = state.db.clone();
    let db_ai_key =
        tokio::task::spawn_blocking(move || db.get_setting("ai_api_key").unwrap_or(None))
            .await
            .unwrap_or(None);
    let ai_enabled = db_ai_key.filter(|k| !k.is_empty()).is_some() || state.ai_api_key.is_some();

    Json(serde_json::json!({
        "demo_mode": state.demo_mode,
        "ai_enabled": ai_enabled,
    }))
}

async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let db = state.db.clone();
    let db_health = tokio::task::spawn_blocking(move || db.health_check())
        .await
        .ok()
        .flatten();

    let status = if db_health.as_ref().is_some_and(|h| h.ok) {
        "ok"
    } else {
        "degraded"
    };

    Json(HealthResponse {
        status: status.to_string(),
        db: db_health,
    })
}

/// Log an event and notify the scheduler for event-triggered jobs
#[allow(clippy::too_many_arguments)]
pub(crate) async fn log_and_notify(
    db: &Db,
    scheduler_tx: &mpsc::Sender<SchedulerCommand>,
    kind: &str,
    severity: EventSeverity,
    message: &str,
    job_id: Option<Uuid>,
    agent_id: Option<Uuid>,
    execution_id: Option<Uuid>,
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
        execution_id,
        details,
        timestamp: chrono::Utc::now(),
    };
    let db2 = db.clone();
    let event2 = event.clone();
    if let Err(e) = tokio::task::spawn_blocking(move || db2.insert_event(&event2)).await {
        tracing::warn!("failed to log event: {e}");
    }
    if let Err(e) = scheduler_tx
        .send(SchedulerCommand::EventOccurred(event))
        .await
    {
        tracing::warn!("failed to notify scheduler of event: {e}");
    }
}
