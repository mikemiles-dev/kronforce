use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::auth::AuthUser;
use super::{AppState, PaginatedResponse, log_and_notify};
use crate::db::models::*;
use crate::db::{Db, db_call};
use crate::error::AppError;
use crate::scheduler::SchedulerCommand;
use crate::scheduler::cron_parser::CronSchedule;

/// Request body for creating a new job.
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
    group: Option<String>,
    retry_max: Option<u32>,
    retry_delay_secs: Option<u64>,
    retry_backoff: Option<f64>,
    #[serde(default)]
    approval_required: bool,
    #[serde(default)]
    priority: i32,
    sla_deadline: Option<String>,
    #[serde(default)]
    sla_warning_mins: u32,
    starts_at: Option<chrono::DateTime<Utc>>,
    expires_at: Option<chrono::DateTime<Utc>>,
    max_concurrent: Option<u32>,
    parameters: Option<Vec<JobParameter>>,
}

/// Request body for updating an existing job. All fields are optional (partial update).
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
    group: Option<String>,
    retry_max: Option<u32>,
    retry_delay_secs: Option<u64>,
    retry_backoff: Option<f64>,
    approval_required: Option<bool>,
    priority: Option<i32>,
    sla_deadline: Option<String>,
    sla_warning_mins: Option<u32>,
    starts_at: Option<chrono::DateTime<Utc>>,
    expires_at: Option<chrono::DateTime<Utc>>,
    max_concurrent: Option<u32>,
    parameters: Option<Vec<JobParameter>>,
}

/// Summary of a job's most recent execution.
#[derive(Serialize)]
pub(crate) struct LastExecution {
    id: uuid::Uuid,
    status: ExecutionStatus,
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

/// Enriched job response with next fire time, execution stats, and dependency status.
#[derive(Serialize)]
pub(crate) struct JobResponse {
    #[serde(flatten)]
    job: Job,
    next_fire_time: Option<chrono::DateTime<Utc>>,
    last_execution: Option<LastExecution>,
    execution_counts: ExecutionCounts,
    deps_satisfied: bool,
    deps_status: Vec<DepStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    webhook_url: Option<String>,
}

/// Query parameters for paginated job listing.
#[derive(Deserialize)]
pub(crate) struct ListJobsQuery {
    status: Option<String>,
    search: Option<String>,
    group: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

/// Response returned when a job is manually triggered.
#[derive(Serialize)]
pub(crate) struct TriggerResponse {
    message: String,
    job_id: Uuid,
}

/// Returns a paginated list of jobs with optional status and search filters.
pub(crate) async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<ListJobsQuery>,
) -> Result<Json<PaginatedResponse<Vec<JobResponse>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;
    let filter_owned = query.status.clone();
    let search_owned = query.search.clone();
    let group_owned = query.group.clone();

    let filter2 = filter_owned.clone();
    let search2 = search_owned.clone();
    let group2 = group_owned.clone();
    let total = db_call(&state.db, move |db| {
        db.count_jobs(filter2.as_deref(), search2.as_deref(), group2.as_deref())
    })
    .await?;

    let jobs = db_call(&state.db, move |db| {
        db.list_jobs(
            filter_owned.as_deref(),
            search_owned.as_deref(),
            group_owned.as_deref(),
            per_page,
            offset,
        )
    })
    .await?;

