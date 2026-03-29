## Why

The Kronforce API has no rate limiting. Any client — authenticated or not — can send unlimited requests, making the system vulnerable to accidental or deliberate abuse: brute-force API key guessing, denial-of-service through excessive job triggers, or resource exhaustion via rapid-fire polling. Rate limiting is a baseline security control that should be in place before production deployment.

## What Changes

- Add per-IP rate limiting on unauthenticated endpoints (health, login) to prevent brute-force attacks
- Add per-API-key rate limiting on authenticated endpoints to prevent abuse by compromised or misconfigured clients
- Return standard `429 Too Many Requests` responses with `Retry-After` header when limits are exceeded
- Rate limit state held in-memory (no external dependency like Redis) — suitable for single-controller deployments
- Configurable limits via environment variables with sensible defaults
- Agent endpoints (heartbeat, queue polling, callbacks) get a higher limit since they are expected to be high-frequency

## Capabilities

### New Capabilities
- `api-rate-limiting`: In-memory rate limiting middleware for the Axum HTTP layer, including per-IP and per-API-key strategies, 429 response handling, configurable limits, and agent endpoint exemptions

### Modified Capabilities

## Impact

- **Backend**: New rate limiting middleware added to `src/api/mod.rs` router layers. New `src/api/rate_limit.rs` module for the limiter implementation. New fields in `src/config.rs` for limit configuration.
- **Dependencies**: New crate `governor` (token-bucket rate limiter) or hand-rolled sliding window counter — to be decided in design.
- **API behavior**: Clients exceeding limits receive `429 Too Many Requests` with a `Retry-After` header. This is a new error response that API consumers need to handle.
- **No database changes**: Rate limit state is in-memory only.
- **No breaking changes**: Existing clients operating within normal bounds are unaffected.
