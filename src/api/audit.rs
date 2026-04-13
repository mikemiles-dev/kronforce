use axum::Json;
use axum::extract::{Query, Request, State};
use serde::Deserialize;

use super::{AppState, PaginatedResponse, paginate, paginated_response};
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

    let (page, per_page, offset) = paginate(query.page, query.per_page.or(Some(50)));

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

    Ok(Json(paginated_response(entries, total, page, per_page)))
}