    let responses: Vec<JobResponse> = db_call(&state.db, move |db| {
        Ok(jobs
            .into_iter()
            .map(|job| JobResponse::from_job(job, db))
            .collect())
    })
    .await?;

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

/// Maximum allowed length for group names.
const MAX_GROUP_NAME_LEN: usize = 50;

/// Default group name for jobs that don't specify one.
const DEFAULT_GROUP_NAME: &str = "Default";

/// Persists a group name to custom_groups so it survives job deletion.
async fn persist_group(db: &crate::db::Db, group: &Option<String>) {
    if let Some(g) = group
        && g != DEFAULT_GROUP_NAME
    {
        let db = db.clone();
        let g = g.clone();
        let _ = db_call(&db, move |db| db.add_custom_group(&g)).await;
    }
}

/// Normalizes and validates a group name. Empty/None becomes "Default".
fn normalize_group(group: Option<String>) -> Result<Option<String>, AppError> {
    match group {
        None => Ok(Some(DEFAULT_GROUP_NAME.to_string())),
        Some(g) if g.trim().is_empty() => Ok(Some(DEFAULT_GROUP_NAME.to_string())),
        Some(g) => {
            let g = g.trim().to_string();
            if g.len() > MAX_GROUP_NAME_LEN {
                return Err(AppError::BadRequest(format!(
                    "group name exceeds {} character limit",
                    MAX_GROUP_NAME_LEN
                )));
            }
            if !g
                .chars()
                .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
            {
                return Err(AppError::BadRequest(
                    "group name may only contain alphanumeric characters, spaces, hyphens, and underscores".into(),
                ));
            }
            Ok(Some(g))
        }
    }
}

/// Maximum allowed length for job names.
const MAX_JOB_NAME_LEN: usize = 255;
/// Maximum allowed length for cron expressions.
const MAX_CRON_EXPR_LEN: usize = 200;

fn validate_job_name(name: &str) -> Result<(), AppError> {
    if name.is_empty() {
        return Err(AppError::BadRequest("job name cannot be empty".into()));
    }
    if name.len() > MAX_JOB_NAME_LEN {
        return Err(AppError::BadRequest(format!(
            "job name exceeds {} character limit",
            MAX_JOB_NAME_LEN
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' || c == '.')
    {
        return Err(AppError::BadRequest(
            "job name may only contain alphanumeric characters, spaces, hyphens, underscores, and dots".into(),
        ));
    }
    Ok(())
}

/// Creates a new job, validates its cron expression and dependencies, and notifies the scheduler.
pub(crate) async fn create_job(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateJobRequest>,
) -> Result<(axum::http::StatusCode, Json<JobResponse>), AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    validate_job_name(&req.name)?;

    // Validate cron expression
    if let ScheduleKind::Cron(ref expr) = req.schedule {
        if expr.0.len() > MAX_CRON_EXPR_LEN {
            return Err(AppError::BadRequest("cron expression too long".into()));
        }
        CronSchedule::parse(&expr.0)?;
    }

    // Validate file push size limit: 5MB binary ≈ 6.67MB base64; limit at 6_700_000 base64 bytes
    if let TaskType::FilePush {
        ref content_base64, ..
    } = req.task
        && content_base64.len() > 6_700_000
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
            .map_err(|e| AppError::Internal(e.to_string()))??;
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
        created_by: auth.0.as_ref().map(|k| k.id),
        created_at: now,
        updated_at: now,
        output_rules: req.output_rules,
        notifications: req.notifications,
        group: normalize_group(req.group)?,
        retry_max: req.retry_max.unwrap_or(0),
        retry_delay_secs: req.retry_delay_secs.unwrap_or(0),
        retry_backoff: req.retry_backoff.unwrap_or(1.0),
        approval_required: req.approval_required,
        priority: req.priority,
        sla_deadline: req.sla_deadline,
        sla_warning_mins: req.sla_warning_mins,
        starts_at: req.starts_at,
        expires_at: req.expires_at,
        max_concurrent: req.max_concurrent.unwrap_or(0),
        parameters: req.parameters,
        webhook_token: None,
    };

    let job_clone = job.clone();
    db_call(&state.db, move |db| db.insert_job(&job_clone)).await?;
    persist_group(&state.db, &job.group).await;

    // Tell scheduler to reload
    let _ = state.scheduler_tx.send(SchedulerCommand::Reload).await;

    // Save initial version
    let version_job = job.clone();
    let version_actor_id = auth.0.as_ref().map(|k| k.id);
    let version_actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let db_ver = state.db.clone();
    let _ = db_call(&db_ver, move |db| {
        db.save_job_version(
            &version_job,
            version_actor_id,
            version_actor_name.as_deref(),
        )
    })
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

    let audit_job_id = job.id.to_string();
    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "job.created",
            "job",
            Some(&audit_job_id),
            actor_id,
            actor_name.as_deref(),
            None,
        )
    })
    .await;

    let resp = db_call(&state.db, move |db| Ok(JobResponse::from_job(job, db))).await?;
    Ok((axum::http::StatusCode::CREATED, Json(resp)))
}

