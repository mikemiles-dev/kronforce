use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::task::TaskTypeDefinition;
use crate::agent::protocol::AgentSystemInfo;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub system_info: Option<AgentSystemInfo>,
}

impl Agent {
    /// Constructs an Agent from a rusqlite row.
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        use crate::db::helpers::{col, parse_datetime, parse_uuid};

        let id_str: String = col(row, "id")?;
        let tags_json: String = col(row, "tags_json")?;
        let agent_type_str: String =
            col(row, "agent_type").unwrap_or_else(|_| "standard".to_string());
        let status_str: String = col(row, "status")?;
        let hb_str: Option<String> = col(row, "last_heartbeat")?;
        let reg_str: String = col(row, "registered_at")?;
        let task_types_str: Option<String> = col(row, "task_types_json").unwrap_or(None);
        let system_info_str: Option<String> = col(row, "system_info_json").unwrap_or(None);

        Ok(Agent {
            id: parse_uuid(&id_str)?,
            name: col(row, "name")?,
            tags: serde_json::from_str(&tags_json).unwrap_or_default(),
            hostname: col(row, "hostname")?,
            address: col(row, "address")?,
            port: {
                let p: i64 = col(row, "port")?;
                p as u16
            },
            agent_type: AgentType::from_str(&agent_type_str),
            status: AgentStatus::from_str(&status_str).unwrap_or(AgentStatus::Offline),
            last_heartbeat: hb_str.map(|s| parse_datetime(&s)).transpose()?,
            registered_at: parse_datetime(&reg_str)?,
            task_types: task_types_str
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default(),
            system_info: system_info_str.and_then(|s| serde_json::from_str(&s).ok()),
        })
    }
}
