use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::db::models::{ExecutionStatus, TaskType, TaskTypeDefinition};

/// System information reported by an agent for node inventory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentSystemInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cpus: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_mb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub os_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_version: Option<String>,
}

/// Registration payload sent by an agent when it first connects to the controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub name: String,
    pub tags: Vec<String>,
    pub hostname: String,
    pub address: String,
    pub port: u16,
    pub agent_type: Option<String>,
    pub task_types: Option<Vec<TaskTypeDefinition>>,
    #[serde(default)]
    pub system_info: Option<AgentSystemInfo>,
}

/// Controller's response to a successful agent registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistrationResponse {
    pub agent_id: Uuid,
    pub heartbeat_interval_secs: u64,
}

/// Periodic heartbeat sent by an agent to report liveness and running executions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeat {
    pub agent_id: Uuid,
    pub running_executions: Vec<Uuid>,
}

/// Request from the controller to an agent to execute a job.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDispatchRequest {
    pub execution_id: Uuid,
    pub job_id: Uuid,
    pub task: TaskType,
    pub run_as: Option<String>,
    pub timeout_secs: Option<u64>,
    pub callback_url: String,
}

/// Agent's response indicating whether a dispatched job was accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDispatchResponse {
    pub accepted: bool,
    pub message: Option<String>,
}

/// Result payload sent by an agent back to the controller after a job finishes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResultReport {
    pub execution_id: Uuid,
    pub job_id: Uuid,
    pub agent_id: Uuid,
    pub status: ExecutionStatus,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
}

/// Request from the controller to cancel a running execution on an agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequest {
    pub execution_id: Uuid,
}
