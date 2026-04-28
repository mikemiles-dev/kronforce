use axum::Json;
use axum::extract::{Path, Query, State};
use serde::Deserialize;
use uuid::Uuid;

use super::{AppState, PaginatedResponse, paginate, paginated_response};
use crate::db::db_call;
use crate::db::models::*;
use crate::error::AppError;
use crate::scheduler::SchedulerCommand;

/// Query parameters for listing executions of a specific job.
#[derive(Deserialize)]
pub(crate) struct ListExecsQuery {
    limit: Option<u32>,
    page: Option<u32>,
    per_page: Option<u32>,
}

/// Query parameters for listing all executions across jobs.
#[derive(Deserialize)]
pub(crate) struct ListAllExecsQuery {
    status: Option<String>,
    search: Option<String>,
    since: Option<String>,
    group: Option<String>,
    page: Option<u32>,
    per_page: Option<u32>,
}

/// Returns a paginated list of executions for a specific job.
pub(crate) async fn list_executions(
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
    Query(query): Query<ListExecsQuery>,
) -> Result<Json<PaginatedResponse<Vec<ExecutionRecord>>>, AppError> {
    let (page, per_page, offset) = paginate(query.page, query.per_page.or(query.limit));

    let total = db_call(&state.db, move |db| db.count_executions_for_job(job_id)).await?;

    let recs = db_call(&state.db, move |db| {
        db.list_executions_for_job(job_id, per_page, offset)
    })
    .await?;

    Ok(Json(paginated_response(recs, total, page, per_page)))
}

/// Returns a paginated list of all executions with optional status, search, and time filters.
pub(crate) async fn list_all_executions(
    State(state): State<AppState>,
    Query(query): Query<ListAllExecsQuery>,
) -> Result<Json<PaginatedResponse<Vec<ExecutionRecord>>>, AppError> {
    let (page, per_page, offset) = paginate(query.page, query.per_page);
    let status = query.status.clone();
    let search = query.search.clone();
    let since = query.since.clone();
    let group = query.group.clone();

    let s2 = status.clone();
    let q2 = search.clone();
    let t2 = since.clone();
    let g2 = group.clone();
    let total = db_call(&state.db, move |db| {
        db.count_all_executions(s2.as_deref(), q2.as_deref(), t2.as_deref(), g2.as_deref())
    })
    .await?;

    let recs = db_call(&state.db, move |db| {
        db.list_all_executions(
            status.as_deref(),
            search.as_deref(),
            since.as_deref(),
            group.as_deref(),
            per_page,
            offset,
        )
    })
    .await?;

    Ok(Json(paginated_response(recs, total, page, per_page)))
}

/// Returns a single execution by ID.
pub(crate) async fn get_execution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ExecutionRecord>, AppError> {
    let rec = db_call(&state.db, move |db| db.get_execution(id))
        .await?
        .ok_or_else(|| AppError::NotFound(format!("execution {id} not found")))?;
    Ok(Json(rec))
}

/// Sends a cancel request for a running execution to the scheduler.
pub(crate) async fn cancel_execution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    auth: super::auth::AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(ref key) = auth.0
        && !key.role.can_write()
    {
        return Err(AppError::Forbidden(
            "write access required (admin or operator role)".into(),
        ));
    }
    state
        .scheduler_tx
        .send(SchedulerCommand::CancelExecution(id))
        .await
        .map_err(|_| AppError::Internal("scheduler unavailable".into()))?;

    Ok(Json(
        serde_json::json!({"message": "cancel request sent", "execution_id": id}),
    ))
}

/// Returns the last N execution statuses per job, for DAG visualization.
/// Response: `{ "job_id": [{"status":"succeeded","started_at":"..."},  ...], ... }`
pub(crate) async fn recent_statuses(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.clone();
    let rows = db_call(&db, move |db| db.recent_execution_statuses(10)).await?;

    let mut map: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();
    for (job_id, status, started_at) in rows {
        let arr = map.entry(job_id).or_insert_with(|| serde_json::json!([]));
        if let Some(a) = arr.as_array_mut() {
            a.push(serde_json::json!({ "status": status, "started_at": started_at }));
        }
    }
    Ok(Json(serde_json::Value::Object(map)))
}

/// SSE endpoint for live output streaming during execution.
pub(crate) async fn stream_execution(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<
    axum::response::sse::Sse<
        impl futures_core::Stream<Item = Result<axum::response::sse::Event, std::convert::Infallible>>,
    >,
    AppError,
> {
    use tokio_stream::StreamExt;
    use tokio_stream::wrappers::BroadcastStream;

    let tx = state
        .live_output
        .get(&id)
        .ok_or_else(|| AppError::NotFound("no live stream for this execution".into()))?;
    let rx = tx.subscribe();
    drop(tx);

    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(line) => {
            if line == "[done]" {
                Some(Ok(axum::response::sse::Event::default()
                    .event("done")
                    .data("")))
            } else {
                Some(Ok(axum::response::sse::Event::default().data(line)))
            }
        }
        Err(_) => None,
    });

    Ok(axum::response::sse::Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("ping"),
    ))
}
