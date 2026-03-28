use axum::Json;
use axum::extract::State;
use serde::Serialize;
use std::collections::HashMap;

use super::AppState;
use crate::db::db_call;
use crate::error::AppError;

#[derive(Serialize)]
pub(crate) struct ChartStats {
    execution_outcomes: HashMap<String, u32>,
    task_types: HashMap<String, u32>,
    schedule_types: HashMap<String, u32>,
}

pub(crate) async fn chart_stats(
    State(state): State<AppState>,
) -> Result<Json<ChartStats>, AppError> {
    let db1 = state.db.clone();
    let db2 = state.db.clone();
    let db3 = state.db.clone();

    let (outcomes, task_types, schedule_types) = tokio::try_join!(
        db_call(&db1, |db| db.get_execution_outcome_counts()),
        db_call(&db2, |db| db.get_task_type_counts()),
        db_call(&db3, |db| db.get_schedule_type_counts()),
    )?;

    Ok(Json(ChartStats {
        execution_outcomes: outcomes,
        task_types,
        schedule_types,
    }))
}
