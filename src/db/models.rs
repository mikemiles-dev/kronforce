use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::notifications::NotificationRecipients;

/// A selectable option for a task field (e.g., dropdown value).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldOption {
    pub value: String,
    pub label: String,
}

/// Schema definition for a single field within a task type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskFieldDefinition {
    pub name: String,
    pub label: String,
    pub field_type: String,
    #[serde(default)]
    pub required: Option<bool>,
    pub placeholder: Option<String>,
    pub options: Option<Vec<FieldOption>>,
}

/// Describes a custom task type with its name, description, and required fields.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskTypeDefinition {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<TaskFieldDefinition>,
}

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

/// The work a job performs. Each variant represents a different execution backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskType {
    /// Execute a shell command on the target host.
    Shell { command: String },
    /// Run a SQL query against a database.
    Sql {
        driver: SqlDriver,
        connection_string: String,
        query: String,
    },
    /// Transfer a file via FTP, FTPS, or SFTP.
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
    /// Make an HTTP request.
    Http {
        method: HttpMethod,
        url: String,
        headers: Option<HashMap<String, String>>,
        body: Option<String>,
        expect_status: Option<u16>,
    },
    /// Run a stored Rhai script by name.
    Script { script_name: String },
    /// Agent-defined custom task type with arbitrary JSON data.
    Custom {
        agent_task_type: String,
        data: serde_json::Value,
    },
    /// Push a file (base64-encoded) to the target host.
    FilePush {
        filename: String,
        destination: String,
        content_base64: String,
        permissions: Option<String>,
        #[serde(default)]
        overwrite: bool,
    },
    /// Publish a message to a Kafka topic.
    Kafka {
        broker: String,
        topic: String,
        message: String,
        key: Option<String>,
        properties: Option<String>,
    },
    /// Publish a message to a RabbitMQ exchange.
    Rabbitmq {
        url: String,
        exchange: String,
        routing_key: String,
        message: String,
        content_type: Option<String>,
    },
    /// Publish a message to an MQTT topic.
    Mqtt {
        broker: String,
        topic: String,
        message: String,
        port: Option<u16>,
        qos: Option<u8>,
        username: Option<String>,
        password: Option<String>,
        client_id: Option<String>,
    },
    /// Publish a message to a Redis channel.
    Redis {
        url: String,
        channel: String,
        message: String,
    },
}

/// Supported SQL database drivers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SqlDriver {
    Postgres,
    Mysql,
    Sqlite,
}

/// File transfer protocol variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FtpProtocol {
    Ftp,
    Ftps,
    Sftp,
}

/// Direction of a file transfer operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferDirection {
    Upload,
    Download,
}

/// HTTP method for HTTP task requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
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

/// Connectivity state of a remote agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    /// Agent is connected and accepting work.
    Online,
    /// Agent has not sent a heartbeat within the timeout window.
    Offline,
    /// Agent is finishing current work but not accepting new jobs.
    Draining,
}

/// Whether an agent uses standard or custom task execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    Standard,
    Custom,
}

impl AgentType {
    /// Returns the string representation of this agent type.
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentType::Standard => "standard",
            AgentType::Custom => "custom",
        }
    }

    /// Parses a string into an `AgentType`, defaulting to `Standard`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "custom" => AgentType::Custom,
            _ => AgentType::Standard,
        }
    }
}

impl AgentStatus {
    /// Returns the string representation of this agent status.
    pub fn as_str(&self) -> &'static str {
        match self {
            AgentStatus::Online => "online",
            AgentStatus::Offline => "offline",
            AgentStatus::Draining => "draining",
        }
    }

    /// Parses a status string into an `AgentStatus`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "online" => Some(AgentStatus::Online),
            "offline" => Some(AgentStatus::Offline),
            "draining" => Some(AgentStatus::Draining),
            _ => None,
        }
    }
}

/// A registered remote agent that can execute jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub id: Uuid,
    pub name: String,
    pub tags: Vec<String>,
    pub hostname: String,
    pub address: String,
    pub port: u16,
    pub agent_type: AgentType,
    pub status: AgentStatus,
    pub last_heartbeat: Option<DateTime<Utc>>,
    pub registered_at: DateTime<Utc>,
    #[serde(default)]
    pub task_types: Vec<TaskTypeDefinition>,
}

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
}

/// An auditable system event (e.g., job fired, agent offline, API action).
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

/// Severity level for system events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Info,
    Success,
    Warning,
    Error,
}

impl EventSeverity {
    /// Returns the string representation of this severity level.
    pub fn as_str(&self) -> &'static str {
        match self {
            EventSeverity::Info => "info",
            EventSeverity::Success => "success",
            EventSeverity::Warning => "warning",
            EventSeverity::Error => "error",
        }
    }

    /// Parses a severity string into an `EventSeverity`.
    #[allow(clippy::should_implement_trait)]
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

/// An API key used for authenticating requests to the controller.
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

const KEY_PREFIX_LEN: usize = 11;

impl ApiKey {
    /// Creates a new API key (optionally from a preset), returning the key and its raw value.
    pub fn bootstrap(role: ApiKeyRole, name: &str, preset_key: Option<String>) -> (Self, String) {
        let (raw_key, prefix) = if let Some(preset) = preset_key.filter(|k| !k.is_empty()) {
            let pfx = preset.get(..KEY_PREFIX_LEN).unwrap_or(&preset).to_string();
            (preset, pfx)
        } else {
            crate::api::generate_api_key()
        };
        let hash = crate::api::hash_api_key(&raw_key);
        (
            ApiKey {
                id: Uuid::new_v4(),
                key_prefix: prefix,
                key_hash: hash,
                name: name.to_string(),
                role,
                created_at: Utc::now(),
                last_used_at: None,
                active: true,
            },
            raw_key,
        )
    }
}

/// Permission role assigned to an API key.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeyRole {
    Admin,
    Operator,
    Viewer,
    Agent,
}

impl ApiKeyRole {
    /// Returns the string representation of this role.
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiKeyRole::Admin => "admin",
            ApiKeyRole::Operator => "operator",
            ApiKeyRole::Viewer => "viewer",
            ApiKeyRole::Agent => "agent",
        }
    }

    /// Parses a role string into an `ApiKeyRole`.
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "admin" => Some(ApiKeyRole::Admin),
            "operator" => Some(ApiKeyRole::Operator),
            "viewer" => Some(ApiKeyRole::Viewer),
            "agent" => Some(ApiKeyRole::Agent),
            _ => None,
        }
    }

    /// Returns `true` if this role has write access (Admin or Operator).
    pub fn can_write(&self) -> bool {
        matches!(self, ApiKeyRole::Admin | ApiKeyRole::Operator)
    }

    /// Returns `true` if this role can create and revoke API keys.
    pub fn can_manage_keys(&self) -> bool {
        matches!(self, ApiKeyRole::Admin)
    }

    /// Returns `true` if this is an agent-scoped key.
    pub fn is_agent(&self) -> bool {
        matches!(self, ApiKeyRole::Agent)
    }
}

/// A named variable that can be substituted into task fields at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}
