use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::ExecutionStatus;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistration {
    pub name: String,
    pub tags: Vec<String>,
    pub hostname: String,
    pub address: String,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRegistrationResponse {
    pub agent_id: Uuid,
    pub heartbeat_interval_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHeartbeat {
    pub agent_id: Uuid,
    pub running_executions: Vec<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDispatchRequest {
    pub execution_id: Uuid,
    pub job_id: Uuid,
    pub command: String,
    pub timeout_secs: Option<u64>,
    pub callback_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobDispatchResponse {
    pub accepted: bool,
    pub message: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CancelRequest {
    pub execution_id: Uuid,
}
