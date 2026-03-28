use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::*;

fn parse_uuid(s: &str) -> rusqlite::Result<Uuid> {
    Uuid::parse_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

fn parse_datetime(s: &str) -> rusqlite::Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
        })
}

fn parse_json<T: serde::de::DeserializeOwned>(s: &str) -> rusqlite::Result<T> {
    serde_json::from_str(s).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e))
    })
}

// Columns: id(0), name(1), description(2), task_json(3), run_as(4), schedule_json(5), status(6),
//          timeout_secs(7), depends_on_json(8), target_json(9), created_by(10), created_at(11), updated_at(12), output_rules_json(13), notifications_json(14)
pub(super) fn row_to_job(row: &rusqlite::Row) -> rusqlite::Result<Job> {
    let id_str: String = row.get(0)?;
    let run_as: Option<String> = row.get(4)?;
    let schedule_json: String = row.get(5)?;
    let status_str: String = row.get(6)?;
    let timeout: Option<i64> = row.get(7)?;
    let depends_json: String = row.get(8)?;
    let target_json: Option<String> = row.get(9)?;
    let created_by_str: Option<String> = row.get(10)?;
    let created_str: String = row.get(11)?;
    let updated_str: String = row.get(12)?;
    let task_json: String = row.get(3)?;

    Ok(Job {
        id: parse_uuid(&id_str)?,
        name: row.get(1)?,
        description: row.get(2)?,
        task: parse_json(&task_json)?,
        run_as,
        schedule: parse_json(&schedule_json)?,
        status: JobStatus::from_str(&status_str).unwrap_or(JobStatus::Unscheduled),
        timeout_secs: timeout.map(|t| t as u64),
        depends_on: serde_json::from_str(&depends_json).unwrap_or_default(),
        target: target_json.and_then(|s| serde_json::from_str(&s).ok()),
        created_by: created_by_str.and_then(|s| Uuid::parse_str(&s).ok()),
        created_at: parse_datetime(&created_str)?,
        updated_at: parse_datetime(&updated_str)?,
        output_rules: {
            let or_json: Option<String> = row.get(13).unwrap_or(None);
            or_json.and_then(|s| serde_json::from_str(&s).ok())
        },
        notifications: {
            let n_json: Option<String> = row.get(14).unwrap_or(None);
            n_json.and_then(|s| serde_json::from_str(&s).ok())
        },
    })
}

// Columns: id(0), job_id(1), agent_id(2), task_snapshot_json(3), status(4), exit_code(5),
//          stdout(6), stderr(7), stdout_truncated(8), stderr_truncated(9), started_at(10), finished_at(11), triggered_by_json(12), extracted_json(13)
pub(super) fn row_to_execution(row: &rusqlite::Row) -> rusqlite::Result<ExecutionRecord> {
    let id_str: String = row.get(0)?;
    let job_id_str: String = row.get(1)?;
    let agent_id_str: Option<String> = row.get(2)?;
    let task_snap_json: Option<String> = row.get(3)?;
    let status_str: String = row.get(4)?;
    let stdout_trunc: i32 = row.get(8)?;
    let stderr_trunc: i32 = row.get(9)?;
    let started_str: Option<String> = row.get(10)?;
    let finished_str: Option<String> = row.get(11)?;
    let triggered_json: String = row.get(12)?;

    Ok(ExecutionRecord {
        id: parse_uuid(&id_str)?,
        job_id: parse_uuid(&job_id_str)?,
        agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
        task_snapshot: task_snap_json.and_then(|s| serde_json::from_str(&s).ok()),
        status: ExecutionStatus::from_str(&status_str).unwrap_or(ExecutionStatus::Failed),
        exit_code: row.get(5)?,
        stdout: row.get(6)?,
        stderr: row.get(7)?,
        stdout_truncated: stdout_trunc != 0,
        stderr_truncated: stderr_trunc != 0,
        started_at: started_str.map(|s| parse_datetime(&s)).transpose()?,
        finished_at: finished_str.map(|s| parse_datetime(&s)).transpose()?,
        triggered_by: parse_json(&triggered_json)?,
        extracted: {
            let ex_json: Option<String> = row.get(13).unwrap_or(None);
            ex_json.and_then(|s| serde_json::from_str(&s).ok())
        },
    })
}

pub(super) fn row_to_agent(row: &rusqlite::Row) -> rusqlite::Result<Agent> {
    let id_str: String = row.get(0)?;
    let tags_json: String = row.get(2)?;
    let agent_type_str: String = row.get(6).unwrap_or_else(|_| "standard".to_string());
    let status_str: String = row.get(7)?;
    let hb_str: Option<String> = row.get(8)?;
    let reg_str: String = row.get(9)?;
    let task_types_str: Option<String> = row.get(10).unwrap_or(None);

    Ok(Agent {
        id: parse_uuid(&id_str)?,
        name: row.get(1)?,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        hostname: row.get(3)?,
        address: row.get(4)?,
        port: {
            let p: i64 = row.get(5)?;
            p as u16
        },
        agent_type: AgentType::from_str(&agent_type_str),
        status: AgentStatus::from_str(&status_str).unwrap_or(AgentStatus::Offline),
        last_heartbeat: hb_str.map(|s| parse_datetime(&s)).transpose()?,
        registered_at: parse_datetime(&reg_str)?,
        task_types: task_types_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
    })
}

pub(super) fn row_to_api_key(row: &rusqlite::Row) -> rusqlite::Result<ApiKey> {
    let id_str: String = row.get(0)?;
    let role_str: String = row.get(4)?;
    let created_str: String = row.get(5)?;
    let last_used_str: Option<String> = row.get(6)?;
    let active_int: i32 = row.get(7)?;

    Ok(ApiKey {
        id: parse_uuid(&id_str)?,
        key_prefix: row.get(1)?,
        key_hash: row.get(2)?,
        name: row.get(3)?,
        role: ApiKeyRole::from_str(&role_str).unwrap_or(ApiKeyRole::Viewer),
        created_at: parse_datetime(&created_str)?,
        last_used_at: last_used_str.map(|s| parse_datetime(&s)).transpose()?,
        active: active_int != 0,
    })
}
