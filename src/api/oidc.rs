use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use axum::Json;
use chrono::{Duration, Utc};
use serde::Deserialize;
use tracing::{error, info, warn};

use super::AppState;
use crate::api::auth::hash_api_key;
use crate::config::OidcConfig;
use crate::db::db_call;
use crate::db::models::session::OidcSession;
use crate::db::models::ApiKeyRole;
use crate::error::AppError;

/// Cached OIDC provider endpoints from discovery.
pub struct OidcProvider {
    pub authorization_endpoint: String,
    pub token_endpoint: String,
    pub issuer: String,
}

/// Runtime OIDC state stored in AppState.
pub struct OidcState {
    pub config: OidcConfig,
    pub provider: OidcProvider,
}

/// Fetches the OIDC discovery document from the issuer.
pub async fn discover(issuer: &str) -> Result<OidcProvider, String> {
    let url = format!(
        "{}/.well-known/openid-configuration",
        issuer.trim_end_matches('/')
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|e| format!("OIDC discovery request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("OIDC discovery returned {}", resp.status()));
    }
    let doc: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("OIDC discovery parse failed: {e}"))?;

    let authorization_endpoint = doc["authorization_endpoint"]
        .as_str()
        .ok_or("missing authorization_endpoint in OIDC discovery")?
        .to_string();
    let token_endpoint = doc["token_endpoint"]
        .as_str()
        .ok_or("missing token_endpoint in OIDC discovery")?
        .to_string();
    let discovered_issuer = doc["issuer"]
        .as_str()
        .ok_or("missing issuer in OIDC discovery")?
        .to_string();

    Ok(OidcProvider {
        authorization_endpoint,
        token_endpoint,
        issuer: discovered_issuer,
    })
}

/// Generates a random URL-safe string for state/nonce parameters.
fn random_string() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, bytes)
}

/// Decodes the payload section of a JWT without verifying the signature.
/// Safe for confidential clients that received the token directly from the IdP over HTTPS.
fn decode_jwt_payload(jwt: &str) -> Result<serde_json::Value, String> {
    let parts: Vec<&str> = jwt.split('.').collect();
    if parts.len() != 3 {
        return Err("invalid JWT format".to_string());
    }
    let payload_bytes = base64::Engine::decode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, parts[1])
        .or_else(|_| {
            // Try with standard base64 (some IdPs use it)
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD_NO_PAD, parts[1])
        })
        .map_err(|e| format!("JWT base64 decode failed: {e}"))?;
    serde_json::from_slice(&payload_bytes).map_err(|e| format!("JWT payload parse failed: {e}"))
}

/// Maps OIDC claims to a Kronforce role using the configured claim path and value lists.
fn map_role(claims: &serde_json::Value, config: &OidcConfig) -> ApiKeyRole {
    // Navigate to the claim using dot-notation path
    let mut current = claims;
    for key in config.role_claim.split('.') {
        match current.get(key) {
            Some(v) => current = v,
            None => return config.default_role,
        }
    }

    // Collect claim values (handle both string and array)
    let values: Vec<String> = match current {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => return config.default_role,
    };

    // Check admin values first, then operator
    for val in &values {
        if config.admin_values.iter().any(|a| a == val) {
            return ApiKeyRole::Admin;
        }
    }
    for val in &values {
        if config.operator_values.iter().any(|o| o == val) {
            return ApiKeyRole::Operator;
        }
    }

    config.default_role
}

// --- Endpoint Handlers ---

/// Returns whether OIDC is configured (for frontend to show SSO button).
pub(crate) async fn oidc_config(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "enabled": state.oidc.is_some(),
    }))
}

/// Initiates the OIDC login flow by redirecting to the IdP.
pub(crate) async fn oidc_login(
    State(state): State<AppState>,
) -> Result<Response, AppError> {
    let oidc = state
        .oidc
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("OIDC not configured".into()))?;

    let state_param = random_string();
    let nonce = random_string();
    let now = Utc::now();

    // Store state+nonce for CSRF validation on callback
    let s = state_param.clone();
    let n = nonce.clone();
    db_call(&state.db, move |db| db.insert_auth_state(&s, &n, now)).await?;

    let query = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("response_type", "code")
        .append_pair("client_id", &oidc.config.client_id)
        .append_pair("redirect_uri", &oidc.config.redirect_uri)
        .append_pair("scope", &oidc.config.scopes)
        .append_pair("state", &state_param)
        .append_pair("nonce", &nonce)
        .finish();
    let auth_url = format!("{}?{}", oidc.provider.authorization_endpoint, query);

    Ok(Redirect::temporary(&auth_url).into_response())
}

