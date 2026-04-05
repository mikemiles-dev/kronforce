use axum::Json;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::AppState;
use crate::db::db_call;
use crate::db::models::*;
use crate::error::AppError;
use axum::extract::Path;
use axum::extract::State;

/// Computes the SHA-256 hash of a raw API key for storage and lookup.
pub fn hash_api_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Generates a new random API key, returning (raw_key, prefix).
pub fn generate_api_key() -> (String, String) {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    let raw = format!(
        "kf_{}",
        base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
    );
    let prefix = raw[..11].to_string(); // "kf_" + first 8 chars of base64
    (raw, prefix)
}

/// Validates a Bearer token from request headers against the database.
/// Returns None if no keys are configured (auth disabled), or the matching ApiKey.
async fn validate_bearer_token(
    db: &crate::db::Db,
    headers: &axum::http::HeaderMap,
    missing_msg: &str,
) -> Result<Option<ApiKey>, AppError> {
    let key_count = db_call(db, move |db| db.count_api_keys()).await?;
    if key_count == 0 {
        return Ok(None);
    }

    let auth_header = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let raw_key = match auth_header {
        Some(ref h) if h.starts_with("Bearer ") => &h[7..],
        _ => return Err(AppError::Unauthorized(missing_msg.into())),
    };

    let hash = hash_api_key(raw_key);
    let api_key = db_call(db, move |db| db.get_api_key_by_hash(&hash)).await?;

    match api_key {
        Some(key) => Ok(Some(key)),
        None => Err(AppError::Unauthorized("invalid API key".into())),
    }
}

/// Extracts and validates a session cookie, returning a synthetic ApiKey if valid.
async fn validate_session_cookie(
    db: &crate::db::Db,
    headers: &axum::http::HeaderMap,
) -> Result<Option<ApiKey>, AppError> {
    let cookie_header = match headers.get("cookie").and_then(|v| v.to_str().ok()) {
        Some(c) => c.to_string(),
        None => return Ok(None),
    };

    // Parse cookies and find kf_session
    let session_value = cookie_header
        .split(';')
        .map(|s| s.trim())
        .find_map(|s| s.strip_prefix("kf_session="))
        .map(|s| s.to_string());

    let raw_session = match session_value {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(None),
    };

    let session_hash = hash_api_key(&raw_session);
    let db2 = db.clone();
    let hash2 = session_hash.clone();
    let session = db_call(&db2, move |db| db.get_session_by_hash(&hash2)).await?;

    match session {
        Some(sess) => {
            // Touch last_active_at
            let db3 = db.clone();
            let hash3 = session_hash;
            let _ = db_call(&db3, move |db| db.touch_session(&hash3)).await;

            // Create a synthetic ApiKey from session data
            Ok(Some(ApiKey {
                id: Uuid::new_v4(),
                key_prefix: String::new(),
                key_hash: String::new(),
                name: sess.user_name,
                role: sess.role,
                created_at: sess.created_at,
                last_used_at: Some(Utc::now()),
                active: true,
                allowed_groups: None, // OIDC sessions have no group restrictions
            }))
        }
        None => Ok(None),
    }
}

/// Updates the last-used timestamp for an API key. Called after permission checks pass.
async fn touch_api_key(db: &crate::db::Db, key_id: Uuid) {
    let now = Utc::now();
    let _ = db_call(db, move |db| db.update_api_key_last_used(key_id, now)).await;
}

/// Middleware that authenticates agent-facing endpoints using Bearer tokens.
/// Skips auth if no API keys are configured (first-time setup).
pub(crate) async fn agent_auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let Some(key) =
        validate_bearer_token(&state.db, req.headers(), "agent API key required").await?
    else {
        return Ok(next.run(req).await);
    };

    if key.role.is_agent() || key.role.can_manage_keys() {
        touch_api_key(&state.db, key.id).await;
        Ok(next.run(req).await)
    } else {
        Err(AppError::Forbidden(
            "this API key does not have agent access".into(),
        ))
    }
}

/// Middleware that authenticates user-facing API endpoints using Bearer tokens
/// or session cookies. Skips auth if no API keys are configured (first-time setup).
pub(crate) async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Try Bearer token first
    let has_bearer = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .is_some_and(|s| s.starts_with("Bearer "));

    if has_bearer {
        let Some(key) = validate_bearer_token(
            &state.db,
            req.headers(),
            "missing or invalid Authorization header",
        )
        .await?
        else {
            return Ok(next.run(req).await);
        };
        touch_api_key(&state.db, key.id).await;
        req.extensions_mut().insert(key);
        return Ok(next.run(req).await);
    }

    // Try session cookie
    if let Ok(Some(key)) = validate_session_cookie(&state.db, req.headers()).await {
        req.extensions_mut().insert(key);
        return Ok(next.run(req).await);
    }

    // No Bearer and no cookie — check if auth is disabled (no keys configured)
    let key_count = db_call(&state.db, move |db| db.count_api_keys()).await?;
    if key_count == 0 {
        return Ok(next.run(req).await);
    }

    Err(AppError::Unauthorized(
        "missing or invalid Authorization header".into(),
    ))
}

/// Extractor for the authenticated API key. Returns None if auth is disabled.
#[derive(Clone)]
pub(crate) struct AuthUser(pub(crate) Option<ApiKey>);

impl<S: Send + Sync> axum::extract::FromRequestParts<S> for AuthUser {
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(AuthUser(parts.extensions.get::<ApiKey>().cloned()))
    }
}

/// Guard that rejects requests unless the caller has admin privileges.
pub(crate) fn require_admin(req: &Request) -> Result<(), AppError> {
    if let Some(key) = req.extensions().get::<ApiKey>() {
        if key.role.can_manage_keys() {
            Ok(())
        } else {
            Err(AppError::Forbidden("admin role required".into()))
        }
    } else {
        Ok(())
    }
}

