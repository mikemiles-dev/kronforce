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
    /// If set, restricts this key to only see/manage jobs in these groups.
    /// None means no restriction (all groups visible). Admin keys ignore this.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowed_groups: Option<Vec<String>>,
    /// If set, restricts this key to requests from these IP addresses/CIDRs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ip_allowlist: Option<Vec<String>>,
    /// If set, the key expires at this time and can no longer authenticate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<DateTime<Utc>>,
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
                allowed_groups: None,
                ip_allowlist: None,
                expires_at: None,
            },
            raw_key,
        )
    }

    /// Constructs an ApiKey from a rusqlite row.
    ///
    /// Columns: id(0), key_prefix(1), key_hash(2), name(3), role(4), created_at(5), last_used_at(6), active(7), allowed_groups_json(8)
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        use crate::db::helpers::{parse_datetime, parse_uuid};

        let id_str: String = row.get(0)?;
        let role_str: String = row.get(4)?;
        let created_str: String = row.get(5)?;
        let last_used_str: Option<String> = row.get(6)?;
        let active_int: i32 = row.get(7)?;
        let groups_json: Option<String> = row.get(8).unwrap_or(None);
        let ip_json: Option<String> = row.get(9).unwrap_or(None);
        let expires_str: Option<String> = row.get(10).unwrap_or(None);

        Ok(ApiKey {
            id: parse_uuid(&id_str)?,
            key_prefix: row.get(1)?,
            key_hash: row.get(2)?,
            name: row.get(3)?,
            role: ApiKeyRole::from_str(&role_str).unwrap_or(ApiKeyRole::Viewer),
            created_at: parse_datetime(&created_str)?,
            last_used_at: last_used_str.map(|s| parse_datetime(&s)).transpose()?,
            active: active_int != 0,
            allowed_groups: groups_json.and_then(|s| serde_json::from_str(&s).ok()),
            ip_allowlist: ip_json.and_then(|s| serde_json::from_str(&s).ok()),
            expires_at: expires_str
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(&s).ok())
                .map(|d| d.with_timezone(&Utc)),
        })
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

impl ApiKey {
    /// Returns true if this key can access jobs in the given group.
    /// Admin keys and keys with no group restriction can access all groups.
    pub fn can_access_group(&self, group: Option<&str>) -> bool {
        if self.role.can_manage_keys() {
            return true; // Admin sees everything
        }
        match &self.allowed_groups {
            None => true, // No restriction
            Some(groups) => {
                let g = group.unwrap_or("Default");
                groups.iter().any(|ag| ag == g)
            }
        }
    }
}
