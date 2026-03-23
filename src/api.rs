use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::response::Html;
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::agent_client::AgentClient;
use crate::cron_parser::CronSchedule;
use crate::dag::DagResolver;
use crate::db::Db;
use crate::error::AppError;
use crate::models::*;
use crate::protocol::{AgentHeartbeat, AgentRegistration, AgentRegistrationResponse, ExecutionResultReport};
use crate::scheduler::SchedulerCommand;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub dag: DagResolver,
    pub scheduler_tx: mpsc::Sender<SchedulerCommand>,
    pub agent_client: AgentClient,
    pub callback_base_url: String,
}

const DASHBOARD_HTML: &str = include_str!("dashboard.html");

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(dashboard))
        .route("/api/jobs", get(list_jobs).post(create_job))
        .route(
            "/api/jobs/{id}",
            get(get_job).put(update_job).delete(delete_job),
        )
        .route("/api/jobs/{id}/trigger", post(trigger_job))
        .route("/api/jobs/{id}/executions", get(list_executions))
        .route("/api/executions/{id}", get(get_execution))
        .route("/api/executions/{id}/cancel", post(cancel_execution))
        .route("/api/health", get(health))
        .route("/api/agents/register", post(register_agent))
        .route("/api/agents/{id}/heartbeat", post(agent_heartbeat))
        .route("/api/agents", get(list_agents))
        .route("/api/agents/{id}", get(get_agent_handler).delete(deregister_agent))
        .route("/api/callbacks/execution-result", post(execution_result_callback))
        .route("/api/events", get(list_events))
        .with_state(state)
}

// --- Request/Response types ---

#[derive(Deserialize)]
struct CreateJobRequest {
    name: String,
    description: Option<String>,
    command: String,
    schedule: ScheduleKind,
    timeout_secs: Option<u64>,
    depends_on: Option<Vec<Dependency>>,
    target: Option<AgentTarget>,
}

#[derive(Deserialize)]
struct UpdateJobRequest {
    name: Option<String>,
    description: Option<String>,
    command: Option<String>,
    schedule: Option<ScheduleKind>,
    status: Option<JobStatus>,
    timeout_secs: Option<u64>,
    depends_on: Option<Vec<Dependency>>,
    target: Option<AgentTarget>,
}

#[derive(Serialize)]
struct LastExecution {
    id: uuid::Uuid,
    status: crate::models::ExecutionStatus,
    exit_code: Option<i32>,
    finished_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Serialize)]
struct ExecutionCounts {
    total: u32,
    succeeded: u32,
    failed: u32,
}

#[derive(Serialize)]
struct JobResponse {
    #[serde(flatten)]
    job: Job,
    next_fire_time: Option<chrono::DateTime<Utc>>,
    last_execution: Option<LastExecution>,
    execution_counts: ExecutionCounts,
}

#[derive(Serialize)]
struct PaginatedResponse<T: serde::Serialize> {
    data: T,
    total: u32,
    page: u32,
    per_page: u32,
    total_pages: u32,
}

#[derive(Deserialize)]
struct ListJobsQuery {
    status: Option<String>,
    search: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Deserialize)]
struct ListExecsQuery {
    limit: Option<u32>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Serialize)]
struct TriggerResponse {
    message: String,
    job_id: Uuid,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
}

// --- Handlers ---

async fn dashboard() -> Html<&'static str> {
    Html(DASHBOARD_HTML)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
    })
}