/// Returns information about the currently authenticated user (API key or OIDC session).
pub(crate) async fn auth_me(req: Request) -> Json<serde_json::Value> {
    if let Some(key) = req.extensions().get::<ApiKey>() {
        if key.key_prefix.is_empty() {
            // OIDC session (synthetic ApiKey has empty key_prefix)
            Json(serde_json::json!({
                "authenticated": true,
                "auth_type": "oidc",
                "name": key.name,
                "role": key.role,
            }))
        } else {
            Json(serde_json::json!({
                "authenticated": true,
                "auth_type": "api_key",
                "key_id": key.id,
                "key_prefix": key.key_prefix,
                "name": key.name,
                "role": key.role,
            }))
        }
    } else {
        Json(serde_json::json!({
            "authenticated": false,
            "message": "no API keys configured, auth disabled",
        }))
    }
}

/// Logs out by clearing the session cookie and deleting the server-side session.
pub(crate) async fn logout(
    State(state): State<AppState>,
    req: Request,
) -> Result<Response, AppError> {
    // Find and delete the session
    if let Some(cookie_header) = req.headers().get("cookie").and_then(|v| v.to_str().ok())
        && let Some(raw_session) = cookie_header
            .split(';')
            .map(|s| s.trim())
            .find_map(|s| s.strip_prefix("kf_session="))
        && !raw_session.is_empty()
    {
        let session_hash = hash_api_key(raw_session);
        let _ = db_call(&state.db, move |db| db.delete_session(&session_hash)).await;
    }

    // Clear the cookie — include Secure flag to match how it was set
    let clear_cookie = "kf_session=; HttpOnly; SameSite=Lax; Secure; Path=/; Max-Age=0";
    let mut resp =
        axum::response::Response::new(axum::body::Body::from(r#"{"status":"logged out"}"#));
    resp.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        clear_cookie.parse().unwrap(),
    );
    resp.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );
    Ok(resp)
}

/// Request body for creating a new API key.
#[derive(Deserialize)]
pub(crate) struct CreateApiKeyRequest {
    name: String,
    role: ApiKeyRole,
    /// Restrict this key to specific job groups. Omit or null for unrestricted access.
    #[serde(default)]
    allowed_groups: Option<Vec<String>>,
}

/// Response containing the newly created key and its raw (unhashed) value.
#[derive(Serialize)]
pub(crate) struct CreateApiKeyResponse {
    key: ApiKey,
    raw_key: String,
}

/// Creates a new API key. Requires admin privileges.
pub(crate) async fn create_api_key(
    State(state): State<AppState>,
    req: Request,
) -> Result<Json<CreateApiKeyResponse>, AppError> {
    require_admin(&req)?;

    let actor_key = req.extensions().get::<ApiKey>().cloned();
    let actor_id = actor_key.as_ref().map(|k| k.id);
    let actor_name = actor_key.as_ref().map(|k| k.name.clone());

    let bytes = axum::body::to_bytes(req.into_body(), 1024 * 64)
        .await
        .map_err(|e| AppError::BadRequest(format!("invalid body: {e}")))?;
    let body: CreateApiKeyRequest = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::BadRequest(format!("invalid JSON: {e}")))?;

    let (raw_key, prefix) = generate_api_key();
    let hash = hash_api_key(&raw_key);

    let key = ApiKey {
        id: Uuid::new_v4(),
        key_prefix: prefix,
        key_hash: hash,
        name: body.name,
        role: body.role,
        created_at: Utc::now(),
        last_used_at: None,
        active: true,
        allowed_groups: body.allowed_groups.filter(|g| !g.is_empty()),
    };

    let key2 = key.clone();
    db_call(&state.db, move |db| db.insert_api_key(&key2)).await?;

    let key_name = key.name.clone();
    let _ = db_call(&state.db, move |db| {
        db.log_event(
            "key.created",
            EventSeverity::Info,
            &format!("API key '{}' created", key_name),
            None,
            None,
        )
    })
    .await;

    let audit_key_id = key.id.to_string();
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "key.created",
            "api_key",
            Some(&audit_key_id),
            actor_id,
            actor_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(Json(CreateApiKeyResponse { key, raw_key }))
}

/// Lists all API keys. Requires admin privileges.
pub(crate) async fn list_api_keys(
    State(state): State<AppState>,
    req: Request,
) -> Result<Json<Vec<ApiKey>>, AppError> {
    require_admin(&req)?;

    let keys = db_call(&state.db, move |db| db.list_api_keys()).await?;
    Ok(Json(keys))
}

/// Revokes an API key by ID. Requires admin privileges.
pub(crate) async fn revoke_api_key(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    req: Request,
) -> Result<axum::http::StatusCode, AppError> {
    require_admin(&req)?;

    let actor_key = req.extensions().get::<ApiKey>().cloned();
    let actor_id = actor_key.as_ref().map(|k| k.id);
    let actor_name = actor_key.as_ref().map(|k| k.name.clone());

    db_call(&state.db, move |db| db.delete_api_key(id)).await?;

    let _ = db_call(&state.db, move |db| {
        db.log_event(
            "key.revoked",
            EventSeverity::Warning,
            &format!("API key {} revoked", id),
            None,
            None,
        )
    })
    .await;

    let audit_key_id = id.to_string();
    let db_audit = state.db.clone();
    let _ = db_call(&db_audit, move |db| {
        db.record_audit(
            "key.revoked",
            "api_key",
            Some(&audit_key_id),
            actor_id,
            actor_name.as_deref(),
            None,
        )
    })
    .await;

    Ok(axum::http::StatusCode::NO_CONTENT)
}
