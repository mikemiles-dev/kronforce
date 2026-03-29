## Context

Kronforce uses Axum with middleware layers for authentication. The router in `src/api/mod.rs` has three route groups: `public` (health, dashboard), `authed` (user API endpoints with `auth_middleware`), and `agent_authed` (agent endpoints with `agent_auth_middleware`). There is currently no rate limiting on any route.

The system runs as a single controller process with an in-memory SQLite connection. There is no Redis or external state store, and the project avoids external dependencies when possible.

## Goals / Non-Goals

**Goals:**
- Protect against brute-force API key attacks on authenticated endpoints
- Prevent accidental DoS from misconfigured clients or runaway scripts
- Return standard HTTP 429 responses with Retry-After headers
- Keep configuration simple via environment variables
- Zero external dependencies for rate limit state

**Non-Goals:**
- Distributed rate limiting across multiple controller instances (single-process only)
- Per-endpoint granular limits (one limit per category is sufficient)
- Rate limit dashboard or metrics endpoint
- IP allowlisting/blocklisting
- Request body size limiting (already handled by Axum defaults)

## Decisions

### 1. Hand-rolled sliding window counter instead of `governor` crate

Use a simple in-memory `HashMap<Key, Vec<Instant>>` sliding window implementation wrapped in `Arc<Mutex<>>`. On each request, prune expired entries and check the count.

**Rationale:** The `governor` crate is well-designed but adds a dependency tree (governor, nonzero_ext, parking_lot, etc.) for what amounts to ~50 lines of code. A sliding window counter is trivial to implement and test for our single-process use case.

**Alternatives considered:**
- `governor` crate: Good API but unnecessary dependency weight
- `tower::RateLimit`: Per-connection, not per-key — wrong abstraction
- Token bucket: Slightly more complex than sliding window, no benefit for our burst patterns

### 2. Three rate limit tiers via Axum middleware layers

| Tier | Scope | Default Limit | Applied To |
|------|-------|--------------|------------|
| **Public** | Per source IP | 30 req/min | `/api/health`, `/` |
| **Authenticated** | Per API key ID | 120 req/min | All `/api/*` authed routes |
| **Agent** | Per API key ID | 600 req/min | Agent register, heartbeat, queue, callbacks |

Each tier is an Axum `from_fn` middleware applied as a `route_layer` on its respective router group.

**Rationale:** Agent endpoints legitimately have high request rates (heartbeat every 10s + queue polling). Authenticated user endpoints need moderate limits. Public endpoints need tight limits to prevent scanning.

**Alternatives considered:**
- Single global limit: Too restrictive for agents, too permissive for public
- Per-endpoint limits: Over-engineered — three tiers cover all use cases

### 3. Key extraction strategy

- **Public tier**: Key is the client IP extracted from `ConnectInfo` or `X-Forwarded-For` header (configurable)
- **Authenticated tier**: Key is the API key UUID from the request extensions (set by auth middleware)
- **Agent tier**: Key is the API key UUID (same as authenticated, different limit)

The rate limit middleware runs *after* auth middleware, so the API key is already available in request extensions.

### 4. Configuration via environment variables

```
KRONFORCE_RATE_LIMIT_PUBLIC=30        # requests per minute for public endpoints
KRONFORCE_RATE_LIMIT_AUTHENTICATED=120 # requests per minute for authed endpoints
KRONFORCE_RATE_LIMIT_AGENT=600        # requests per minute for agent endpoints
KRONFORCE_RATE_LIMIT_ENABLED=true     # master switch to disable rate limiting
```

Setting any limit to `0` disables that tier. Setting `KRONFORCE_RATE_LIMIT_ENABLED=false` disables all rate limiting.

### 5. 429 response format

```json
{
  "error": "rate limit exceeded, retry after 15 seconds"
}
```

With headers:
- `Retry-After: 15` (seconds until the window resets)
- `X-RateLimit-Limit: 120` (the tier's limit)
- `X-RateLimit-Remaining: 0` (requests remaining in window)

### 6. Periodic cleanup of stale entries

A background `tokio::spawn` task runs every 60 seconds to prune rate limit entries older than the window. This prevents unbounded memory growth from unique IPs/keys that never return.

## Risks / Trade-offs

- **In-memory state lost on restart** → Acceptable. Rate limits reset on controller restart, which is fine — restarts are rare and the window is only 1 minute.
- **Single Mutex contention** → At realistic request rates (<1000 req/s), mutex contention on the rate limit map is negligible. If it becomes an issue, switch to `DashMap`.
- **No distributed support** → If Kronforce ever supports multiple controller instances, this needs to move to Redis or a shared store. Documented as a known limitation.
- **X-Forwarded-For spoofing for public tier** → An attacker behind a proxy can rotate IPs. Mitigation: in production, the reverse proxy (nginx, etc.) should handle IP-based rate limiting. Kronforce's public tier is a secondary defense.
