use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ApiKeyRole;

/// An authenticated OIDC session stored in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcSession {
    pub id_hash: String,
    pub user_email: String,
    pub user_name: String,
    pub role: ApiKeyRole,
    pub id_token_claims: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}

impl OidcSession {
    pub(crate) fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        use crate::db::helpers::{col, parse_datetime};

        let role_str: String = col(row, "role")?;
        let created_str: String = col(row, "created_at")?;
        let expires_str: String = col(row, "expires_at")?;
        let last_active_str: String = col(row, "last_active_at")?;

        Ok(OidcSession {
            id_hash: col(row, "id_hash")?,
            user_email: col(row, "user_email")?,
            user_name: col(row, "user_name")?,
            role: ApiKeyRole::from_str(&role_str).unwrap_or(ApiKeyRole::Viewer),
            id_token_claims: col(row, "id_token_claims")?,
            created_at: parse_datetime(&created_str)?,
            expires_at: parse_datetime(&expires_str)?,
            last_active_at: parse_datetime(&last_active_str)?,
        })
    }
}

/// Temporary OIDC authorization state for CSRF protection.
pub struct OidcAuthState {
    pub state: String,
    pub nonce: String,
    pub created_at: DateTime<Utc>,
}
