use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::*;

// Columns: id(0), name(1), description(2), task_json(3), run_as(4), schedule_json(5), status(6),
//          timeout_secs(7), depends_on_json(8), target_json(9), created_by(10), created_at(11), updated_at(12), output_rules_json(13)
pub(super) fn row_to_job(row: &rusqlite::Row) -> Job {
    let id_str: String = row.get(0).unwrap();
    let run_as: Option<String> = row.get(4).unwrap();
    let schedule_json: String = row.get(5).unwrap();
    let status_str: String = row.get(6).unwrap();
    let timeout: Option<i64> = row.get(7).unwrap();
    let depends_json: String = row.get(8).unwrap();
    let target_json: Option<String> = row.get(9).unwrap();
    let created_by_str: Option<String> = row.get(10).unwrap();
    let created_str: String = row.get(11).unwrap();
    let updated_str: String = row.get(12).unwrap();

    let task_json: String = row.get(3).unwrap();

    Job {
        id: Uuid::parse_str(&id_str).unwrap(),
        name: row.get(1).unwrap(),
        description: row.get(2).unwrap(),
        task: serde_json::from_str(&task_json).unwrap(),
        run_as,
        schedule: serde_json::from_str(&schedule_json).unwrap(),
        status: JobStatus::from_str(&status_str).unwrap(),
        timeout_secs: timeout.map(|t| t as u64),
        depends_on: serde_json::from_str(&depends_json).unwrap_or_default(),
        target: target_json.and_then(|s| serde_json::from_str(&s).ok()),
        created_by: created_by_str.and_then(|s| Uuid::parse_str(&s).ok()),
        created_at: DateTime::parse_from_rfc3339(&created_str)
            .unwrap()
            .with_timezone(&Utc),
        updated_at: DateTime::parse_from_rfc3339(&updated_str)
            .unwrap()
            .with_timezone(&Utc),
        output_rules: {
            let or_json: Option<String> = row.get(13).unwrap_or(None);
            or_json.and_then(|s| serde_json::from_str(&s).ok())
        },
        notifications: {
            let n_json: Option<String> = row.get(14).unwrap_or(None);
            n_json.and_then(|s| serde_json::from_str(&s).ok())
        },
    }
}

// Columns: id(0), job_id(1), agent_id(2), task_snapshot_json(3), status(4), exit_code(5),
//          stdout(6), stderr(7), stdout_truncated(8), stderr_truncated(9), started_at(10), finished_at(11), triggered_by_json(12), extracted_json(13)
pub(super) fn row_to_execution(row: &rusqlite::Row) -> ExecutionRecord {
    let id_str: String = row.get(0).unwrap();
    let job_id_str: String = row.get(1).unwrap();
    let agent_id_str: Option<String> = row.get(2).unwrap();
    let task_snap_json: Option<String> = row.get(3).unwrap();
    let status_str: String = row.get(4).unwrap();
    let stdout_trunc: i32 = row.get(8).unwrap();
    let stderr_trunc: i32 = row.get(9).unwrap();
    let started_str: Option<String> = row.get(10).unwrap();
    let finished_str: Option<String> = row.get(11).unwrap();
    let triggered_json: String = row.get(12).unwrap();

    ExecutionRecord {
        id: Uuid::parse_str(&id_str).unwrap(),
        job_id: Uuid::parse_str(&job_id_str).unwrap(),
        agent_id: agent_id_str.and_then(|s| Uuid::parse_str(&s).ok()),
        task_snapshot: task_snap_json.and_then(|s| serde_json::from_str(&s).ok()),
        status: ExecutionStatus::from_str(&status_str).unwrap(),
        exit_code: row.get(5).unwrap(),
        stdout: row.get(6).unwrap(),
        stderr: row.get(7).unwrap(),
        stdout_truncated: stdout_trunc != 0,
        stderr_truncated: stderr_trunc != 0,
        started_at: started_str.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        }),
        finished_at: finished_str.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        }),
        triggered_by: serde_json::from_str(&triggered_json).unwrap(),
        extracted: {
            let ex_json: Option<String> = row.get(13).unwrap_or(None);
            ex_json.and_then(|s| serde_json::from_str(&s).ok())
        },
    }
}

pub(super) fn row_to_agent(row: &rusqlite::Row) -> Agent {
    let id_str: String = row.get(0).unwrap();
    let tags_json: String = row.get(2).unwrap();
    let agent_type_str: String = row.get(6).unwrap_or_else(|_| "standard".to_string());
    let status_str: String = row.get(7).unwrap();
    let hb_str: Option<String> = row.get(8).unwrap();
    let reg_str: String = row.get(9).unwrap();
    let task_types_str: Option<String> = row.get(10).unwrap_or(None);

    Agent {
        id: Uuid::parse_str(&id_str).unwrap(),
        name: row.get(1).unwrap(),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        hostname: row.get(3).unwrap(),
        address: row.get(4).unwrap(),
        port: {
            let p: i64 = row.get(5).unwrap();
            p as u16
        },
        agent_type: AgentType::from_str(&agent_type_str),
        status: AgentStatus::from_str(&status_str).unwrap_or(AgentStatus::Offline),
        last_heartbeat: hb_str.map(|s| {
            DateTime::parse_from_rfc3339(&s)
                .unwrap()
                .with_timezone(&Utc)
        }),
        registered_at: DateTime::parse_from_rfc3339(&reg_str)
            .unwrap()
            .with_timezone(&Utc),
        task_types: task_types_str
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default(),
    }
}

pub(super) fn row_to_api_key(row: &rusqlite::Row) -> ApiKey {
    let id_str: String = row.get(0).unwrap();
    let role_str: String = row.get(4).unwrap();
    let created_str: String = row.get(5).unwrap();
    let last_used_str: Option<String> = row.get(6).unwrap();
    let active_int: i32 = row.get(7).unwrap();

    ApiKey {
        id: Uuid::parse_str(&id_str).unwrap(),
        key_prefix: row.get(1).unwrap(),
        key_hash: row.get(2).unwrap(),
        name: row.get(3).unwrap(),
        role: ApiKeyRole::from_str(&role_str).unwrap_or(ApiKeyRole::Viewer),
        created_at: DateTime::parse_from_rfc3339(&created_str).unwrap().with_timezone(&Utc),
        last_used_at: last_used_str.map(|s| DateTime::parse_from_rfc3339(&s).unwrap().with_timezone(&Utc)),
        active: active_int != 0,
    }
}
