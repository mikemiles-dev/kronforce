use chrono::{DateTime, Utc};
use rusqlite::params;

use super::Db;
use crate::db::models::session::{OidcAuthState, OidcSession};
use crate::error::AppError;

impl Db {
    /// Inserts a new OIDC session.
    pub fn insert_session(&self, session: &OidcSession) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "INSERT INTO oidc_sessions (id_hash, user_email, user_name, role, id_token_claims, created_at, expires_at, last_active_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                session.id_hash,
                session.user_email,
                session.user_name,
                session.role.as_str(),
                session.id_token_claims,
                session.created_at.to_rfc3339(),
                session.expires_at.to_rfc3339(),
                session.last_active_at.to_rfc3339(),
            ],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Looks up a session by its hashed ID. Returns None if not found or expired.
    pub fn get_session_by_hash(&self, id_hash: &str) -> Result<Option<OidcSession>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let now = Utc::now().to_rfc3339();
        let mut stmt = conn
            .prepare("SELECT id_hash, user_email, user_name, role, id_token_claims, created_at, expires_at, last_active_at FROM oidc_sessions WHERE id_hash = ?1 AND expires_at > ?2")
            .map_err(AppError::Db)?;
        let mut rows = stmt
            .query_map(params![id_hash, now], OidcSession::from_row)
            .map_err(AppError::Db)?;
        match rows.next() {
            Some(Ok(session)) => Ok(Some(session)),
            Some(Err(e)) => Err(AppError::Internal(format!("session parse: {e}"))),
            None => Ok(None),
        }
    }

    /// Updates last_active_at for a session.
    pub fn touch_session(&self, id_hash: &str) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let now = Utc::now().to_rfc3339();
        conn.execute(
            "UPDATE oidc_sessions SET last_active_at = ?1 WHERE id_hash = ?2",
            params![now, id_hash],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Deletes a session by its hashed ID.
    pub fn delete_session(&self, id_hash: &str) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "DELETE FROM oidc_sessions WHERE id_hash = ?1",
            params![id_hash],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Removes expired sessions and stale auth states.
    pub fn cleanup_expired_sessions(&self) -> Result<u64, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let now = Utc::now().to_rfc3339();
        let sessions_removed = conn
            .execute(
                "DELETE FROM oidc_sessions WHERE expires_at <= ?1",
                params![now],
            )
            .map_err(AppError::Db)? as u64;
        // Clean up auth states older than 5 minutes
        let cutoff = (Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
        let _ = conn.execute(
            "DELETE FROM oidc_auth_states WHERE created_at < ?1",
            params![cutoff],
        );
        Ok(sessions_removed)
    }

    /// Stores a temporary OIDC auth state for CSRF protection.
    pub fn insert_auth_state(
        &self,
        state: &str,
        nonce: &str,
        created_at: DateTime<Utc>,
    ) -> Result<(), AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        conn.execute(
            "INSERT INTO oidc_auth_states (state, nonce, created_at) VALUES (?1, ?2, ?3)",
            params![state, nonce, created_at.to_rfc3339()],
        )
        .map_err(AppError::Db)?;
        Ok(())
    }

    /// Consumes an auth state by its state parameter. Returns the nonce if found.
    pub fn consume_auth_state(&self, state: &str) -> Result<Option<OidcAuthState>, AppError> {
        let conn = self
            .pool
            .get()
            .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
        let cutoff = (Utc::now() - chrono::Duration::minutes(5)).to_rfc3339();
        let mut stmt = conn
            .prepare("SELECT state, nonce, created_at FROM oidc_auth_states WHERE state = ?1 AND created_at >= ?2")
            .map_err(AppError::Db)?;
        let result = stmt
            .query_row(params![state, cutoff], |row| {
                let created_str: String = row.get(2)?;
                Ok(OidcAuthState {
                    state: row.get(0)?,
                    nonce: row.get(1)?,
                    created_at: crate::db::helpers::parse_datetime(&created_str)?,
                })
            })
            .ok();
        // Delete the consumed state
        let _ = conn.execute(
            "DELETE FROM oidc_auth_states WHERE state = ?1",
            params![state],
        );
        Ok(result)
    }
}
