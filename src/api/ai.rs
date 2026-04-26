//! AI assistant endpoint — generates job configurations from natural language descriptions.

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use super::AppState;
use super::auth::AuthUser;
use crate::error::AppError;

#[derive(Deserialize)]
pub(crate) struct AiRequest {
    prompt: String,
}

#[derive(Serialize)]
pub(crate) struct AiResponse {
    pub job: serde_json::Value,
}

const SYSTEM_PROMPT: &str = r#"You are a job configuration assistant for Kronforce, a workload automation engine. Given a natural language description, generate a JSON job configuration.

Respond with ONLY a JSON object (no markdown, no explanation) with these fields:
- "name": kebab-case job name (required)
- "description": brief description (required)
- "group": group name like "Default", "ETL", "Monitoring", "Deploys", "Maintenance" (optional)
- "task": task object (required), one of:
  - {"type": "shell", "command": "..."} — shell command
  - {"type": "http", "method": "get|post|put|delete", "url": "...", "expect_status": 200} — HTTP request
  - {"type": "sql", "driver": "postgres|mysql|sqlite", "query": "...", "connection": "conn-name"} — SQL query
- "schedule": schedule object (required), one of:
  - {"type": "cron", "value": "sec min hr dom mon dow"} — 6-field cron (seconds first)
  - {"type": "on_demand"} — manual trigger only
  - {"type": "interval", "value": {"interval_secs": N}} — repeat N seconds after last completion
  - {"type": "calendar", "value": {"anchor": "day_1|last_day|first_monday|nth_weekday", "offset_days": 0, "hour": 8, "minute": 0, "months": [], "skip_weekends": false, "holidays": []}}
- "timeout_secs": number (optional)
- "notifications": {"on_failure": true, "on_success": false} (optional)
- "retry_max": number (optional, 0 = no retry)
- "connection": connection name for the task (optional, only if task uses external credentials)
- "parameters": array of {"name": "PARAM", "param_type": "text|select|number", "required": bool, "default": "value", "description": "..."} (optional)

Cron format: 6 fields = seconds minutes hours day-of-month month day-of-week. Use 0 for seconds unless sub-minute precision is needed. Examples:
- Every 5 minutes: "0 */5 * * * *"
- Daily at 3am: "0 0 3 * * *"
- Weekdays at 5pm: "0 0 17 * * 1-5"
- Every Monday at 9am: "0 0 9 * * 1"

Always output valid JSON. No comments, no trailing commas."#;

pub(crate) async fn ai_generate_job(
    State(state): State<AppState>,
    _auth: AuthUser,
    Json(req): Json<AiRequest>,
) -> Result<Json<AiResponse>, AppError> {
    // Check DB settings first, then fall back to env var config
    let db = state.db.clone();
    let (db_key, db_provider, db_model) = tokio::task::spawn_blocking(move || {
        let key = db.get_setting("ai_api_key").unwrap_or(None);
        let provider = db.get_setting("ai_provider").unwrap_or(None);
        let model = db.get_setting("ai_model").unwrap_or(None);
        (key, provider, model)
    })
    .await
    .unwrap_or((None, None, None));

    let api_key = db_key
        .filter(|k| !k.is_empty())
        .or_else(|| state.ai_api_key.clone())
        .ok_or_else(|| {
            AppError::BadRequest(
                "AI not configured — set an API key in Settings or KRONFORCE_AI_API_KEY".into(),
            )
        })?;

    let provider = db_provider
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| state.ai_provider.clone());

    let prompt = req.prompt.trim().to_string();
    if prompt.is_empty() {
        return Err(AppError::BadRequest("prompt cannot be empty".into()));
    }
    if prompt.len() > 2000 {
        return Err(AppError::BadRequest(
            "prompt too long (max 2000 chars)".into(),
        ));
    }

    let model = db_model
        .filter(|m| !m.is_empty())
        .or_else(|| state.ai_model.clone())
        .unwrap_or_else(|| {
            if provider == "openai" {
                "gpt-4o".to_string()
            } else {
                "claude-3-5-sonnet-latest".to_string()
            }
        });

    let client = reqwest::Client::new();
    let response_text = if provider == "openai" {
        call_openai(&client, &api_key, &model, &prompt).await?
    } else {
        call_anthropic(&client, &api_key, &model, &prompt).await?
    };

    // Parse the response as JSON
    let job: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|_| {
            // Try to extract JSON from the response if it has surrounding text
            extract_json(&response_text)
                .ok_or_else(|| AppError::Internal("AI returned invalid JSON".into()))
        })
        .or_else(|r| r)?;

    Ok(Json(AiResponse { job }))
}