/// Returns a single job by ID with enriched response data.
pub(crate) async fn get_job_handler(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<JobResponse>, AppError> {
    let job = db_call(&state.db, move |db| db.get_job(id))
        .await?
        .ok_or_else(|| AppError::NotFound(format!("job {id} not found")))?;

    let resp = db_call(&state.db, move |db| Ok(JobResponse::from_job(job, db))).await?;
    Ok(Json(resp))
}

/// Updates a job with partial fields and logs an audit trail of the changes.
pub(crate) async fn update_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
    Json(req): Json<UpdateJobRequest>,
) -> Result<Json<JobResponse>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    let mut job = db_call(&state.db, move |db| db.get_job(id))
        .await?
        .ok_or_else(|| AppError::NotFound(format!("job {id} not found")))?;

    // Snapshot before changes for audit diff
    let old_task = serde_json::to_string(&job.task).unwrap_or_default();
    let old_schedule = serde_json::to_string(&job.schedule).unwrap_or_default();
    let old_status = job.status.as_str().to_string();
    let old_run_as = job.run_as.clone();

    if let Some(name) = req.name {
        validate_job_name(&name)?;
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
            if expr.0.len() > MAX_CRON_EXPR_LEN {
                return Err(AppError::BadRequest("cron expression too long".into()));
            }
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
    if req.group.is_some() {
        job.group = normalize_group(req.group)?;
        persist_group(&state.db, &job.group).await;
    }
    if let Some(rm) = req.retry_max {
        job.retry_max = rm;
    }
    if let Some(rd) = req.retry_delay_secs {
        job.retry_delay_secs = rd;
    }
    if let Some(rb) = req.retry_backoff {
        job.retry_backoff = rb;
    }
    if let Some(ar) = req.approval_required {
        job.approval_required = ar;
    }
    if let Some(p) = req.priority {
        job.priority = p;
    }
    if req.sla_deadline.is_some() {
        job.sla_deadline = req.sla_deadline;
    }
    if let Some(w) = req.sla_warning_mins {
        job.sla_warning_mins = w;
    }
    if req.starts_at.is_some() {
        job.starts_at = req.starts_at;
    }
    if req.expires_at.is_some() {
        job.expires_at = req.expires_at;
    }
    if let Some(mc) = req.max_concurrent {
        job.max_concurrent = mc;
    }
    if req.parameters.is_some() {
        job.parameters = req.parameters;
    }

    job.updated_at = Utc::now();

    let job_clone = job.clone();
    db_call(&state.db, move |db| db.update_job(&job_clone)).await?;

    // Save version snapshot
    let version_job = job.clone();
    let version_actor_id = auth.0.as_ref().map(|k| k.id);
    let version_actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let db_ver = state.db.clone();
    let _ = db_call(&db_ver, move |db| {
        db.save_job_version(
            &version_job,
            version_actor_id,
            version_actor_name.as_deref(),
        )
    })
    .await;

    let _ = state.scheduler_tx.send(SchedulerCommand::Reload).await;

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
        details.clone(),
    )
    .await;

    let audit_job_id = job.id.to_string();
    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "job.updated",
            "job",
            Some(&audit_job_id),
            actor_id,
            actor_name.as_deref(),
            details.as_deref(),
        )
    })
    .await;

    let resp = db_call(&state.db, move |db| Ok(JobResponse::from_job(job, db))).await?;
    Ok(Json(resp))
}

/// Deletes a job and notifies the scheduler to reload.
pub(crate) async fn delete_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    db_call(&state.db, move |db| db.delete_job(id)).await?;

    let _ = state.scheduler_tx.send(SchedulerCommand::Reload).await;
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

    let audit_job_id = id.to_string();
    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "job.deleted",
            "job",
            Some(&audit_job_id),
            actor_id,
            actor_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Query parameters for the trigger endpoint.
#[derive(Deserialize)]
pub(crate) struct TriggerQuery {
    /// When true, skip dependency checks for this single execution.
    skip_deps: Option<bool>,
}

/// Optional JSON body for trigger requests (params, etc.).
#[derive(Deserialize, Default)]
pub(crate) struct TriggerBody {
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// Manually triggers a job execution outside of its schedule.
/// If the job has `approval_required`, creates a pending_approval execution instead.
pub(crate) async fn trigger_job(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Query(trigger_query): Query<TriggerQuery>,
    auth: AuthUser,
    body: Option<Json<TriggerBody>>,
) -> Result<(axum::http::StatusCode, Json<TriggerResponse>), AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    let job = db_call(&state.db, move |db| db.get_job(id))
        .await?
        .ok_or_else(|| AppError::NotFound(format!("job {id} not found")))?;

    let trigger_params = body.and_then(|b| b.0.params);

    // Validate params is an object if provided
    if let Some(ref p) = trigger_params
        && !p.is_object()
    {
        return Err(AppError::BadRequest("params must be a JSON object".into()));
    }

