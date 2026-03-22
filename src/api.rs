use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::response::Html;
use axum::{Json, Router};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use uuid::Uuid;

use crate::cron_parser::CronSchedule;
use crate::dag::DagResolver;
use crate::db::Db;
use crate::error::AppError;
use crate::models::*;
use crate::scheduler::SchedulerCommand;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub dag: DagResolver,
    pub scheduler_tx: mpsc::Sender<SchedulerCommand>,
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
    depends_on: Option<Vec<Uuid>>,
}

#[derive(Deserialize)]
struct UpdateJobRequest {
    name: Option<String>,
    description: Option<String>,
    command: Option<String>,
    schedule: Option<ScheduleKind>,
    status: Option<JobStatus>,
    timeout_secs: Option<u64>,
    depends_on: Option<Vec<Uuid>>,
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

#[derive(Deserialize)]
struct ListJobsQuery {
    status: Option<String>,
}

#[derive(Deserialize)]
struct ListExecsQuery {
    limit: Option<u32>,
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
) -> Result<Json<Vec<JobResponse>>, AppError> {
    let filter = query.status.as_deref();
    let db = state.db.clone();
    let filter_owned = filter.map(|s| s.to_string());
    let jobs = tokio::task::spawn_blocking(move || db.list_jobs(filter_owned.as_deref()))
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

    Ok(Json(responses))
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
        status: JobStatus::Active,
        timeout_secs: req.timeout_secs,
        depends_on,
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

    Ok(Json(TriggerResponse {
        message: "job triggered".to_string(),
        job_id: id,
    }))
}

async fn list_executions(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
    Query(query): Query<ListExecsQuery>,
) -> Result<Json<Vec<ExecutionRecord>>, AppError> {
    let limit = query.limit.unwrap_or(20);
    let db = state.db.clone();
    let recs = tokio::task::spawn_blocking(move || db.list_executions_for_job(job_id, limit))
        .await
        .unwrap()?;
    Ok(Json(recs))
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
