use axum::extract::{Path, Query, Request, State};
use axum::middleware::{self, Next};
use axum::response::Response;
use axum::routing::{get, post, delete};
use axum::response::Html;
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
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
    // Routes that require auth
    let authed = Router::new()
        .route("/api/jobs", get(list_jobs).post(create_job))
        .route(
            "/api/jobs/{id}",
            get(get_job).put(update_job).delete(delete_job),
        )
        .route("/api/jobs/{id}/trigger", post(trigger_job))
        .route("/api/jobs/{id}/executions", get(list_executions))
        .route("/api/executions/{id}", get(get_execution))
        .route("/api/executions/{id}/cancel", post(cancel_execution))
        .route("/api/agents", get(list_agents))
        .route("/api/agents/{id}", get(get_agent_handler).delete(deregister_agent))
        .route("/api/events", get(list_events))
        .route("/api/timeline", get(get_timeline))
        .route("/api/timeline/{job_id}", get(get_job_timeline))
        .route("/api/timeline-detail/{bucket}", get(get_timeline_detail))
        .route("/api/keys", get(list_api_keys).post(create_api_key))
        .route("/api/keys/{id}", delete(revoke_api_key))
        .route("/api/auth/me", get(auth_me))
        .route_layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .with_state(state.clone());

    // Routes exempt from auth
    let public = Router::new()
        .route("/", get(dashboard))
        .route("/api/health", get(health))
        .route("/api/agents/register", post(register_agent))
        .route("/api/agents/{id}/heartbeat", post(agent_heartbeat))
        .route("/api/callbacks/execution-result", post(execution_result_callback))
        .with_state(state);

    public.merge(authed)
}

// --- Request/Response types ---

#[derive(Deserialize)]
struct CreateJobRequest {
    name: String,
    description: Option<String>,
    task: TaskType,
    run_as: Option<String>,
    schedule: ScheduleKind,
    timeout_secs: Option<u64>,
    depends_on: Option<Vec<Dependency>>,
    target: Option<AgentTarget>,
}

#[derive(Deserialize)]
struct UpdateJobRequest {
    name: Option<String>,
    description: Option<String>,
    task: Option<TaskType>,
    run_as: Option<String>,
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
struct DepStatus {
    job_id: Uuid,
    job_name: Option<String>,
    within_secs: Option<u64>,
    satisfied: bool,
}

#[derive(Serialize)]
struct JobResponse {
    #[serde(flatten)]
    job: Job,
    next_fire_time: Option<chrono::DateTime<Utc>>,
    last_execution: Option<LastExecution>,
    execution_counts: ExecutionCounts,
    deps_satisfied: bool,
    deps_status: Vec<DepStatus>,
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
    auth: AuthUser,
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
        task: req.task,
        run_as: req.run_as,
        schedule: req.schedule,
        status: JobStatus::Scheduled,
        timeout_secs: req.timeout_secs,
        depends_on,
        target: req.target,
        created_by: None, // TODO: set from auth context
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