    if job.approval_required {
        // Create a pending_approval execution instead of running immediately
        let exec_id = Uuid::new_v4();
        let rec = ExecutionRecord {
            id: exec_id,
            job_id: id,
            agent_id: None,
            task_snapshot: Some(job.task.clone()),
            status: ExecutionStatus::PendingApproval,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            started_at: None,
            finished_at: None,
            triggered_by: TriggerSource::Api,
            extracted: None,
            retry_of: None,
            attempt_number: 1,
            params: trigger_params.clone(),
        };
        let rec_clone = rec.clone();
        db_call(&state.db, move |db| db.insert_execution(&rec_clone)).await?;

        log_and_notify(
            &state.db,
            &state.scheduler_tx,
            "job.pending_approval",
            EventSeverity::Warning,
            &format!(
                "Job '{}' awaiting approval (execution {})",
                job.name, exec_id
            ),
            Some(id),
            None,
            &auth,
            None,
        )
        .await;

        return Ok((
            axum::http::StatusCode::ACCEPTED,
            Json(TriggerResponse {
                message: "job awaiting approval".to_string(),
                job_id: id,
            }),
        ));
    }

    let skip_deps = trigger_query.skip_deps.unwrap_or(false);

    state
        .scheduler_tx
        .send(SchedulerCommand::TriggerNow {
            job_id: id,
            skip_deps,
            params: trigger_params,
        })
        .await
        .map_err(|_| AppError::Internal("scheduler unavailable".into()))?;
    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "job.triggered",
        EventSeverity::Info,
        &format!(
            "Job manually triggered ({}){}",
            id,
            if skip_deps {
                " (dependencies skipped)"
            } else {
                ""
            }
        ),
        Some(id),
        None,
        &auth,
        None,
    )
    .await;

    let audit_job_id = id.to_string();
    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "job.triggered",
            "job",
            Some(&audit_job_id),
            actor_id,
            actor_name.as_deref(),
            None,
        )
    })
    .await;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(TriggerResponse {
            message: "job triggered".to_string(),
            job_id: id,
        }),
    ))
}

impl JobResponse {
    /// Builds an enriched job response from a job and database.
    pub(crate) fn from_job(job: Job, db: &Db) -> Self {
        let next = Self::compute_next_fire(&job);
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
        let (deps_satisfied, deps_status) = Self::evaluate_deps(db, &job.depends_on);

        let webhook_url = job
            .webhook_token
            .as_ref()
            .map(|t| format!("/api/webhooks/{}", t));

        JobResponse {
            job,
            next_fire_time: next,
            last_execution,
            execution_counts: ExecutionCounts {
                total,
                succeeded,
                failed,
            },
            deps_satisfied,
            deps_status,
            webhook_url,
        }
    }

    fn compute_next_fire(job: &Job) -> Option<chrono::DateTime<Utc>> {
        let now = Utc::now();

        // If the job has expired, no next fire
        if let Some(expires_at) = job.expires_at
            && now > expires_at
        {
            return None;
        }

        let next = match &job.schedule {
            ScheduleKind::Cron(expr) => {
                let schedule = CronSchedule::parse(&expr.0).ok()?;
                schedule.next_after(now)
            }
            ScheduleKind::OneShot(t) => {
                if *t > now {
                    Some(*t)
                } else {
                    None
                }
            }
            ScheduleKind::Calendar(cal) => compute_next_calendar_fire(cal, now),
            ScheduleKind::OnDemand | ScheduleKind::Event(_) => None,
        };

        // Clamp to starts_at if the next fire is before the window opens
        let next = match (next, job.starts_at) {
            (Some(t), Some(starts_at)) if t < starts_at => Some(starts_at),
            _ => next,
        };

        // Return None if next fire is after expiry
        match (next, job.expires_at) {
            (Some(t), Some(expires_at)) if t > expires_at => None,
            _ => next,
        }
    }

    fn evaluate_deps(db: &Db, deps: &[Dependency]) -> (bool, Vec<DepStatus>) {
        let now = chrono::Utc::now();
        let mut all_satisfied = true;
        let statuses: Vec<DepStatus> = deps
            .iter()
            .map(|dep| {
                let dep_name = db.get_job(dep.job_id).ok().flatten().map(|j| j.name);
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
        (all_satisfied, statuses)
    }
}

/// Returns the list of distinct group names across all jobs.
pub(crate) async fn list_groups(
    State(state): State<AppState>,
) -> Result<Json<Vec<String>>, AppError> {
    let groups = db_call(&state.db, |db| db.get_distinct_groups()).await?;
    Ok(Json(groups))
}

/// Request body for creating a new empty group.
#[derive(Deserialize)]
pub(crate) struct CreateGroupRequest {
    name: String,
}

/// Creates a new empty group (persisted in settings).
pub(crate) async fn create_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateGroupRequest>,
) -> Result<(axum::http::StatusCode, Json<serde_json::Value>), AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    let name = normalize_group(Some(req.name))?.unwrap_or_else(|| DEFAULT_GROUP_NAME.to_string());
    let name_clone = name.clone();
    db_call(&state.db, move |db| db.add_custom_group(&name_clone)).await?;
    Ok((
        axum::http::StatusCode::CREATED,
        Json(serde_json::json!({"name": name})),
    ))
}

