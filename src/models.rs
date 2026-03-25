use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CronExpr(pub String);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
pub enum ScheduleKind {
    Cron(CronExpr),
    OneShot(DateTime<Utc>),
    #[serde(alias = "manual")]
    OnDemand,
    Event(EventTriggerConfig),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventTriggerConfig {
    pub kind_pattern: String,
    pub severity: Option<EventSeverity>,
    pub job_name_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    pub job_id: Uuid,
    pub within_secs: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskType {
    Shell {
        command: String,
    },
    Sql {
        driver: SqlDriver,
        connection_string: String,
        query: String,
    },
    Ftp {
        protocol: FtpProtocol,
        host: String,
        port: Option<u16>,
        username: String,
        password: String,
        direction: TransferDirection,
        remote_path: String,
        local_path: String,
    },
    Http {
        method: HttpMethod,
        url: String,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        expect_status: Option<u16>,
    },
    Script {
        script_name: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqlDriver {
    Postgres,
    Mysql,
    Sqlite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FtpProtocol {
    Ftp,
    Ftps,
    Sftp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Upload,
    Download,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Scheduled,
    Paused,
    Unscheduled,
}

impl JobStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            JobStatus::Scheduled => "scheduled",
            JobStatus::Paused => "paused",
            JobStatus::Unscheduled => "unscheduled",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "scheduled" | "enabled" | "active" => Some(JobStatus::Scheduled),
            "paused" | "disabled" => Some(JobStatus::Paused),
            "unscheduled" | "completed" => Some(JobStatus::Unscheduled),
            _ => None,
        }
    }
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentTarget {
    Local,
    Agent { agent_id: Uuid },
    Tagged { tag: String },
    Any,
    All,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Online,
    Offline,
    Draining,
}

impl AgentStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Online => "online",
            AgentStatus::Offline => "offline",
            AgentStatus::Draining => "draining",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "online" => Some(AgentStatus::Online),
            "offline" => Some(AgentStatus::Offline),
            "draining" => Some(AgentStatus::Draining),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub tags: Vec<String>,
    pub hostname: String,
    pub address: String,
    pub port: u16,
    pub status: AgentStatus,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub registered_at: DateTime<Utc>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerSource {
    Scheduler,
    Api,
    Dependency { parent_execution_id: Uuid },
    Event { event_id: Uuid },
}

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub id: Uuid,
    pub kind: String,
    pub severity: EventSeverity,
    pub message: String,
    pub job_id: Option<Uuid>,
    pub agent_id: Option<Uuid>,
    pub api_key_id: Option<Uuid>,
    pub api_key_name: Option<String>,
    pub details: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Info,
    Success,
    Warning,
    Error,
}

impl EventSeverity {
    pub fn as_str(&self) -> &'static str {
        match self {
            EventSeverity::Info => "info",
            EventSeverity::Success => "success",
            EventSeverity::Warning => "warning",
            EventSeverity::Error => "error",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "info" => Some(EventSeverity::Info),
            "success" => Some(EventSeverity::Success),
            "warning" => Some(EventSeverity::Warning),
            "error" => Some(EventSeverity::Error),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: Uuid,
    pub key_prefix: String,
    #[serde(skip_serializing)]
    pub key_hash: String,
    pub name: String,
    pub role: ApiKeyRole,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub active: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyRole {
    Admin,
    Operator,
    Viewer,
}

impl ApiKeyRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyRole::Admin => "admin",
            ApiKeyRole::Operator => "operator",
            ApiKeyRole::Viewer => "viewer",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(ApiKeyRole::Admin),
            "operator" => Some(ApiKeyRole::Operator),
            "viewer" => Some(ApiKeyRole::Viewer),
            _ => None,
        }
    }

    pub fn can_write(&self) -> bool {
        matches!(self, ApiKeyRole::Admin | ApiKeyRole::Operator)
    }

    pub fn can_manage_keys(&self) -> bool {
        matches!(self, ApiKeyRole::Admin)
    }
}
