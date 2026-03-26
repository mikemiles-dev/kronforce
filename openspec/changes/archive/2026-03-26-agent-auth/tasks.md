## 1. Controller Config

- [x] 1.1 Add `agent_key: Option<String>` to `ControllerConfig` in `src/config.rs`, read from `KRONFORCE_AGENT_KEY`
- [x] 1.2 Add `agent_key: Option<String>` to `AppState` in `src/api/mod.rs`
- [x] 1.3 Pass `config.agent_key` to `AppState` in `src/bin/controller.rs`

## 2. Agent Auth Middleware

- [x] 2.1 Add `agent_auth_middleware` function in `src/api/auth.rs` — reads `AppState.agent_key`, checks Authorization header, passes through if no key configured
- [x] 2.2 Move agent endpoints from `public` to a new `agent_authed` route group in `src/api/mod.rs` with the agent auth middleware layer
- [x] 2.3 Keep health and dashboard in `public` (no auth)

## 3. Standard Agent Binary

- [x] 3.1 Add `agent_key: Option<String>` to `AgentConfig` in `src/config.rs`, read from `KRONFORCE_AGENT_KEY`
- [x] 3.2 Include `Authorization: Bearer <key>` header on registration request in `src/bin/agent.rs`
- [x] 3.3 Include the header on heartbeat requests
- [x] 3.4 The agent's HTTP client for callbacks already uses the callback URL — ensure the execution result POST includes the key header

## 4. Agent Server (Standard Agent)

- [x] 4.1 Update the agent server's callback POST in `src/agent/server.rs` to include the agent key header when reporting results back to the controller

## 5. Python Example

- [x] 5.1 Update `examples/custom_agent.py` to read `KRONFORCE_AGENT_KEY` from environment
- [x] 5.2 Include the key in registration, polling, and callback requests when set

## 6. Documentation

- [x] 6.1 Add `KRONFORCE_AGENT_KEY` to controller and agent config tables in README
- [x] 6.2 Update `docs/CUSTOM_AGENTS.md` protocol section to mention the agent key requirement
- [x] 6.3 Update the in-app docs (dashboard.html) Custom Agents section with auth info
- [x] 6.4 Update the wizard's agent step to mention setting the key