async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<ListJobsQuery>,
) -> Result<Json<PaginatedResponse<Vec<JobResponse>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;
    let filter_owned = query.status.clone();
    let search_owned = query.search.clone();

    let db = state.db.clone();
    let filter2 = filter_owned.clone();
    let search2 = search_owned.clone();
    let total =
        tokio::task::spawn_blocking(move || db.count_jobs(filter2.as_deref(), search2.as_deref()))
            .await
            .unwrap()?;

    let db = state.db.clone();
    let jobs = tokio::task::spawn_blocking(move || {
        db.list_jobs(filter_owned.as_deref(), search_owned.as_deref(), per_page, offset)
    })
    .await
    .unwrap()?;

    let db2 = state.db.clone();
    let responses: Vec<JobResponse> = tokio::task::spawn_blocking(move || {
        jobs.into_iter()
            .map(|job| build_job_response(job, &db2))
            .collect()
    })
    .await
    .unwrap();

    let total_pages = if total == 0 { 1 } else { (total + per_page - 1) / per_page };

    Ok(Json(PaginatedResponse {
        data: responses,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn create_job(
    State(state): State<AppState>,
    Json(req): Json<CreateJobRequest>,
) -> Result<(axum::http::StatusCode, Json<JobResponse>), AppError> {
    // Validate cron expression
    if let ScheduleKind::Cron(ref expr) = req.schedule {
        CronSchedule::parse(&expr.0)?;
    }

    let depends_on = req.depends_on.unwrap_or_default();

    // Validate dependencies (no cycles)
    let job_id = Uuid::new_v4();
    if !depends_on.is_empty() {
        let dag = state.dag.clone();
        let deps = depends_on.clone();
        tokio::task::spawn_blocking(move || dag.validate_no_cycle(job_id, &deps))
            .await
            .unwrap()?;
    }

    let now = Utc::now();
    let job = Job {
        id: job_id,
        name: req.name,
        description: req.description,
        command: req.command,
        schedule: req.schedule,
        status: JobStatus::Enabled,
        timeout_secs: req.timeout_secs,
        depends_on,
        target: req.target,
        created_at: now,
        updated_at: now,
    };

    let db = state.db.clone();
    let job_clone = job.clone();
    tokio::task::spawn_blocking(move || db.insert_job(&job_clone))
        .await
        .unwrap()?;

    // Tell scheduler to reload
    let _ = state.scheduler_tx.send(SchedulerCommand::Reload).await;

    let db_log = state.db.clone();
    let job_name = job.name.clone();
    let job_id_log = job.id;
    let _ = tokio::task::spawn_blocking(move || {
        db_log.log_event("job.created", EventSeverity::Info, &format!("Job '{}' created", job_name), Some(job_id_log), None)
    }).await;

    let db2 = state.db.clone();
    let resp = tokio::task::spawn_blocking(move || build_job_response(job, &db2))
        .await
        .unwrap();
    Ok((axum::http::StatusCode::CREATED, Json(resp)))
}

async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<JobResponse>, AppError> {
    let db = state.db.clone();
    let job = tokio::task::spawn_blocking(move || db.get_job(id))
        .await
        .unwrap()?
        .ok_or_else(|| AppError::NotFound(format!("job {id} not found")))?;

    let db2 = state.db.clone();
    let resp = tokio::task::spawn_blocking(move || build_job_response(job, &db2))
        .await
        .unwrap();
    Ok(Json(resp))
}

async fn update_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateJobRequest>,
) -> Result<Json<JobResponse>, AppError> {
    let db = state.db.clone();
    let mut job = tokio::task::spawn_blocking(move || db.get_job(id))
        .await
        .unwrap()?
        .ok_or_else(|| AppError::NotFound(format!("job {id} not found")))?;

    if let Some(name) = req.name {
        job.name = name;
    }
    if let Some(desc) = req.description {
        job.description = Some(desc);
    }
    if let Some(cmd) = req.command {
        job.command = cmd;
    }
    if let Some(schedule) = req.schedule {
        if let ScheduleKind::Cron(ref expr) = schedule {
            CronSchedule::parse(&expr.0)?;
        }
        job.schedule = schedule;
    }
    if let Some(status) = req.status {
        job.status = status;
    }
    if let Some(timeout) = req.timeout_secs {
        job.timeout_secs = Some(timeout);
    }
    if let Some(deps) = req.depends_on {
        if !deps.is_empty() {
            let dag = state.dag.clone();
            let deps_clone = deps.clone();
            tokio::task::spawn_blocking(move || dag.validate_no_cycle(id, &deps_clone))
                .await
                .unwrap()?;
        }
        job.depends_on = deps;
    }
    if let Some(target) = req.target {
        job.target = Some(target);
    }

    job.updated_at = Utc::now();

    let db = state.db.clone();
    let job_clone = job.clone();
    tokio::task::spawn_blocking(move || db.update_job(&job_clone))
        .await
        .unwrap()?;

    let _ = state.scheduler_tx.send(SchedulerCommand::Reload).await;

    let db2 = state.db.clone();
    let resp = tokio::task::spawn_blocking(move || build_job_response(job, &db2))
        .await
        .unwrap();
    Ok(Json(resp))
}

async fn delete_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.delete_job(id))
        .await
        .unwrap()?;

    let _ = state.scheduler_tx.send(SchedulerCommand::Reload).await;
    let db_log = state.db.clone();
    let _ = tokio::task::spawn_blocking(move || {
        db_log.log_event("job.deleted", EventSeverity::Warning, &format!("Job deleted ({})", id), Some(id), None)
    }).await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn trigger_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<TriggerResponse>, AppError> {
    // Verify job exists
    let db = state.db.clone();
    let _ = tokio::task::spawn_blocking(move || db.get_job(id))
        .await
        .unwrap()?
        .ok_or_else(|| AppError::NotFound(format!("job {id} not found")))?;

    state
        .scheduler_tx
        .send(SchedulerCommand::TriggerNow(id))
        .await
        .map_err(|_| AppError::Internal("scheduler unavailable".into()))?;

    let db_log = state.db.clone();
    let _ = tokio::task::spawn_blocking(move || {
        db_log.log_event("job.triggered", EventSeverity::Info, &format!("Job manually triggered ({})", id), Some(id), None)
    }).await;

    Ok(Json(TriggerResponse {
        message: "job triggered".to_string(),
        job_id: id,
    }))
}

async fn list_executions(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
    Query(query): Query<ListExecsQuery>,
) -> Result<Json<PaginatedResponse<Vec<ExecutionRecord>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(query.limit.unwrap_or(20)).min(100);
    let offset = (page - 1) * per_page;

    let db = state.db.clone();
    let total = tokio::task::spawn_blocking(move || db.count_executions_for_job(job_id))
        .await
        .unwrap()?;

    let db = state.db.clone();
    let recs =
        tokio::task::spawn_blocking(move || db.list_executions_for_job(job_id, per_page, offset))
            .await
            .unwrap()?;

    let total_pages = if total == 0 { 1 } else { (total + per_page - 1) / per_page };

    Ok(Json(PaginatedResponse {
        data: recs,
        total,
        page,
        per_page,
        total_pages,
    }))
}

