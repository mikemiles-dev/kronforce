//! Trigger-related handlers: manual trigger, approval, webhooks, and calendar fire computation.

use axum::Json;
use axum::extract::{Path, Query, State};
use chrono::Utc;
use uuid::Uuid;

use super::log_and_notify;
use super::{AppState, AuthUser, TriggerBody, TriggerQuery, TriggerResponse};
use crate::db::db_call;
use crate::db::models::*;
use crate::error::AppError;
use crate::scheduler::SchedulerCommand;

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
            Some(exec_id),
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
        Some(exec_id),
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
        None,
        &super::AuthUser(None),
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
pub(super) fn compute_next_calendar_fire(
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
