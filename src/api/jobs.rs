use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth::AuthUser;
use super::{AppState, PaginatedResponse, log_and_notify};
use crate::cron_parser::CronSchedule;
use crate::db::Db;
use crate::error::AppError;
use crate::models::*;

#[derive(Deserialize)]
pub(crate) struct CreateJobRequest {
    name: String,
    description: Option<String>,
    task: TaskType,
    run_as: Option<String>,
    schedule: ScheduleKind,
    timeout_secs: Option<u64>,
    depends_on: Option<Vec<Dependency>>,
    target: Option<AgentTarget>,
    output_rules: Option<OutputRules>,
    notifications: Option<JobNotificationConfig>,
}

#[derive(Deserialize)]
pub(crate) struct UpdateJobRequest {
    name: Option<String>,
    description: Option<String>,
    task: Option<TaskType>,
    run_as: Option<String>,
    schedule: Option<ScheduleKind>,
    status: Option<JobStatus>,
    timeout_secs: Option<u64>,
    depends_on: Option<Vec<Dependency>>,
    target: Option<AgentTarget>,
    output_rules: Option<OutputRules>,
    notifications: Option<JobNotificationConfig>,
}

#[derive(Serialize)]
pub(crate) struct LastExecution {
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
pub(crate) struct JobResponse {
    #[serde(flatten)]
    job: Job,
    next_fire_time: Option<chrono::DateTime<Utc>>,
    last_execution: Option<LastExecution>,
    execution_counts: ExecutionCounts,
    deps_satisfied: bool,
    deps_status: Vec<DepStatus>,
}

#[derive(Deserialize)]
pub(crate) struct ListJobsQuery {
    status: Option<String>,
    search: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Serialize)]
pub(crate) struct TriggerResponse {
    message: String,
    job_id: Uuid,
}

pub(crate) async fn list_jobs(
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
        db.list_jobs(
            filter_owned.as_deref(),
            search_owned.as_deref(),
            per_page,
            offset,
        )
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

    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(per_page)
    };

    Ok(Json(PaginatedResponse {
        data: responses,
        total,
        page,
        per_page,
        total_pages,
    }))
}

pub(crate) async fn create_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateJobRequest>,
) -> Result<(axum::http::StatusCode, Json<JobResponse>), AppError> {
    // Validate cron expression
    if let ScheduleKind::Cron(ref expr) = req.schedule {
        CronSchedule::parse(&expr.0)?;
    }

    // Validate file push size limit (5MB = ~6.7MB base64)
    if let TaskType::FilePush {
        ref content_base64, ..
    } = req.task
        && content_base64.len() > 7_000_000
    {
        return Err(AppError::BadRequest("file exceeds 5MB limit".to_string()));
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
        output_rules: req.output_rules,
        notifications: req.notifications,
    };

    let db = state.db.clone();
    let job_clone = job.clone();
    tokio::task::spawn_blocking(move || db.insert_job(&job_clone))
        .await
        .unwrap()?;

    // Tell scheduler to reload
    let _ = state
        .scheduler_tx
        .send(crate::scheduler::SchedulerCommand::Reload)
        .await;

    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "job.created",
        EventSeverity::Info,
        &format!("Job '{}' created", job.name),
        Some(job.id),
        None,
        &auth,
        None,
    )
    .await;

    let db2 = state.db.clone();
    let resp = tokio::task::spawn_blocking(move || build_job_response(job, &db2))
        .await
        .unwrap();
    Ok((axum::http::StatusCode::CREATED, Json(resp)))
}

pub(crate) async fn get_job_handler(
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

pub(crate) async fn update_job(
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
    if req.output_rules.is_some() {
        job.output_rules = req.output_rules;
    }
    if req.notifications.is_some() {
        job.notifications = req.notifications;
    }

    job.updated_at = Utc::now();

    let db = state.db.clone();
    let job_clone = job.clone();
    tokio::task::spawn_blocking(move || db.update_job(&job_clone))
        .await
        .unwrap()?;

    let _ = state
        .scheduler_tx
        .send(crate::scheduler::SchedulerCommand::Reload)
        .await;

    // Build audit diff
    let mut changes = Vec::new();
    let new_task = serde_json::to_string(&job.task).unwrap_or_default();
    let new_schedule = serde_json::to_string(&job.schedule).unwrap_or_default();
    if old_task != new_task {
        changes.push(format!("task: {} -> {}", old_task, new_task));
    }
    if old_schedule != new_schedule {
        changes.push(format!("schedule: {} -> {}", old_schedule, new_schedule));
    }
    if old_status != job.status.as_str() {
        changes.push(format!("status: {} -> {}", old_status, job.status.as_str()));
    }
    if old_run_as != job.run_as {
        changes.push(format!("run_as: {:?} -> {:?}", old_run_as, job.run_as));
    }
    let details = if changes.is_empty() {
        None
    } else {
        Some(changes.join("; "))
    };

    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "job.updated",
        EventSeverity::Info,
        &format!("Job '{}' updated", job.name),
        Some(job.id),
        None,
        &auth,
        details,
    )
    .await;

    let db2 = state.db.clone();
    let resp = tokio::task::spawn_blocking(move || build_job_response(job, &db2))
        .await
        .unwrap();
    Ok(Json(resp))
}

pub(crate) async fn delete_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, AppError> {
    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.delete_job(id))
        .await
        .unwrap()?;

    let _ = state
        .scheduler_tx
        .send(crate::scheduler::SchedulerCommand::Reload)
        .await;
    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "job.deleted",
        EventSeverity::Warning,
        &format!("Job deleted ({})", id),
        Some(id),
        None,
        &auth,
        None,
    )
    .await;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

pub(crate) async fn trigger_job(
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
        .send(crate::scheduler::SchedulerCommand::TriggerNow(id))
        .await
        .map_err(|_| AppError::Internal("scheduler unavailable".into()))?;

    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "job.triggered",
        EventSeverity::Info,
        &format!("Job manually triggered ({})", id),
        Some(id),
        None,
        &auth,
        None,
    )
    .await;

    Ok(Json(TriggerResponse {
        message: "job triggered".to_string(),
        job_id: id,
    }))
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

pub(crate) fn build_job_response(job: Job, db: &Db) -> JobResponse {
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
    let (total, succeeded, failed) = db.get_execution_counts(job.id).unwrap_or((0, 0, 0));

    // Check dependency status
    let now = chrono::Utc::now();
    let mut all_satisfied = true;
    let deps_status: Vec<DepStatus> = job
        .depends_on
        .iter()
        .map(|dep| {
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
            if !satisfied {
                all_satisfied = false;
            }
            DepStatus {
                job_id: dep.job_id,
                job_name: dep_name,
                within_secs: dep.within_secs,
                satisfied,
            }
        })
        .collect();

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