    log_and_notify(&state.db, &state.scheduler_tx, "job.created", EventSeverity::Info,
        &format!("Job '{}' created", job.name), Some(job.id), None, &auth, None).await;

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
    auth: AuthUser,
    Json(req): Json<UpdateJobRequest>,
) -> Result<Json<JobResponse>, AppError> {
    let db = state.db.clone();
    let mut job = tokio::task::spawn_blocking(move || db.get_job(id))
        .await
        .unwrap()?
        .ok_or_else(|| AppError::NotFound(format!("job {id} not found")))?;

    // Snapshot before changes for audit diff
    let old_task = serde_json::to_string(&job.task).unwrap_or_default();
    let old_schedule = serde_json::to_string(&job.schedule).unwrap_or_default();
    let old_status = job.status.as_str().to_string();
    let old_run_as = job.run_as.clone();

    if let Some(name) = req.name {
        job.name = name;
    }
    if let Some(desc) = req.description {
        job.description = Some(desc);
    }
    if let Some(task) = req.task {
        job.task = task;
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
    if req.run_as.is_some() {
        job.run_as = req.run_as;
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

    // Build audit diff
    let mut changes = Vec::new();
    let new_task = serde_json::to_string(&job.task).unwrap_or_default();
    let new_schedule = serde_json::to_string(&job.schedule).unwrap_or_default();
    if old_task != new_task { changes.push(format!("task: {} -> {}", old_task, new_task)); }
    if old_schedule != new_schedule { changes.push(format!("schedule: {} -> {}", old_schedule, new_schedule)); }
    if old_status != job.status.as_str() { changes.push(format!("status: {} -> {}", old_status, job.status.as_str())); }
    if old_run_as != job.run_as { changes.push(format!("run_as: {:?} -> {:?}", old_run_as, job.run_as)); }
    let details = if changes.is_empty() { None } else { Some(changes.join("; ")) };

    log_and_notify(&state.db, &state.scheduler_tx, "job.updated", EventSeverity::Info,
        &format!("Job '{}' updated", job.name), Some(job.id), None, &auth, details).await;

    let db2 = state.db.clone();
    let resp = tokio::task::spawn_blocking(move || build_job_response(job, &db2))
        .await
        .unwrap();
    Ok(Json(resp))
}

async fn delete_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, AppError> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.delete_job(id))
        .await
        .unwrap()?;

    let _ = state.scheduler_tx.send(SchedulerCommand::Reload).await;
    log_and_notify(&state.db, &state.scheduler_tx, "job.deleted", EventSeverity::Warning,
        &format!("Job deleted ({})", id), Some(id), None, &auth, None).await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

async fn trigger_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> Result<Json<TriggerResponse>, AppError> {
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

    log_and_notify(&state.db, &state.scheduler_tx, "job.triggered", EventSeverity::Info,
        &format!("Job manually triggered ({})", id), Some(id), None, &auth, None).await;

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
        ScheduleKind::OnDemand | ScheduleKind::Event(_) => None,
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

    // Check dependency status
    let now = chrono::Utc::now();
    let mut all_satisfied = true;
    let deps_status: Vec<DepStatus> = job.depends_on.iter().map(|dep| {
        let dep_job = db.get_job(dep.job_id).ok().flatten();
        let dep_name = dep_job.map(|j| j.name);
        let satisfied = match db.get_latest_execution_for_job(dep.job_id).ok().flatten() {
            Some(exec) if exec.status == ExecutionStatus::Succeeded => {
                if let Some(within) = dep.within_secs {
                    exec.finished_at
                        .map(|f| (now - f).num_seconds() <= within as i64)
                        .unwrap_or(false)
                } else {
                    true
                }
            }
            _ => false,
        };
        if !satisfied { all_satisfied = false; }
        DepStatus {
            job_id: dep.job_id,
            job_name: dep_name,
            within_secs: dep.within_secs,
            satisfied,
        }
    }).collect();

    JobResponse {
        job,
        next_fire_time: next,
        last_execution,
        execution_counts: ExecutionCounts {
            total,
            succeeded,
            failed,
        },
        deps_satisfied: all_satisfied,
        deps_status,
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

    let task_snap = existing.as_ref().and_then(|e| e.task_snapshot.clone());
    let triggered_by = existing
        .map(|e| e.triggered_by)
        .unwrap_or(TriggerSource::Scheduler);

    let rec = ExecutionRecord {
        id: result.execution_id,
        job_id: result.job_id,
        agent_id: Some(result.agent_id),
        task_snapshot: task_snap,
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
    // Log event and notify scheduler for event-triggered jobs
    let no_auth = AuthUser(None);
    let msg = format!("Execution {} finished: {:?}", result.execution_id, status);
    log_and_notify(&state.db, &state.scheduler_tx, "execution.completed", severity,
        &msg, Some(result.job_id), Some(result.agent_id), &no_auth, None).await;

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

// --- Timeline ---

#[derive(Deserialize)]
struct TimelineQuery {
    minutes: Option<u32>,
}

#[derive(Serialize)]
struct TimelineBucket {
    time: String,
    succeeded: u32,
    failed: u32,
    other: u32,
}

async fn get_timeline(
    State(state): State<AppState>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineBucket>>, AppError> {
    let minutes = query.minutes.unwrap_or(15);
    let db = state.db.clone();
    let data = tokio::task::spawn_blocking(move || db.get_execution_timeline(None, minutes))
        .await
        .unwrap()?;
    Ok(Json(data.into_iter().map(|(t, s, f, o)| TimelineBucket { time: t, succeeded: s, failed: f, other: o }).collect()))
}

async fn get_job_timeline(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineBucket>>, AppError> {
    let minutes = query.minutes.unwrap_or(60);
    let db = state.db.clone();
    let data = tokio::task::spawn_blocking(move || db.get_execution_timeline(Some(job_id), minutes))
        .await
        .unwrap()?;
    Ok(Json(data.into_iter().map(|(t, s, f, o)| TimelineBucket { time: t, succeeded: s, failed: f, other: o }).collect()))
}

#[derive(Serialize)]
struct TimelineDetailEntry {
    job_name: String,
    status: String,
    count: u32,
}

async fn get_timeline_detail(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
) -> Result<Json<Vec<TimelineDetailEntry>>, AppError> {
    let db = state.db.clone();
    let data = tokio::task::spawn_blocking(move || db.get_timeline_detail(&bucket))
        .await
        .unwrap()?;
    Ok(Json(data.into_iter().map(|(name, status, count)| TimelineDetailEntry { job_name: name, status, count }).collect()))
}

// --- Auth ---

pub fn hash_api_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn generate_api_key() -> (String, String) {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    let raw = format!("kf_{}", base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes));
    let prefix = raw[..11].to_string(); // "kf_" + first 8 chars of base64
    (raw, prefix)
}

async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Check if auth is enabled (any keys exist)
    let db = state.db.clone();
    let key_count = tokio::task::spawn_blocking(move || db.count_api_keys())
        .await
        .unwrap()?;

    // If no keys exist, skip auth (first-time setup)
    if key_count == 0 {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let raw_key = match auth_header {
        Some(ref h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err(AppError::Unauthorized("missing or invalid Authorization header".into()));
        }
    };

    let hash = hash_api_key(raw_key);
    let db = state.db.clone();
    let hash2 = hash.clone();
    let api_key = tokio::task::spawn_blocking(move || db.get_api_key_by_hash(&hash2))
        .await
        .unwrap()?;

    match api_key {
        Some(key) => {
            // Update last_used_at
            let db = state.db.clone();
            let key_id = key.id;
            let now = Utc::now();
            let _ = tokio::task::spawn_blocking(move || db.update_api_key_last_used(key_id, now)).await;

            req.extensions_mut().insert(key);
            Ok(next.run(req).await)
        }
        None => Err(AppError::Unauthorized("invalid API key".into())),
    }
}

/// Extractor for the authenticated API key. Returns None if auth is disabled.
#[derive(Clone)]
struct AuthUser(Option<ApiKey>);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for AuthUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(AuthUser(parts.extensions.get::<ApiKey>().cloned()))
    }
}

impl AuthUser {
    fn log_audit(&self, db: &Db, kind: &str, message: &str, job_id: Option<Uuid>, agent_id: Option<Uuid>, details: Option<String>) {
        if let Some(ref key) = self.0 {
            let _ = db.log_audit(kind, message, job_id, agent_id, key, details);
        } else {
            let _ = db.log_event(kind, EventSeverity::Info, message, job_id, agent_id);
        }
    }
}

/// Log an event and notify the scheduler for event-triggered jobs
async fn log_and_notify(
    db: &Db,
    scheduler_tx: &mpsc::Sender<SchedulerCommand>,
    kind: &str,
    severity: EventSeverity,
    message: &str,
    job_id: Option<Uuid>,
    agent_id: Option<Uuid>,
    auth: &AuthUser,
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

fn require_write(req: &Request) -> Result<(), AppError> {
    if let Some(key) = req.extensions().get::<ApiKey>() {
        if key.role.can_write() {
            Ok(())
        } else {
            Err(AppError::Forbidden("viewer role cannot modify resources".into()))
        }
    } else {
        Ok(()) // no auth context = no keys configured, allow
    }
}

fn require_admin(req: &Request) -> Result<(), AppError> {
    if let Some(key) = req.extensions().get::<ApiKey>() {
        if key.role.can_manage_keys() {
            Ok(())
        } else {
            Err(AppError::Forbidden("admin role required".into()))
        }
    } else {
        Ok(())
    }
}

async fn auth_me(req: Request) -> Json<serde_json::Value> {
    if let Some(key) = req.extensions().get::<ApiKey>() {
        Json(serde_json::json!({
            "authenticated": true,
            "key_id": key.id,
            "key_prefix": key.key_prefix,
            "name": key.name,
            "role": key.role,
        }))
    } else {
        Json(serde_json::json!({
            "authenticated": false,
            "message": "no API keys configured, auth disabled",
        }))
    }
}

#[derive(Deserialize)]
struct CreateApiKeyRequest {
    name: String,
    role: ApiKeyRole,
}

#[derive(Serialize)]
struct CreateApiKeyResponse {
    key: ApiKey,
    raw_key: String,
}

async fn create_api_key(
    State(state): State<AppState>,
    req: Request,
) -> Result<Json<CreateApiKeyResponse>, AppError> {
    require_admin(&req)?;

    let bytes = axum::body::to_bytes(req.into_body(), 1024 * 64)
        .await
        .map_err(|e| AppError::BadRequest(format!("invalid body: {e}")))?;
    let body: CreateApiKeyRequest = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::BadRequest(format!("invalid JSON: {e}")))?;

    let (raw_key, prefix) = generate_api_key();
    let hash = hash_api_key(&raw_key);

    let key = ApiKey {
        id: Uuid::new_v4(),
        key_prefix: prefix,
        key_hash: hash,
        name: body.name,
        role: body.role,
        created_at: Utc::now(),
        last_used_at: None,
        active: true,
    };

    let db = state.db.clone();
    let key2 = key.clone();
    tokio::task::spawn_blocking(move || db.insert_api_key(&key2))
        .await
        .unwrap()?;

    let db_log = state.db.clone();
    let key_name = key.name.clone();
    let _ = tokio::task::spawn_blocking(move || {
        db_log.log_event("key.created", EventSeverity::Info, &format!("API key '{}' created", key_name), None, None)
    }).await;

    Ok(Json(CreateApiKeyResponse { key, raw_key }))
}

async fn list_api_keys(
    State(state): State<AppState>,
    req: Request,
) -> Result<Json<Vec<ApiKey>>, AppError> {
    require_admin(&req)?;

    let db = state.db.clone();
    let keys = tokio::task::spawn_blocking(move || db.list_api_keys())
        .await
        .unwrap()?;
    Ok(Json(keys))
}

async fn revoke_api_key(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    req: Request,
) -> Result<axum::http::StatusCode, AppError> {
    require_admin(&req)?;

    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.delete_api_key(id))
        .await
        .unwrap()?;

    let db_log = state.db.clone();
    let _ = tokio::task::spawn_blocking(move || {
        db_log.log_event("key.revoked", EventSeverity::Warning, &format!("API key {} revoked", id), None, None)
    }).await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
