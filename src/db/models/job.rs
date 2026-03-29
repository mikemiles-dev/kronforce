use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::event::EventSeverity;
use super::task::TaskType;
use crate::executor::notifications::NotificationRecipients;

/// A cron expression string (e.g., `"0 */5 * * *"`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronExpr(pub String);

/// How a job is scheduled to run.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ScheduleKind {
    /// Recurring schedule driven by a cron expression.
    Cron(CronExpr),
    /// Fires exactly once at the specified time.
    OneShot(DateTime<Utc>),
    /// Only runs when triggered manually via the API.
    #[serde(alias = "manual")]
    OnDemand,
    /// Fires in response to matching system events.
    Event(EventTriggerConfig),
}

/// Configuration for event-driven job triggers, with kind/severity/job-name filters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTriggerConfig {
    pub kind_pattern: String,
    pub severity: Option<EventSeverity>,
    pub job_name_filter: Option<String>,
}

/// A dependency on another job, optionally constrained by a recency window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub job_id: Uuid,
    pub within_secs: Option<u64>,
}

/// Current scheduling state of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    /// Job is active and will fire according to its schedule.
    Scheduled,
    /// Job exists but is temporarily disabled.
    Paused,
    /// Job has no future scheduled runs (e.g., one-shot that already fired).
    Unscheduled,
}

impl JobStatus {
    /// Returns the string representation of this status.
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Scheduled => "scheduled",
            JobStatus::Paused => "paused",
            JobStatus::Unscheduled => "unscheduled",
        }
    }

    /// Parses a status string, accepting legacy aliases (e.g., "enabled" -> Scheduled).
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "scheduled" | "enabled" | "active" => Some(JobStatus::Scheduled),
            "paused" | "disabled" => Some(JobStatus::Paused),
            "unscheduled" | "completed" => Some(JobStatus::Unscheduled),
            _ => None,
        }
    }
}

/// A rule for extracting named values from task output using regex or JSONPath.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionRule {
    pub name: String,
    pub pattern: String,
    #[serde(rename = "type")]
    pub rule_type: String, // "regex" or "jsonpath"
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub write_to_variable: Option<String>,
}

/// Matches a pattern in task output and raises an event at the given severity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputTrigger {
    pub pattern: String,
    pub severity: String, // "error", "warning", "info", "success"
}

/// A pattern that must appear in task output; if absent the execution is marked failed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputAssertion {
    pub pattern: String,
    pub message: Option<String>, // custom failure message
}

/// Post-execution output processing: extractions, triggers, and assertions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OutputRules {
    #[serde(default)]
    pub extractions: Vec<ExtractionRule>,
    #[serde(default)]
    pub triggers: Vec<OutputTrigger>,
    #[serde(default)]
    pub assertions: Vec<OutputAssertion>,
}

/// Per-job notification preferences (on failure, success, or assertion failure).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JobNotificationConfig {
    #[serde(default)]
    pub on_failure: bool,
    #[serde(default)]
    pub on_success: bool,
    #[serde(default)]
    pub on_assertion_failure: bool,
    #[serde(default)]
    pub recipients: Option<NotificationRecipients>,
}

/// Where a job should be executed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTarget {
    /// Run on the controller itself.
    Local,
    /// Run on a specific agent by ID.
    Agent { agent_id: Uuid },
    /// Run on any agent matching the given tag.
    Tagged { tag: String },
    /// Run on any available online agent.
    Any,
    /// Run on all online agents simultaneously.
    All,
}

/// A scheduled job definition including its task, schedule, dependencies, and targeting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Job {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub task: TaskType,
    pub run_as: Option<String>,
    pub schedule: ScheduleKind,
    pub status: JobStatus,
    pub timeout_secs: Option<u64>,
    pub depends_on: Vec<Dependency>,
    pub target: Option<AgentTarget>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub output_rules: Option<OutputRules>,
    #[serde(default)]
    pub notifications: Option<JobNotificationConfig>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub retry_max: u32,
    #[serde(default)]
    pub retry_delay_secs: u64,
    #[serde(default = "default_retry_backoff")]
    pub retry_backoff: f64,
}

fn default_retry_backoff() -> f64 {
    1.0
}

impl Job {
    /// Constructs a Job from a rusqlite row.
    ///
    /// Columns: id(0), name(1), description(2), task_json(3), run_as(4), schedule_json(5), status(6),
    ///          timeout_secs(7), depends_on_json(8), target_json(9), created_by(10), created_at(11), updated_at(12), output_rules_json(13), notifications_json(14), group_name(15), retry_max(16), retry_delay_secs(17), retry_backoff(18)
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        use crate::db::helpers::{parse_datetime, parse_json, parse_uuid};

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
            group: row.get::<_, Option<String>>(15).unwrap_or(None).or_else(|| Some("Default".to_string())),
            retry_max: row.get::<_, Option<i64>>(16).unwrap_or(None).unwrap_or(0) as u32,
            retry_delay_secs: row.get::<_, Option<i64>>(17).unwrap_or(None).unwrap_or(0) as u64,
            retry_backoff: row.get::<_, Option<f64>>(18).unwrap_or(None).unwrap_or(1.0),
        })
    }
}
