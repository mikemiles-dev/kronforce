use axum::Json;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::Response;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::AppState;
use crate::error::AppError;
use crate::models::*;
use axum::extract::Path;
use axum::extract::State;

pub fn hash_api_key(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

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

pub(crate) async fn agent_auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Check if any API keys exist — if not, skip auth (first-time setup)
    let db = state.db.clone();
    let key_count = tokio::task::spawn_blocking(move || db.count_api_keys())
        .await
        .unwrap()?;

    if key_count == 0 {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let raw_key = match auth_header {
        Some(ref h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err(AppError::Unauthorized("agent API key required".into()));
        }
    };

    let hash = hash_api_key(raw_key);
    let db = state.db.clone();
    let api_key = tokio::task::spawn_blocking(move || db.get_api_key_by_hash(&hash))
        .await
        .unwrap()?;

    match api_key {
        Some(key) if key.role.is_agent() || key.role.can_manage_keys() => {
            // Agent keys and admin keys can access agent endpoints
            let db = state.db.clone();
            let key_id = key.id;
            let now = Utc::now();
            let _ =
                tokio::task::spawn_blocking(move || db.update_api_key_last_used(key_id, now)).await;
            Ok(next.run(req).await)
        }
        Some(_) => Err(AppError::Forbidden(
            "this API key does not have agent access".into(),
        )),
        None => Err(AppError::Unauthorized("invalid API key".into())),
    }
}

pub(crate) async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Check if auth is enabled (any keys exist)
    let db = state.db.clone();
    let key_count = tokio::task::spawn_blocking(move || db.count_api_keys())
        .await
        .unwrap()?;

    // If no keys exist, skip auth (first-time setup)
    if key_count == 0 {
        return Ok(next.run(req).await);
    }

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let raw_key = match auth_header {
        Some(ref h) if h.starts_with("Bearer ") => &h[7..],
        _ => {
            return Err(AppError::Unauthorized(
                "missing or invalid Authorization header".into(),
            ));
        }
    };

    let hash = hash_api_key(raw_key);
    let db = state.db.clone();
    let hash2 = hash.clone();
    let api_key = tokio::task::spawn_blocking(move || db.get_api_key_by_hash(&hash2))
        .await
        .unwrap()?;

    match api_key {
        Some(key) => {
            // Update last_used_at
            let db = state.db.clone();
            let key_id = key.id;
            let now = Utc::now();
            let _ =
                tokio::task::spawn_blocking(move || db.update_api_key_last_used(key_id, now)).await;

            req.extensions_mut().insert(key);
            Ok(next.run(req).await)
        }
        None => Err(AppError::Unauthorized("invalid API key".into())),
    }
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

#[derive(Deserialize)]
pub(crate) struct CreateApiKeyRequest {
    name: String,
    role: ApiKeyRole,
}

#[derive(Serialize)]
pub(crate) struct CreateApiKeyResponse {
    key: ApiKey,
    raw_key: String,
}

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

    let db = state.db.clone();
    let key2 = key.clone();
    tokio::task::spawn_blocking(move || db.insert_api_key(&key2))
        .await
        .unwrap()?;

    let db_log = state.db.clone();
    let key_name = key.name.clone();
    let _ = tokio::task::spawn_blocking(move || {
        db_log.log_event(
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

pub(crate) async fn list_api_keys(
    State(state): State<AppState>,
    req: Request,
) -> Result<Json<Vec<ApiKey>>, AppError> {
    require_admin(&req)?;

    let db = state.db.clone();
    let keys = tokio::task::spawn_blocking(move || db.list_api_keys())
        .await
        .unwrap()?;
    Ok(Json(keys))
}

pub(crate) async fn revoke_api_key(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    req: Request,
) -> Result<axum::http::StatusCode, AppError> {
    require_admin(&req)?;

    let db = state.db.clone();
    tokio::task::spawn_blocking(move || db.delete_api_key(id))
        .await
        .unwrap()?;

    let db_log = state.db.clone();
    let _ = tokio::task::spawn_blocking(move || {
        db_log.log_event(
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
