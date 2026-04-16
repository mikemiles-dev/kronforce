use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use axum::extract::Request;
use axum::http::HeaderValue;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use serde_json::json;

/// Result of a rate limit check.
pub enum RateLimitResult {
    Allowed { remaining: u32 },
    Limited { retry_after_secs: u64 },
}

/// In-memory sliding window rate limiter.
#[derive(Clone)]
pub struct RateLimiter {
    state: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    max_requests: u32,
    window_secs: u64,
}

impl RateLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            max_requests,
            window_secs,
        }
    }

    /// Check if a request from `key` is allowed. Prunes expired entries first.
    pub fn check(&self, key: &str) -> RateLimitResult {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let window = std::time::Duration::from_secs(self.window_secs);

        let entries = state.entry(key.to_string()).or_default();

        // Prune expired
        entries.retain(|t| now.duration_since(*t) < window);

        if entries.len() >= self.max_requests as usize {
            // Find when the oldest entry in the window expires
            let oldest = entries.first().copied().unwrap_or(now);
            let retry_after = window
                .checked_sub(now.duration_since(oldest))
                .unwrap_or_default()
                .as_secs()
                + 1;
            RateLimitResult::Limited {
                retry_after_secs: retry_after,
            }
        } else {
            entries.push(now);
            let remaining = self.max_requests - entries.len() as u32;
            RateLimitResult::Allowed { remaining }
        }
    }

    /// Remove entries for clients with no activity in the last 2 minutes.
    pub fn cleanup(&self) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let now = Instant::now();
        let stale_threshold = std::time::Duration::from_secs(120);
        state.retain(|_, entries| {
            entries
                .last()
                .is_some_and(|t| now.duration_since(*t) < stale_threshold)
        });
    }

    pub fn max_requests(&self) -> u32 {
        self.max_requests
    }
}

/// Shared rate limiter state for all three tiers.
#[derive(Clone)]
pub struct RateLimiters {
    pub public: Option<RateLimiter>,
    pub authenticated: Option<RateLimiter>,
    pub agent: Option<RateLimiter>,
}

