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

fn rate_limit_response(retry_after: u64, limit: u32) -> Response {
    let body =
        json!({"error": format!("rate limit exceeded, retry after {} seconds", retry_after)});
    let mut resp = (axum::http::StatusCode::TOO_MANY_REQUESTS, axum::Json(body)).into_response();
    let headers = resp.headers_mut();
    headers.insert("Retry-After", HeaderValue::from(retry_after));
    headers.insert("X-RateLimit-Limit", HeaderValue::from(limit));
    headers.insert("X-RateLimit-Remaining", HeaderValue::from(0u32));
    resp
}

fn add_rate_limit_headers(resp: &mut Response, limit: u32, remaining: u32) {
    let headers = resp.headers_mut();
    headers.insert("X-RateLimit-Limit", HeaderValue::from(limit));
    headers.insert("X-RateLimit-Remaining", HeaderValue::from(remaining));
}

/// Rate limit middleware for public endpoints (keyed by client IP).
pub async fn rate_limit_public_middleware(req: Request, next: Next) -> Response {
    let limiters = req.extensions().get::<RateLimiters>().cloned();
    let limiter = limiters.as_ref().and_then(|l| l.public.as_ref());
    let Some(limiter) = limiter else {
        return next.run(req).await;
    };

    // Extract client IP from X-Forwarded-For or ConnectInfo
    let ip = req
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| {
            req.extensions()
                .get::<axum::extract::connect_info::ConnectInfo<std::net::SocketAddr>>()
                .map(|ci| ci.0.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    let limit = limiter.max_requests();
    match limiter.check(&ip) {
        RateLimitResult::Limited { retry_after_secs } => {
            rate_limit_response(retry_after_secs, limit)
        }
        RateLimitResult::Allowed { remaining } => {
            let mut resp = next.run(req).await;
            add_rate_limit_headers(&mut resp, limit, remaining);
            resp
        }
    }
}

/// Rate limit middleware for authenticated endpoints (keyed by API key UUID).
pub async fn rate_limit_authed_middleware(req: Request, next: Next) -> Response {
    let limiters = req.extensions().get::<RateLimiters>().cloned();
    let limiter = limiters.as_ref().and_then(|l| l.authenticated.as_ref());
    let Some(limiter) = limiter else {
        return next.run(req).await;
    };

    let key = req
        .extensions()
        .get::<crate::db::models::ApiKey>()
        .map(|k| k.id.to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    let limit = limiter.max_requests();
    match limiter.check(&key) {
        RateLimitResult::Limited { retry_after_secs } => {
            rate_limit_response(retry_after_secs, limit)
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
        .unwrap_or_else(|| "anonymous".to_string());

    let limit = limiter.max_requests();
    match limiter.check(&key) {
        RateLimitResult::Limited { retry_after_secs } => {
            rate_limit_response(retry_after_secs, limit)
        }
        RateLimitResult::Allowed { remaining } => {
            let mut resp = next.run(req).await;
            add_rate_limit_headers(&mut resp, limit, remaining);
            resp
        }
    }
}