async fn get_execution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ExecutionRecord>, AppError> {
    let db = state.db.clone();
    let rec = tokio::task::spawn_blocking(move || db.get_execution(id))
        .await
        .unwrap()?
        .ok_or_else(|| AppError::NotFound(format!("execution {id} not found")))?;
    Ok(Json(rec))
}

async fn cancel_execution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .scheduler_tx
        .send(SchedulerCommand::CancelExecution(id))
        .await
        .map_err(|_| AppError::Internal("scheduler unavailable".into()))?;

    Ok(Json(serde_json::json!({"message": "cancel request sent", "execution_id": id})))
}

fn compute_next_fire(job: &Job) -> Option<chrono::DateTime<Utc>> {
    match &job.schedule {
        ScheduleKind::Cron(expr) => {
            let schedule = CronSchedule::parse(&expr.0).ok()?;
            schedule.next_after(Utc::now())
        }
        ScheduleKind::OneShot(t) => {
            if *t > Utc::now() {
                Some(*t)
            } else {
                None
            }
        }
        ScheduleKind::Manual => None,
    }
}

fn build_job_response(job: Job, db: &Db) -> JobResponse {
    let next = compute_next_fire(&job);
    let last_execution = db
        .get_latest_execution_for_job(job.id)
        .ok()
        .flatten()
        .map(|e| LastExecution {
            id: e.id,
            status: e.status,
            exit_code: e.exit_code,
            finished_at: e.finished_at,
        });
    let (total, succeeded, failed) = db
        .get_execution_counts(job.id)
        .unwrap_or((0, 0, 0));
    JobResponse {
        job,
        next_fire_time: next,
        last_execution,
        execution_counts: ExecutionCounts {
            total,
            succeeded,
            failed,
        },
    }
}

