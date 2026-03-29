## 1. Rate Limiter Core

- [x] 1.1 Create `src/api/rate_limit.rs` with a `RateLimiter` struct holding `Arc<Mutex<HashMap<String, Vec<Instant>>>>` and a `max_requests` / `window_secs` config
- [x] 1.2 Implement `RateLimiter::check(&self, key: &str) -> RateLimitResult` that prunes expired entries, checks count, and returns either `Allowed { remaining }` or `Limited { retry_after_secs }`
- [x] 1.3 Implement `RateLimiter::cleanup(&self)` that removes all entries with no timestamps within the last 2 minutes

## 2. Configuration

- [x] 2.1 Add rate limit fields to `ControllerConfig` in `src/config.rs`: `rate_limit_enabled` (bool), `rate_limit_public` (u32), `rate_limit_authenticated` (u32), `rate_limit_agent` (u32) with defaults (true, 30, 120, 600)
- [x] 2.2 Parse `KRONFORCE_RATE_LIMIT_ENABLED`, `KRONFORCE_RATE_LIMIT_PUBLIC`, `KRONFORCE_RATE_LIMIT_AUTHENTICATED`, `KRONFORCE_RATE_LIMIT_AGENT` from environment

## 3. Middleware

- [x] 3.1 Create `rate_limit_public_middleware` in `src/api/rate_limit.rs` that extracts client IP from `ConnectInfo` or `X-Forwarded-For` header and calls `RateLimiter::check`
- [x] 3.2 Create `rate_limit_authed_middleware` that extracts the API key UUID from request extensions and calls `RateLimiter::check`
- [x] 3.3 Create `rate_limit_agent_middleware` that extracts the API key UUID from request extensions and calls a separate agent-tier `RateLimiter::check`
- [x] 3.4 All three middlewares SHALL return 429 JSON response with `Retry-After`, `X-RateLimit-Limit`, and `X-RateLimit-Remaining` headers on limit exceeded, and add `X-RateLimit-Limit` and `X-RateLimit-Remaining` headers on allowed requests

## 4. Router Integration

- [x] 4.1 Add `RateLimiterState` (holding three `RateLimiter` instances) to `AppState` or pass via Axum state extension
- [x] 4.2 Apply `rate_limit_public_middleware` as a layer on the `public` router in `src/api/mod.rs`
- [x] 4.3 Apply `rate_limit_authed_middleware` as a layer on the `authed` router in `src/api/mod.rs`
- [x] 4.4 Apply `rate_limit_agent_middleware` as a layer on the `agent_authed` router in `src/api/mod.rs`
- [x] 4.5 Skip all rate limit layers when `rate_limit_enabled` is false

## 5. Background Cleanup

- [x] 5.1 Spawn a background `tokio::spawn` task in `src/bin/controller.rs` that calls `cleanup()` on all three limiters every 60 seconds

## 6. Verify

- [x] 6.1 Register the `rate_limit` module in `src/api/mod.rs`
- [x] 6.2 Run `cargo check` and `cargo test` to verify compilation and all existing tests pass
- [x] 6.3 Run `cargo clippy` to verify no new warnings