/// Request body for bulk group assignment.
#[derive(Deserialize)]
pub(crate) struct BulkGroupRequest {
    job_ids: Vec<Uuid>,
    group: Option<String>,
}

/// Assigns a group to multiple jobs at once.
pub(crate) async fn bulk_set_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<BulkGroupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    let group = normalize_group(req.group)?;
    persist_group(&state.db, &group).await;
    let ids = req.job_ids;
    let count = db_call(&state.db, move |db| {
        db.bulk_set_group(&ids, group.as_deref())
    })
    .await?;
    Ok(Json(serde_json::json!({"updated": count})))
}

/// Request body for renaming a group.
#[derive(Deserialize)]
pub(crate) struct RenameGroupRequest {
    old_name: String,
    new_name: String,
}

/// Renames all jobs from one group to another.
pub(crate) async fn rename_group(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RenameGroupRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    let new_name =
        normalize_group(Some(req.new_name))?.unwrap_or_else(|| DEFAULT_GROUP_NAME.to_string());
    let old_name = req.old_name;
    let old_clone = old_name.clone();
    let new_clone = new_name.clone();
    let count = db_call(&state.db, move |db| db.rename_group(&old_clone, &new_clone)).await?;

    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let details = format!("renamed '{}' to '{}'", old_name, new_name);
    let _ = db_call(&state.db, move |db| {
        db.record_audit(
            "group.renamed",
            "group",
            None,
            actor_id,
            actor_name.as_deref(),
            Some(&details),
        )
    })
    .await;

    Ok(Json(serde_json::json!({"updated": count})))
}

/// Approves a pending_approval execution, allowing it to run.
pub(crate) async fn approve_execution(
    State(state): State<AppState>,
    Path(exec_id): Path<Uuid>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required to approve executions".into(),
        ));
    }

    let exec = db_call(&state.db, move |db| db.get_execution(exec_id))
        .await?
        .ok_or_else(|| AppError::NotFound(format!("execution {exec_id} not found")))?;

    if exec.status != ExecutionStatus::PendingApproval {
        return Err(AppError::BadRequest(format!(
            "execution is {:?}, not pending_approval",
            exec.status
        )));
    }

    // Trigger the job through the scheduler, preserving original params
    state
        .scheduler_tx
        .send(SchedulerCommand::TriggerNow {
            job_id: exec.job_id,
            skip_deps: false,
            params: exec.params.clone(),
        })
        .await
        .map_err(|_| AppError::Internal("scheduler unavailable".into()))?;

    // Mark the pending_approval execution as superseded (cancelled)
    let db2 = state.db.clone();
    let _ = db_call(&db2, move |db| {
        db.update_execution_status(exec_id, ExecutionStatus::Cancelled)
    })
    .await;

    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "execution.approved",
        EventSeverity::Info,
        &format!("Execution {} approved", exec_id),
        Some(exec.job_id),
        None,
        &auth,
        None,
    )
    .await;

    let actor_id = auth.0.as_ref().map(|k| k.id);
    let actor_name = auth.0.as_ref().map(|k| k.name.clone());
    let db_audit = state.db.clone();
    let eid = exec_id.to_string();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "execution.approved",
            "execution",
            Some(&eid),
            actor_id,
            actor_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(Json(serde_json::json!({
        "message": "execution approved",
        "execution_id": exec_id,
    })))
}

/// Returns version history for a job, newest first.
pub(crate) async fn list_job_versions(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let versions = db_call(&state.db, move |db| db.list_job_versions(id)).await?;
    Ok(Json(versions))
}

// --- Webhook Triggers ---

