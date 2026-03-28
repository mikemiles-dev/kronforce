use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::task::TaskType;

/// Lifecycle status of a single job execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    TimedOut,
    Cancelled,
    Skipped,
}

impl ExecutionStatus {
    /// Returns the string representation of this execution status.
    pub fn as_str(&self) -> &'static str {
        match self {
            ExecutionStatus::Pending => "pending",
            ExecutionStatus::Running => "running",
            ExecutionStatus::Succeeded => "succeeded",
            ExecutionStatus::Failed => "failed",
            ExecutionStatus::TimedOut => "timed_out",
            ExecutionStatus::Cancelled => "cancelled",
            ExecutionStatus::Skipped => "skipped",
        }
    }

    /// Parses a status string into an `ExecutionStatus`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "pending" => Some(ExecutionStatus::Pending),
            "running" => Some(ExecutionStatus::Running),
            "succeeded" => Some(ExecutionStatus::Succeeded),
            "failed" => Some(ExecutionStatus::Failed),
            "timed_out" => Some(ExecutionStatus::TimedOut),
            "cancelled" => Some(ExecutionStatus::Cancelled),
            "skipped" => Some(ExecutionStatus::Skipped),
            _ => None,
        }
    }
}

/// What initiated a job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerSource {
    Scheduler,
    Api,
    Dependency { parent_execution_id: Uuid },
    Event { event_id: Uuid },
}

/// Recorded result of a single job execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRecord {
    pub id: Uuid,
    pub job_id: Uuid,
    pub agent_id: Option<Uuid>,
    pub task_snapshot: Option<TaskType>,
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub triggered_by: TriggerSource,
    #[serde(default)]
    pub extracted: Option<serde_json::Value>,
}

impl ExecutionRecord {
    /// Creates a new pending execution record.
    pub fn new(id: Uuid, job_id: Uuid, trigger: TriggerSource) -> Self {
        Self {
            id,
            job_id,
            agent_id: None,
            task_snapshot: None,
            status: ExecutionStatus::Pending,
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            stdout_truncated: false,
            stderr_truncated: false,
            started_at: None,
            finished_at: None,
            triggered_by: trigger,
            extracted: None,
        }
    }

    /// Sets the execution status via builder pattern.
    pub fn with_status(mut self, status: ExecutionStatus) -> Self {
        self.status = status;
        self
    }

    /// Sets the executing agent ID via builder pattern.
    pub fn with_agent_id(mut self, agent_id: Uuid) -> Self {
        self.agent_id = Some(agent_id);
        self
    }

    /// Attaches a snapshot of the task definition via builder pattern.
    pub fn with_task_snapshot(mut self, task: TaskType) -> Self {
        self.task_snapshot = Some(task);
        self
    }

    /// Sets the execution start time via builder pattern.
    pub fn with_started_at(mut self, at: DateTime<Utc>) -> Self {
        self.started_at = Some(at);
        self
    }

    /// Constructs an ExecutionRecord from a rusqlite row.
    ///
    /// Columns: id(0), job_id(1), agent_id(2), task_snapshot_json(3), status(4), exit_code(5),
    ///          stdout(6), stderr(7), stdout_truncated(8), stderr_truncated(9), started_at(10), finished_at(11), triggered_by_json(12), extracted_json(13)
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        use crate::db::helpers::{parse_datetime, parse_json, parse_uuid};

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
}
