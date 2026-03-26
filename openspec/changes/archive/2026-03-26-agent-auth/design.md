## Context

The API has two route groups in `api/mod.rs`: `authed` (with `auth_middleware` for API keys) and `public` (agent endpoints, health, dashboard). Agent endpoints are in the `public` group because agents don't have API keys.

The controller config is in `src/config.rs` (`ControllerConfig`). The agent config is in the same file (`AgentConfig`). Both read from environment variables.

## Goals / Non-Goals

**Goals:**
- Single shared secret for all agent communication
- Configured via environment variable on both controller and agent
- Opt-in — if not set, agent endpoints stay open
- Clear error messages when key is missing or wrong

**Non-Goals:**
- Per-agent keys or key rotation
- Encrypting the key at rest
- TLS enforcement (orthogonal concern)

## Decisions

### 1. Agent key as environment variable, not database setting

**Decision**: `KRONFORCE_AGENT_KEY` is an environment variable, not a database setting. The controller reads it on startup and stores it in `AppState`. The agent binary reads it and includes it in headers.

**Rationale**: Environment variables are the standard way to pass secrets. Database storage would require the key to bootstrap (chicken-and-egg). Env vars work with Docker, systemd, and cloud deployments.

### 2. Separate middleware for agent endpoints

**Decision**: Add `agent_auth_middleware` that checks `Authorization: Bearer <key>` against the configured agent key. Apply it to a new `agent_authed` route group that sits between `authed` and `public`.

Route groups become:
- `authed` — dashboard API endpoints (existing API key auth)
- `agent_authed` — agent endpoints (agent key auth, or open if no key set)
- `public` — health check, dashboard HTML only

```rust
let agent_authed = Router::new()
    .route("/api/agents/register", post(agents::register_agent))
    .route("/api/agent-queue/{agent_id}/next", get(agents::poll_agent_queue))
    .route("/api/agents/{id}/heartbeat", post(agents::agent_heartbeat))
    .route("/api/callbacks/execution-result", post(callbacks::execution_result_callback))
    .route("/api/agents/{id}/task-types", get(agents::get_agent_task_types))
    .route_layer(middleware::from_fn_with_state(state.clone(), agent_auth_middleware))
    .with_state(state.clone());
```

**Rationale**: Separates concerns cleanly. The existing API key middleware is untouched. The agent middleware is simpler (single key check vs. database lookup).

### 3. Middleware skips check if no key configured

**Decision**: `agent_auth_middleware` reads the agent key from `AppState`. If `None` (not configured), it passes the request through. If `Some(key)`, it checks the header.

**Rationale**: Backwards compatible. Existing deployments without the env var continue to work.

### 4. Agent key stored in AppState

**Decision**: Add `agent_key: Option<String>` to `AppState`. Set from `ControllerConfig` which reads `KRONFORCE_AGENT_KEY` from the environment.

**Rationale**: Available to the middleware without re-reading the environment on every request.

### 5. Standard agent reads key from environment

**Decision**: Add `agent_key: Option<String>` to `AgentConfig`, read from `KRONFORCE_AGENT_KEY`. The agent includes `Authorization: Bearer <key>` on all requests to the controller when set.

**Rationale**: Same env var name on both sides. Simple to configure — set the same value on controller and agent.

## Risks / Trade-offs

- **Single key for all agents** → If compromised, all agents are affected. Acceptable for a self-hosted tool. Per-agent keys can be added later.
- **Key in environment variable** → Visible in process lists on some systems. Standard practice for container deployments. Use secrets managers for production.
- **No key rotation** → Changing the key requires restarting controller and all agents. Acceptable for the simplicity gained.
