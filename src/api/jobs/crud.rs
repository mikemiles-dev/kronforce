//! CRUD handlers for jobs: list, create, get, update, delete.

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::Utc;
use uuid::Uuid;

use super::{
    AppState, AuthUser, CreateJobRequest, CronSchedule, DepStatus, ExecutionCounts, JobResponse,
    LastExecution, ListJobsQuery, MAX_CRON_EXPR_LEN, UpdateJobRequest,
};
use super::{
    log_and_notify, normalize_group, paginate, paginated_response, persist_group, validate_job_name,
};
use crate::db::db_call;
use crate::db::models::*;
use crate::error::AppError;
use crate::scheduler::SchedulerCommand;

use super::{Db, PaginatedResponse};

/// Returns a paginated list of jobs with optional status and search filters.
pub(crate) async fn list_jobs(
    State(state): State<AppState>,
    Query(query): Query<ListJobsQuery>,
) -> Result<Json<PaginatedResponse<Vec<JobResponse>>>, AppError> {
    let (page, per_page, offset) = paginate(query.page, query.per_page);
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

    Ok(Json(paginated_response(responses, total, page, per_page)))
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

    // Validate file push size limit: 5MB binary ~ 6.67MB base64; limit at 6_700_000 base64 bytes
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
        timezone: req.timezone,
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

    // Optimistic concurrency check
    if let Some(expected) = req.if_unmodified_since {
        let diff = (job.updated_at - expected)
            .num_milliseconds()
            .unsigned_abs();
        if diff > 1000 {
            return Err(AppError::Conflict(
                "job was modified by another user — reload and try again".into(),
            ));
        }
    }

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
                .map_err(|e| AppError::Internal(e.to_string()))??;
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
    if req.timezone.is_some() {
        job.timezone = req.timezone;
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

impl JobResponse {
    /// Builds an enriched job response from a job and database.
    pub(crate) fn from_job(job: Job, db: &Db) -> Self {
        let next = Self::compute_next_fire(&job, db);
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

    fn compute_next_fire(job: &Job, db: &Db) -> Option<chrono::DateTime<Utc>> {
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
            ScheduleKind::Calendar(cal) => super::triggers::compute_next_calendar_fire(cal, now),
            ScheduleKind::Interval { interval_secs } => {
                // Next fire = last execution finish + interval
                let last = db
                    .get_latest_execution_for_job(job.id)
                    .ok()
                    .flatten()
                    .and_then(|e| e.finished_at);
                match last {
                    Some(finished) => {
                        Some(finished + chrono::Duration::seconds(*interval_secs as i64))
                    }
                    None => Some(now), // Never run, fire now
                }
            }
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
