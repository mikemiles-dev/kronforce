use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AppState, PaginatedResponse};
use crate::db::db_call;
use crate::error::AppError;
use crate::db::models::Event;

/// Query parameters for paginated event listing.
#[derive(Deserialize)]
pub(crate) struct ListEventsQuery {
    page: Option<u32>,
    per_page: Option<u32>,
    since: Option<String>,
}

/// Query parameters for timeline endpoints.
#[derive(Deserialize)]
pub(crate) struct TimelineQuery {
    minutes: Option<u32>,
}

/// A single time bucket in the execution timeline with success/failure/other counts.
#[derive(Serialize)]
pub(crate) struct TimelineBucket {
    time: String,
    succeeded: u32,
    failed: u32,
    other: u32,
}

/// Per-job breakdown within a single timeline bucket.
#[derive(Serialize)]
pub(crate) struct TimelineDetailEntry {
    job_name: String,
    status: String,
    count: u32,
}

/// Returns a paginated list of system events.
pub(crate) async fn list_events(
    State(state): State<AppState>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<PaginatedResponse<Vec<Event>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(100);
    let offset = (page - 1) * per_page;
    let since = query.since.clone();

    let t2 = since.clone();
    let total = db_call(&state.db, move |db| db.count_events(t2.as_deref())).await?;

    let events = db_call(&state.db, move |db| {
        db.list_events(since.as_deref(), per_page, offset)
    })
    .await?;

    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(per_page)
    };

    Ok(Json(PaginatedResponse {
        data: events,
        total,
        page,
        per_page,
        total_pages,
    }))
}

/// Returns execution counts per minute for the global timeline.
pub(crate) async fn get_timeline(
    State(state): State<AppState>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineBucket>>, AppError> {
    let minutes = query.minutes.unwrap_or(15);
    let data = db_call(&state.db, move |db| {
        db.get_execution_timeline(None, minutes)
    })
    .await?;
    Ok(Json(
        data.into_iter()
            .map(|(t, s, f, o)| TimelineBucket {
                time: t,
                succeeded: s,
                failed: f,
                other: o,
            })
            .collect(),
    ))
}

/// Returns execution counts per minute for a specific job.
pub(crate) async fn get_job_timeline(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineBucket>>, AppError> {
    let minutes = query.minutes.unwrap_or(60);
    let data = db_call(&state.db, move |db| {
        db.get_execution_timeline(Some(job_id), minutes)
    })
    .await?;
    Ok(Json(
        data.into_iter()
            .map(|(t, s, f, o)| TimelineBucket {
                time: t,
                succeeded: s,
                failed: f,
                other: o,
            })
            .collect(),
    ))
}

/// Returns per-job execution details for a specific minute bucket.
pub(crate) async fn get_timeline_detail(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
) -> Result<Json<Vec<TimelineDetailEntry>>, AppError> {
    let data = db_call(&state.db, move |db| db.get_timeline_detail(&bucket)).await?;
    Ok(Json(
        data.into_iter()
            .map(|(name, status, count)| TimelineDetailEntry {
                job_name: name,
                status,
                count,
            })
            .collect(),
    ))
}
