use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A named connection profile storing credentials for external systems.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Connection {
    pub name: String,
    pub conn_type: ConnectionType,
    pub description: Option<String>,
    pub config: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Supported connection protocol types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionType {
    Postgres,
    Mysql,
    Sqlite,
    Ftp,
    Sftp,
    Http,
    Kafka,
    Mqtt,
    Rabbitmq,
    Redis,
    Mongodb,
    Ssh,
    Smtp,
    S3,
}

impl std::fmt::Display for ConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{:?}", self));
        write!(f, "{}", s)
    }
}

/// Sensitive field names that should be masked in API responses.
const SENSITIVE_FIELDS: &[&str] = &[
    "password",
    "private_key",
    "token",
    "secret_key",
    "header_value",
    "access_key",
];

/// Mask sensitive fields in a connection config for API responses.
pub fn mask_config(config: &serde_json::Value) -> serde_json::Value {
    match config {
        serde_json::Value::Object(map) => {
            let mut masked = serde_json::Map::new();
            for (k, v) in map {
                if SENSITIVE_FIELDS.contains(&k.as_str()) {
                    if let Some(s) = v.as_str() {
                        if !s.is_empty() {
                            masked.insert(k.clone(), serde_json::Value::String("********".into()));
                        } else {
                            masked.insert(k.clone(), v.clone());
                        }
                    } else {
                        masked.insert(k.clone(), v.clone());
                    }
                } else {
                    masked.insert(k.clone(), v.clone());
                }
            }
            serde_json::Value::Object(masked)
        }
        other => other.clone(),
    }
}

/// Sentinel value used by the frontend for masked fields.
pub const MASK_SENTINEL: &str = "********";

/// Merge an update config with existing config, preserving masked sentinel values.
pub fn merge_config_preserving_secrets(
    existing: &serde_json::Value,
    update: &serde_json::Value,
) -> serde_json::Value {
    match (existing, update) {
        (serde_json::Value::Object(old), serde_json::Value::Object(new)) => {
            let mut merged = serde_json::Map::new();
            for (k, new_val) in new {
                if new_val.as_str() == Some(MASK_SENTINEL) {
                    // Preserve existing value
                    if let Some(old_val) = old.get(k) {
                        merged.insert(k.clone(), old_val.clone());
                    } else {
                        merged.insert(k.clone(), new_val.clone());
                    }
                } else {
                    merged.insert(k.clone(), new_val.clone());
                }
            }
            serde_json::Value::Object(merged)
        }
        _ => update.clone(),
    }
}
