use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use uuid::Uuid;

use super::{AppState, PaginatedResponse};
use crate::error::AppError;
use crate::models::*;
use crate::scheduler::SchedulerCommand;

#[derive(Deserialize)]
pub(crate) struct ListExecsQuery {
    limit: Option<u32>,
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Deserialize)]
pub(crate) struct ListAllExecsQuery {
    status: Option<String>,
    search: Option<String>,
    since: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

pub(crate) async fn list_executions(
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

    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(per_page)
    };

    Ok(Json(PaginatedResponse {
        data: recs,
        total,
        page,
        per_page,
        total_pages,
    }))
}

pub(crate) async fn list_all_executions(
    State(state): State<AppState>,
    Query(query): Query<ListAllExecsQuery>,
) -> Result<Json<PaginatedResponse<Vec<ExecutionRecord>>>, AppError> {
    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(20).min(100);
    let offset = (page - 1) * per_page;
    let status = query.status.clone();
    let search = query.search.clone();
    let since = query.since.clone();

    let db = state.db.clone();
    let s2 = status.clone();
    let q2 = search.clone();
    let t2 = since.clone();
    let total = tokio::task::spawn_blocking(move || {
        db.count_all_executions(s2.as_deref(), q2.as_deref(), t2.as_deref())
    })
    .await
    .unwrap()?;

    let db = state.db.clone();
    let recs = tokio::task::spawn_blocking(move || {
        db.list_all_executions(
            status.as_deref(),
            search.as_deref(),
            since.as_deref(),
            per_page,
            offset,
        )
    })
    .await
    .unwrap()?;

    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(per_page)
    };

    Ok(Json(PaginatedResponse {
        data: recs,
        total,
        page,
        per_page,
        total_pages,
    }))
}

pub(crate) async fn get_execution(
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

pub(crate) async fn cancel_execution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .scheduler_tx
        .send(SchedulerCommand::CancelExecution(id))
        .await
        .map_err(|_| AppError::Internal("scheduler unavailable".into()))?;

    Ok(Json(
        serde_json::json!({"message": "cancel request sent", "execution_id": id}),
    ))
}
