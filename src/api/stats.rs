use axum::Json;
use axum::extract::State;
use axum::response::IntoResponse;
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

/// Prometheus-compatible metrics endpoint. Returns text/plain in Prometheus exposition format.
pub(crate) async fn prometheus_metrics(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let db1 = state.db.clone();
    let db2 = state.db.clone();
    let db3 = state.db.clone();
    let db4 = state.db.clone();
    let db5 = state.db.clone();

    let (outcomes, task_types, job_count, agent_count, group_count) = tokio::try_join!(
        db_call(&db1, |db| db.get_execution_outcome_counts()),
        db_call(&db2, |db| db.get_task_type_counts()),
        db_call(&db3, |db| db.count_jobs(None, None, None)),
        db_call(&db4, |db| db.count_agents()),
        db_call(&db5, |db| db.count_groups()),
    )?;

    let mut lines = Vec::new();

    // Execution counts by status
    lines.push("# HELP kronforce_executions_total Total executions by status.".to_string());
    lines.push("# TYPE kronforce_executions_total counter".to_string());
    for (status, count) in &outcomes {
        lines.push(format!(
            "kronforce_executions_total{{status=\"{}\"}} {}",
            status, count
        ));
    }

    // Task type counts
    lines.push("# HELP kronforce_jobs_by_task_type Number of jobs by task type.".to_string());
    lines.push("# TYPE kronforce_jobs_by_task_type gauge".to_string());
    for (task_type, count) in &task_types {
        lines.push(format!(
            "kronforce_jobs_by_task_type{{type=\"{}\"}} {}",
            task_type, count
        ));
    }

    // Totals
    lines.push("# HELP kronforce_jobs_total Total number of jobs.".to_string());
    lines.push("# TYPE kronforce_jobs_total gauge".to_string());
    lines.push(format!("kronforce_jobs_total {}", job_count));

    lines.push("# HELP kronforce_agents_total Total number of registered agents.".to_string());
    lines.push("# TYPE kronforce_agents_total gauge".to_string());
    lines.push(format!("kronforce_agents_total {}", agent_count));

    lines.push("# HELP kronforce_groups_total Total number of job groups.".to_string());
    lines.push("# TYPE kronforce_groups_total gauge".to_string());
    lines.push(format!("kronforce_groups_total {}", group_count));

    // DB health
    let db_health = {
        let db = state.db.clone();
        tokio::task::spawn_blocking(move || db.health_check())
            .await
            .ok()
            .flatten()
    };
    if let Some(health) = db_health {
        lines.push("# HELP kronforce_db_ok Database is accessible (1=ok, 0=error).".to_string());
        lines.push("# TYPE kronforce_db_ok gauge".to_string());
        lines.push(format!("kronforce_db_ok {}", if health.ok { 1 } else { 0 }));

        if let Some(size) = health.size_bytes {
            lines.push(
                "# HELP kronforce_db_size_bytes Database file size in bytes.".to_string(),
            );
            lines.push("# TYPE kronforce_db_size_bytes gauge".to_string());
            lines.push(format!("kronforce_db_size_bytes {}", size));
        }
        if let Some(wal) = health.wal_size_bytes {
            lines.push(
                "# HELP kronforce_db_wal_size_bytes WAL file size in bytes.".to_string(),
            );
            lines.push("# TYPE kronforce_db_wal_size_bytes gauge".to_string());
            lines.push(format!("kronforce_db_wal_size_bytes {}", wal));
        }
    }

    lines.push(String::new()); // trailing newline

    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        lines.join("\n"),
    ))
}