#[derive(Deserialize)]
pub(crate) struct OidcCallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// Handles the OIDC callback: validates state, exchanges code, creates session.
pub(crate) async fn oidc_callback(
    State(state): State<AppState>,
    Query(params): Query<OidcCallbackParams>,
) -> Result<Response, AppError> {
    // Check for IdP error
    if let Some(err) = params.error {
        let desc = params.error_description.unwrap_or_default();
        warn!("OIDC login error: {} — {}", err, desc);
        return Err(AppError::BadRequest(format!("OIDC login failed: {err}")));
    }

    let code = params
        .code
        .ok_or_else(|| AppError::BadRequest("missing authorization code".into()))?;
    let state_param = params
        .state
        .ok_or_else(|| AppError::BadRequest("missing state parameter".into()))?;

    let oidc = state
        .oidc
        .as_ref()
        .ok_or_else(|| AppError::Internal("OIDC not configured".into()))?;

    // Validate CSRF state
    let sp = state_param.clone();
    let auth_state = db_call(&state.db, move |db| db.consume_auth_state(&sp)).await?;
    let auth_state =
        auth_state.ok_or_else(|| AppError::BadRequest("invalid or expired state parameter".into()))?;

    // Exchange authorization code for tokens
    let client = reqwest::Client::new();
    let token_resp = client
        .post(&oidc.provider.token_endpoint)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &oidc.config.redirect_uri),
            ("client_id", &oidc.config.client_id),
            ("client_secret", &oidc.config.client_secret),
        ])
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("token exchange failed: {e}")))?;

    if !token_resp.status().is_success() {
        let body = token_resp.text().await.unwrap_or_default();
        error!("OIDC token exchange failed: {}", body);
        return Err(AppError::Internal("token exchange failed".into()));
    }

    let token_data: serde_json::Value = token_resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("token response parse failed: {e}")))?;

    let id_token_str = token_data["id_token"]
        .as_str()
        .ok_or_else(|| AppError::Internal("no id_token in token response".into()))?;

    // Decode ID token claims
    let claims = decode_jwt_payload(id_token_str)
        .map_err(|e| AppError::Internal(format!("ID token decode failed: {e}")))?;

    // Validate nonce
    if let Some(token_nonce) = claims["nonce"].as_str()
        && token_nonce != auth_state.nonce
    {
        return Err(AppError::BadRequest("nonce mismatch".into()));
    }

    // Extract user info from claims
    let email = claims["email"]
        .as_str()
        .unwrap_or("unknown@unknown")
        .to_string();
    let name = claims["name"]
        .as_str()
        .or_else(|| claims["preferred_username"].as_str())
        .or_else(|| claims["email"].as_str())
        .unwrap_or("Unknown User")
        .to_string();

    // Map role from claims
    let role = map_role(&claims, &oidc.config);

    info!(
        "OIDC login: {} ({}) mapped to role {:?}",
        email, name, role
    );

    // Create session
    let raw_session_id = random_string();
    let session_hash = hash_api_key(&raw_session_id);
    let now = Utc::now();
    let expires_at = now + Duration::seconds(oidc.config.session_ttl_secs as i64);

    let session = OidcSession {
        id_hash: session_hash,
        user_email: email,
        user_name: name,
        role,
        id_token_claims: claims.to_string(),
        created_at: now,
        expires_at,
        last_active_at: now,
    };

    db_call(&state.db, move |db| db.insert_session(&session)).await?;

    // Build Set-Cookie header
    let cookie = cookie::Cookie::build(("kf_session", raw_session_id))
        .http_only(true)
        .same_site(cookie::SameSite::Lax)
        .path("/")
        .max_age(cookie::time::Duration::seconds(
            oidc.config.session_ttl_secs as i64,
        ))
        .build();

    // Redirect to dashboard
    let mut resp = Redirect::temporary("/").into_response();
    resp.headers_mut().insert(
        axum::http::header::SET_COOKIE,
        cookie.to_string().parse().unwrap(),
    );

    Ok(resp)
}
