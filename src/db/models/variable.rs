use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A named variable that can be substituted into task fields at runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
    /// If true, the value is masked in API responses and the UI.
    #[serde(default)]
    pub secret: bool,
    /// Optional expiration date. Expired variables are still stored but flagged in the UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
}