/// Triggers a job via its webhook token (no auth required).
pub(crate) async fn webhook_trigger(
    State(state): State<AppState>,
    Path(token): Path<String>,
    body: Option<Json<TriggerBody>>,
) -> Result<(axum::http::StatusCode, Json<TriggerResponse>), AppError> {
    let token_clone = token.clone();
    let job = db_call(&state.db, move |db| {
        db.get_job_by_webhook_token(&token_clone)
    })
    .await?
    .ok_or_else(|| AppError::NotFound("invalid webhook token".into()))?;

    let params = body.and_then(|b| b.0.params);

    state
        .scheduler_tx
        .send(SchedulerCommand::TriggerNow {
            job_id: job.id,
            skip_deps: false,
            params,
        })
        .await
        .map_err(|_| AppError::Internal("scheduler unavailable".into()))?;

    let token_prefix = if token.len() >= 8 {
        token[..8].to_string()
    } else {
        token.clone()
    };

    log_and_notify(
        &state.db,
        &state.scheduler_tx,
        "job.triggered",
        EventSeverity::Info,
        &format!(
            "Job '{}' triggered via webhook ({}...)",
            job.name, token_prefix
        ),
        Some(job.id),
        None,
        &super::auth::AuthUser(None),
        None,
    )
    .await;

    Ok((
        axum::http::StatusCode::ACCEPTED,
        Json(TriggerResponse {
            message: "webhook triggered".to_string(),
            job_id: job.id,
        }),
    ))
}

/// Generates a webhook token for a job.
pub(crate) async fn generate_webhook(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    // Verify job exists
    let job_id = id;
    db_call(&state.db, move |db| db.get_job(job_id))
        .await?
        .ok_or_else(|| AppError::NotFound(format!("job {id} not found")))?;

    let bytes: [u8; 16] = rand::random();
    let token: String = bytes.iter().map(|b| format!("{:02x}", b)).collect();
    let token_clone = token.clone();
    db_call(&state.db, move |db| {
        db.set_webhook_token(job_id, Some(&token_clone))
    })
    .await?;

    Ok(Json(serde_json::json!({
        "token": token,
        "webhook_url": format!("/api/webhooks/{}", token),
    })))
}

/// Removes the webhook token from a job.
pub(crate) async fn delete_webhook(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: AuthUser,
) -> Result<axum::http::StatusCode, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    db_call(&state.db, move |db| db.set_webhook_token(id, None)).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}

/// Compute the next fire date for a calendar schedule by scanning forward up to 13 months.
fn compute_next_calendar_fire(
    cal: &CalendarSchedule,
    now: chrono::DateTime<Utc>,
) -> Option<chrono::DateTime<Utc>> {
    use chrono::{Datelike, NaiveDate, TimeZone};

    for month_offset in 0..=13i32 {
        let total_months = now.year() * 12 + now.month() as i32 - 1 + month_offset;
        let year = total_months / 12;
        let month = (total_months % 12 + 1) as u32;

        if !cal.months.is_empty() && !cal.months.contains(&month) {
            continue;
        }

        let anchor = if cal.anchor == "last_day" {
            if month == 12 {
                NaiveDate::from_ymd_opt(year + 1, 1, 1)
            } else {
                NaiveDate::from_ymd_opt(year, month + 1, 1)
            }
            .and_then(|d| d.pred_opt())
        } else if cal.anchor.starts_with("day_") {
            let day: u32 = cal.anchor[4..].parse().unwrap_or(1);
            NaiveDate::from_ymd_opt(year, month, day)
        } else if cal.anchor == "nth_weekday" {
            let nth = cal.nth.unwrap_or(1);
            let wd = crate::scheduler::parse_weekday(cal.weekday.as_deref().unwrap_or("monday"));
            crate::scheduler::nth_weekday_of_month(year, month, wd, nth)
        } else if cal.anchor.starts_with("first_") {
            let wd = crate::scheduler::parse_weekday(&cal.anchor[6..]);
            crate::scheduler::nth_weekday_of_month(year, month, wd, 1)
        } else if cal.anchor.starts_with("last_") && cal.anchor != "last_day" {
            let wd = crate::scheduler::parse_weekday(&cal.anchor[5..]);
            crate::scheduler::last_weekday_of_month(year, month, wd)
        } else {
            None
        };

        let Some(anchor_date) = anchor else {
            continue;
        };

        let target = anchor_date + chrono::Duration::days(cal.offset_days as i64);
        let fire_dt = target.and_hms_opt(cal.hour, cal.minute, 0)?;
        let fire_utc = Utc.from_utc_datetime(&fire_dt);

        if fire_utc > now {
            return Some(fire_utc);
        }
    }
    None
}
