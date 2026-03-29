use axum::Json;
use axum::extract::{Query, Request, State};
use serde::Deserialize;

use super::{AppState, PaginatedResponse};
use crate::db::audit::AuditEntry;
use crate::db::db_call;
use crate::error::AppError;

use super::auth::require_admin;

#[derive(Deserialize)]
pub(crate) struct ListAuditQuery {
    page: Option<u32>,
    per_page: Option<u32>,
    operation: Option<String>,
    actor: Option<String>,
    since: Option<String>,
}

pub(crate) async fn list_audit_log(
    State(state): State<AppState>,
    Query(query): Query<ListAuditQuery>,
    req: Request,
) -> Result<Json<PaginatedResponse<Vec<AuditEntry>>>, AppError> {
    require_admin(&req)?;

    let page = query.page.unwrap_or(1).max(1);
    let per_page = query.per_page.unwrap_or(50).min(100);
    let offset = (page - 1) * per_page;

    let op = query.operation.clone();
    let actor = query.actor.clone();
    let since = query.since.clone();

    let op2 = op.clone();
    let actor2 = actor.clone();
    let since2 = since.clone();

    let total = db_call(&state.db, move |db| {
        db.count_audit_log(op2.as_deref(), actor2.as_deref(), since2.as_deref())
    })
    .await?;

    let entries = db_call(&state.db, move |db| {
        db.list_audit_log(
            op.as_deref(),
            actor.as_deref(),
            since.as_deref(),
            per_page,
            offset,
        )
    })
    .await?;

    let total_pages = if total == 0 {
        1
    } else {
        total.div_ceil(per_page)
    };

    Ok(Json(PaginatedResponse {
        data: entries,
        total,
        page,
        per_page,
        total_pages,
    }))
}
