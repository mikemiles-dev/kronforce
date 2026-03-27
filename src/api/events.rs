use axum::Json;
use axum::extract::{Path, Query, State};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{AppState, PaginatedResponse};
use crate::error::AppError;

#[derive(Deserialize)]
pub(crate) struct ListEventsQuery {
    page: Option<u32>,
    per_page: Option<u32>,
    since: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct TimelineQuery {
    minutes: Option<u32>,
}

#[derive(Serialize)]
pub(crate) struct TimelineBucket {
    time: String,
    succeeded: u32,
    failed: u32,
    other: u32,
}

#[derive(Serialize)]
pub(crate) struct TimelineDetailEntry {
    job_name: String,
    status: String,
    count: u32,
}

pub(crate) async fn list_events(
    State(state): State<AppState>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<PaginatedResponse<Vec<crate::models::Event>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(100);
    let offset = (page - 1) * per_page;
    let since = query.since.clone();

    let db = state.db.clone();
    let t2 = since.clone();
    let total = tokio::task::spawn_blocking(move || db.count_events(t2.as_deref()))
        .await
        .unwrap()?;

    let db = state.db.clone();
    let events =
        tokio::task::spawn_blocking(move || db.list_events(since.as_deref(), per_page, offset))
            .await
            .unwrap()?;

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

pub(crate) async fn get_timeline(
    State(state): State<AppState>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineBucket>>, AppError> {
    let minutes = query.minutes.unwrap_or(15);
    let db = state.db.clone();
    let data = tokio::task::spawn_blocking(move || db.get_execution_timeline(None, minutes))
        .await
        .unwrap()?;
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

pub(crate) async fn get_job_timeline(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
    Query(query): Query<TimelineQuery>,
) -> Result<Json<Vec<TimelineBucket>>, AppError> {
    let minutes = query.minutes.unwrap_or(60);
    let db = state.db.clone();
    let data =
        tokio::task::spawn_blocking(move || db.get_execution_timeline(Some(job_id), minutes))
            .await
            .unwrap()?;
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

pub(crate) async fn get_timeline_detail(
    State(state): State<AppState>,
    Path(bucket): Path<String>,
) -> Result<Json<Vec<TimelineDetailEntry>>, AppError> {
    let db = state.db.clone();
    let data = tokio::task::spawn_blocking(move || db.get_timeline_detail(&bucket))
        .await
        .unwrap()?;
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