// --- Agent Handlers ---

async fn register_agent(
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

    let agent = Agent {
        id: agent_id,
        name: req.name,
        tags: req.tags,
        hostname: req.hostname,
        address: req.address,
        port: req.port,
        status: AgentStatus::Online,
        last_heartbeat: Some(now),
        registered_at: existing.map(|a| a.registered_at).unwrap_or(now),
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

async fn agent_heartbeat(
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

async fn list_agents(
    State(state): State<AppState>,
) -> Result<Json<Vec<Agent>>, AppError> {
    let db = state.db.clone();
    let agents = tokio::task::spawn_blocking(move || db.list_agents())
        .await
        .unwrap()?;
    Ok(Json(agents))
}

async fn get_agent_handler(
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

async fn deregister_agent(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.delete_agent(id))
        .await
        .unwrap()?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn execution_result_callback(
    State(state): State<AppState>,
    Json(result): Json<ExecutionResultReport>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.clone();
    let exec_id = result.execution_id;

    // Get existing execution to preserve triggered_by
    let db2 = db.clone();
    let existing = tokio::task::spawn_blocking(move || db2.get_execution(exec_id))
        .await
        .unwrap()?;

    let triggered_by = existing
        .map(|e| e.triggered_by)
        .unwrap_or(TriggerSource::Scheduler);

    let rec = ExecutionRecord {
        id: result.execution_id,
        job_id: result.job_id,
        agent_id: Some(result.agent_id),
        status: result.status,
        exit_code: result.exit_code,
        stdout: result.stdout,
        stderr: result.stderr,
        stdout_truncated: result.stdout_truncated,
        stderr_truncated: result.stderr_truncated,
        started_at: Some(result.started_at),
        finished_at: Some(result.finished_at),
        triggered_by,
    };

    let status = rec.status;
    tokio::task::spawn_blocking(move || db.update_execution(&rec))
        .await
        .unwrap()?;

    let severity = match status {
        ExecutionStatus::Succeeded => EventSeverity::Success,
        ExecutionStatus::Failed | ExecutionStatus::TimedOut => EventSeverity::Error,
        _ => EventSeverity::Info,
    };
    let db_log = state.db.clone();
    let exec_id = result.execution_id;
    let job_id = result.job_id;
    let agent_id = result.agent_id;
    let _ = tokio::task::spawn_blocking(move || {
        db_log.log_event(
            "execution.completed",
            severity,
            &format!("Execution {} finished: {:?}", exec_id, status),
            Some(job_id),
            Some(agent_id),
        )
    }).await;

    tracing::info!(
        "received execution result {} from agent {}: {:?}",
        result.execution_id,
        result.agent_id,
        status
    );

    Ok(Json(serde_json::json!({"status": "ok"})))
}

#[derive(Deserialize)]
struct ListEventsQuery {
    page: Option<u32>,
    per_page: Option<u32>,
}

async fn list_events(
    State(state): State<AppState>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<PaginatedResponse<Vec<crate::models::Event>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(100);
    let offset = (page - 1) * per_page;

    let db = state.db.clone();
    let total = tokio::task::spawn_blocking(move || db.count_events())
        .await
        .unwrap()?;

    let db = state.db.clone();
    let events = tokio::task::spawn_blocking(move || db.list_events(per_page, offset))
        .await
        .unwrap()?;

    let total_pages = if total == 0 { 1 } else { (total + per_page - 1) / per_page };

    Ok(Json(PaginatedResponse {
        data: events,
        total,
        page,
        per_page,
        total_pages,
    }))
}