async fn call_anthropic(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<String, AppError> {
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "system": SYSTEM_PROMPT,
        "messages": [{"role": "user", "content": prompt}]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2024-10-22")
        .header("content-type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("AI request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "AI API error {}: {}",
            status, body
        )));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("AI response parse error: {e}")))?;

    data["content"][0]["text"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| AppError::Internal("unexpected AI response format".into()))
}

async fn call_openai(
    client: &reqwest::Client,
    api_key: &str,
    model: &str,
    prompt: &str,
) -> Result<String, AppError> {
    let body = serde_json::json!({
        "model": model,
        "max_tokens": 1024,
        "messages": [
            {"role": "system", "content": SYSTEM_PROMPT},
            {"role": "user", "content": prompt}
        ]
    });

    let resp = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&body)
        .timeout(std::time::Duration::from_secs(30))
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("AI request failed: {e}")))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!(
            "AI API error {}: {}",
            status, body
        )));
    }

    let data: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| AppError::Internal(format!("AI response parse error: {e}")))?;

    data["choices"][0]["message"]["content"]
        .as_str()
        .map(String::from)
        .ok_or_else(|| AppError::Internal("unexpected AI response format".into()))
}

/// List available models for the configured AI provider.
pub(crate) async fn ai_list_models(
    State(state): State<AppState>,
    _auth: AuthUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let db = state.db.clone();
    let (db_key, db_provider) = tokio::task::spawn_blocking(move || {
        let key = db.get_setting("ai_api_key").unwrap_or(None);
        let provider = db.get_setting("ai_provider").unwrap_or(None);
        (key, provider)
    })
    .await
    .unwrap_or((None, None));

    let api_key = db_key
        .filter(|k| !k.is_empty())
        .or_else(|| state.ai_api_key.clone())
        .ok_or_else(|| AppError::BadRequest("no AI API key configured".into()))?;

    let provider = db_provider
        .filter(|p| !p.is_empty())
        .unwrap_or_else(|| state.ai_provider.clone());

    let client = reqwest::Client::new();

    if provider == "openai" {
        let resp = client
            .get("https://api.openai.com/v1/models")
            .header("authorization", format!("Bearer {}", api_key))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("request failed: {e}")))?;
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("parse error: {e}")))?;
        Ok(Json(data))
    } else {
        let resp = client
            .get("https://api.anthropic.com/v1/models")
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2024-10-22")
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("request failed: {e}")))?;
        let data: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| AppError::Internal(format!("parse error: {e}")))?;
        Ok(Json(data))
    }
}

/// Try to extract a JSON object from text that may have surrounding markdown/explanation.
fn extract_json(text: &str) -> Option<serde_json::Value> {
    // Try extracting from ```json ... ```
    if let Some(start) = text.find("```json") {
        let json_start = start + 7;
        if let Some(end) = text[json_start..].find("```") {
            let json_str = text[json_start..json_start + end].trim();
            if let Ok(v) = serde_json::from_str(json_str) {
                return Some(v);
            }
        }
    }
    // Try finding first { to last }
    if let Some(start) = text.find('{')
        && let Some(end) = text.rfind('}')
    {
        let json_str = &text[start..=end];
        if let Ok(v) = serde_json::from_str(json_str) {
            return Some(v);
        }
    }
    None
}
