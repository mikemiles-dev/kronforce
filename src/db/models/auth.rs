use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
