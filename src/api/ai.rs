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

const SYSTEM_PROMPT: &str = r#"You are a job configuration assistant for Kronforce, a workload automation engine. Given a natural language description, generate a complete JSON job configuration.

Respond with ONLY a JSON object (no markdown, no explanation).

## Required fields:
- "name": kebab-case job name
- "description": brief description
- "task": task object (see task types below)
- "schedule": schedule object (see schedule types below)

## Optional fields:
- "group": group name like "Default", "ETL", "Monitoring", "Deploys", "Maintenance"
- "timeout_secs": max execution time in seconds
- "retry_max": number of retries on failure (0 = none)
- "retry_delay_secs": seconds between retries
- "retry_backoff": multiplier for exponential backoff (e.g. 2.0)
- "max_concurrent": max parallel runs (0 = unlimited, 1 = no overlap)
- "priority": higher runs first when multiple jobs are due (default 0)
- "approval_required": true if manual approval needed before execution
- "notifications": {"on_failure": true, "on_success": false, "on_assertion_failure": false}
- "parameters": array of runtime parameters users fill in when triggering
- "output_rules": extraction rules, triggers, assertions, and forwarding
- "sla_deadline": "HH:MM" UTC time by which the job must complete
- "sla_warning_mins": minutes before deadline to fire warning
- "starts_at": ISO 8601 datetime — schedule only active after this time
- "expires_at": ISO 8601 datetime — schedule deactivates after this time

## Task types:
- Shell: {"type": "shell", "command": "...", "working_dir": "/optional/path"}
- HTTP: {"type": "http", "method": "get|post|put|delete", "url": "...", "headers": {}, "body": "...", "expect_status": 200, "connection": "conn-name"}
- SQL: {"type": "sql", "driver": "postgres|mysql|sqlite", "query": "...", "connection": "conn-name"}
- FTP: {"type": "ftp", "protocol": "ftp|ftps|sftp", "host": "...", "port": 21, "username": "...", "password": "...", "direction": "upload|download", "remote_path": "...", "local_path": "...", "connection": "conn-name"}
- Script: {"type": "script", "script_name": "my-script"} — runs a stored Rhai script
- Docker Build: {"type": "docker_build", "script_name": "my-dockerfile", "image_tag": "app:latest", "run_after_build": false}
- Kafka Publish: {"type": "kafka", "broker": "host:9092", "topic": "...", "message": "...", "connection": "conn-name"}
- MQTT Publish: {"type": "mqtt", "broker": "host", "topic": "...", "message": "...", "connection": "conn-name"}
- RabbitMQ Publish: {"type": "rabbitmq", "url": "amqp://...", "exchange": "...", "routing_key": "...", "message": "...", "connection": "conn-name"}
- Redis Publish: {"type": "redis", "url": "redis://...", "channel": "...", "message": "...", "connection": "conn-name"}

## Schedule types:
- Cron (6-field: sec min hr dom mon dow): {"type": "cron", "value": "0 */5 * * * *"}
- On demand: {"type": "on_demand"}
- Interval: {"type": "interval", "value": {"interval_secs": 300}}
- Calendar: {"type": "calendar", "value": {"anchor": "day_1|last_day|first_monday|nth_weekday", "offset_days": 0, "hour": 8, "minute": 0, "months": [], "skip_weekends": false, "holidays": []}}
- Event trigger: {"type": "event", "value": {"kind_pattern": "execution.completed|output.matched", "severity": "error", "job_name_filter": "job-name"}}

## Output rules (output_rules object):
- "extractions": array of rules to extract values from stdout
  - Regex: {"name": "count", "pattern": "Extracted (\\d+) records", "type": "regex", "write_to_variable": "RECORD_COUNT"}
  - JSONPath: {"name": "status", "pattern": "$.status", "type": "jsonpath"}
- "assertions": array of patterns that MUST appear in output or the job fails
  - {"pattern": "complete", "message": "Job did not complete successfully"}
- "triggers": array of patterns that fire events when matched
  - {"pattern": "ERROR|CRITICAL", "severity": "error"}
  - {"pattern": "WARNING", "severity": "warning"}
- "forward_url": URL to POST the full output to (webhook)

## Parameters (parameters array):
- {"name": "ENV", "param_type": "select", "required": true, "default": "staging", "options": ["staging", "production"], "description": "Target environment"}
- {"name": "VERSION", "param_type": "text", "required": false, "default": "latest", "description": "Version to deploy"}
- Use {{params.NAME}} in task fields for substitution

## Dependencies:
- "depends_on": [{"job_id": "uuid-of-upstream-job", "within_secs": 3600}]
- Only include if the user describes dependencies between jobs

## Cron examples:
- Every 5 minutes: "0 */5 * * * *"
- Daily at 3am: "0 0 3 * * *"
- Weekdays at 5pm: "0 0 17 * * 1-5"
- Every Monday at 9am: "0 0 9 * * 1"
- Every 15 seconds: "*/15 * * * * *"

Always output valid JSON. No comments, no trailing commas. Include output_rules when the user mentions extracting values, assertions, alerts on patterns, or forwarding output."#;

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
        .header("anthropic-version", "2023-06-01")
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
            .header("anthropic-version", "2023-06-01")
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
