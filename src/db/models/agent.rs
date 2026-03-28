use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::task::TaskTypeDefinition;

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
