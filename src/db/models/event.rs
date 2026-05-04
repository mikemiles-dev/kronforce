use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
    /// The execution this event refers to, when applicable. Lets the UI link
    /// directly from an event row to the execution's output.
    #[serde(default)]
    pub execution_id: Option<Uuid>,
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
