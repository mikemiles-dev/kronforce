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
use crate::error::AppError;
use crate::models::*;
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
        Some(key) => {
            let key_id = key.id;
            let now = Utc::now();
            let _ = db_call(db, move |db| db.update_api_key_last_used(key_id, now)).await;
            Ok(Some(key))
        }
        None => Err(AppError::Unauthorized("invalid API key".into())),
    }
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
        Ok(next.run(req).await)
    } else {
        Err(AppError::Forbidden(
            "this API key does not have agent access".into(),
        ))
    }
}

/// Middleware that authenticates user-facing API endpoints using Bearer tokens.
/// Skips auth if no API keys are configured (first-time setup).
pub(crate) async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    let Some(key) = validate_bearer_token(
        &state.db,
        req.headers(),
        "missing or invalid Authorization header",
    )
    .await?
    else {
        return Ok(next.run(req).await);
    };

    req.extensions_mut().insert(key);
    Ok(next.run(req).await)
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

/// Returns information about the currently authenticated API key.
pub(crate) async fn auth_me(req: Request) -> Json<serde_json::Value> {
    if let Some(key) = req.extensions().get::<ApiKey>() {
        Json(serde_json::json!({
            "authenticated": true,
            "key_id": key.id,
            "key_prefix": key.key_prefix,
            "name": key.name,
            "role": key.role,
        }))
    } else {
        Json(serde_json::json!({
            "authenticated": false,
            "message": "no API keys configured, auth disabled",
        }))
    }
}

/// Request body for creating a new API key.
#[derive(Deserialize)]
pub(crate) struct CreateApiKeyRequest {
    name: String,
    role: ApiKeyRole,
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

    Ok(axum::http::StatusCode::NO_CONTENT)
}