fn rate_limit_response(retry_after: u64, limit: u32, accepts_html: bool) -> Response {
    if accepts_html {
        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head><title>Rate Limited</title>
<meta name="viewport" content="width=device-width,initial-scale=1">
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', sans-serif; display: flex; align-items: center; justify-content: center; min-height: 100vh; margin: 0; background: #0d1117; color: #e6edf3; }}
  .card {{ text-align: center; padding: 48px; max-width: 420px; }}
  h1 {{ font-size: 48px; margin: 0 0 16px; }}
  h2 {{ font-size: 20px; margin: 0 0 12px; color: #e6a817; }}
  p {{ color: #8b949e; line-height: 1.6; margin: 0 0 24px; }}
  .retry {{ font-size: 14px; color: #58a6ff; font-weight: 500; }}
  button {{ background: #238636; color: #fff; border: none; padding: 10px 24px; border-radius: 6px; font-size: 14px; cursor: pointer; margin-top: 16px; }}
  button:hover {{ background: #2ea043; }}
</style>
</head>
<body>
<div class="card">
  <h1>&#9202;</h1>
  <h2>Slow Down</h2>
  <p>You're sending requests too quickly. Please wait a moment and try again.</p>
  <p class="retry">Retry after <strong>{retry_after}</strong> second{plural}</p>
  <button onclick="location.reload()">Try Again</button>
</div>
</body>
</html>"#,
            retry_after = retry_after,
            plural = if retry_after == 1 { "" } else { "s" },
        );
        let mut resp = (
            axum::http::StatusCode::TOO_MANY_REQUESTS,
            [(axum::http::header::CONTENT_TYPE, "text/html; charset=utf-8")],
            html,
        )
            .into_response();
        let headers = resp.headers_mut();
        headers.insert("Retry-After", HeaderValue::from(retry_after));
        headers.insert("X-RateLimit-Limit", HeaderValue::from(limit));
        headers.insert("X-RateLimit-Remaining", HeaderValue::from(0u32));
        resp
    } else {
        let body =
            json!({"error": format!("rate limit exceeded, retry after {} seconds", retry_after)});
        let mut resp =
            (axum::http::StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
        let headers = resp.headers_mut();
        headers.insert("Retry-After", HeaderValue::from(retry_after));
        headers.insert("X-RateLimit-Limit", HeaderValue::from(limit));
        headers.insert("X-RateLimit-Remaining", HeaderValue::from(0u32));
        resp
    }
}

fn add_rate_limit_headers(resp: &mut Response, limit: u32, remaining: u32) {
    let headers = resp.headers_mut();
    headers.insert("X-RateLimit-Limit", HeaderValue::from(limit));
    headers.insert("X-RateLimit-Remaining", HeaderValue::from(remaining));
}

/// Extract client IP from X-Forwarded-For header or direct connection.
fn extract_client_ip(req: &Request) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            req.extensions()
                .get::<axum::extract::connect_info::ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        })
}

/// Check if the request prefers HTML (browser navigation vs API call).
fn accepts_html(req: &Request) -> bool {
    req.headers()
        .get(axum::http::header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v.contains("text/html"))
}

/// Rate limit middleware for public endpoints (keyed by client IP).
pub async fn rate_limit_public_middleware(req: Request, next: Next) -> Response {
    let limiters = req.extensions().get::<RateLimiters>().cloned();
    let limiter = limiters.as_ref().and_then(|l| l.public.as_ref());
    let Some(limiter) = limiter else {
        return next.run(req).await;
    };

    let ip = extract_client_ip(&req);
    let html = accepts_html(&req);
    let limit = limiter.max_requests();
    match limiter.check(&ip) {
        RateLimitResult::Limited { retry_after_secs } => {
            rate_limit_response(retry_after_secs, limit, html)
        }
        RateLimitResult::Allowed { remaining } => {
            let mut resp = next.run(req).await;
            add_rate_limit_headers(&mut resp, limit, remaining);
            resp
        }
    }
}

/// Rate limit middleware for authenticated endpoints.
/// Keyed by API key UUID when available, falls back to client IP (e.g. demo mode).
pub async fn rate_limit_authed_middleware(req: Request, next: Next) -> Response {
    let limiters = req.extensions().get::<RateLimiters>().cloned();
    let limiter = limiters.as_ref().and_then(|l| l.authenticated.as_ref());
    let Some(limiter) = limiter else {
        return next.run(req).await;
    };

    // Use API key UUID if present, otherwise fall back to IP
    let key = req
        .extensions()
        .get::<crate::db::models::ApiKey>()
        .map(|k| k.id.to_string())
        .unwrap_or_else(|| extract_client_ip(&req));

    let html = accepts_html(&req);
    let limit = limiter.max_requests();
    match limiter.check(&key) {
        RateLimitResult::Limited { retry_after_secs } => {
            rate_limit_response(retry_after_secs, limit, html)
        }
        RateLimitResult::Allowed { remaining } => {
            let mut resp = next.run(req).await;
            add_rate_limit_headers(&mut resp, limit, remaining);
            resp
        }
    }
}

/// Rate limit middleware for agent endpoints (keyed by API key UUID, higher limit).
pub async fn rate_limit_agent_middleware(req: Request, next: Next) -> Response {
    let limiters = req.extensions().get::<RateLimiters>().cloned();
    let limiter = limiters.as_ref().and_then(|l| l.agent.as_ref());
    let Some(limiter) = limiter else {
        return next.run(req).await;
    };

    let key = req
        .extensions()
        .get::<crate::db::models::ApiKey>()
        .map(|k| k.id.to_string())
        .unwrap_or_else(|| extract_client_ip(&req));

    let html = accepts_html(&req);
    let limit = limiter.max_requests();
    match limiter.check(&key) {
        RateLimitResult::Limited { retry_after_secs } => {
            rate_limit_response(retry_after_secs, limit, html)
        }
        RateLimitResult::Allowed { remaining } => {
            let mut resp = next.run(req).await;
            add_rate_limit_headers(&mut resp, limit, remaining);
            resp
        }
    }
}
