## 1. MCP Server Core

- [x] 1.1 Create `src/mcp_server.rs` module with JSON-RPC request/response types, SSE response formatting, and session ID generation
- [x] 1.2 Implement the `mcp_handler` Axum POST handler that deserializes JSON-RPC messages and dispatches to `handle_initialize`, `handle_tools_list`, `handle_tools_call`, or returns 202 for notifications
- [x] 1.3 Implement `handle_initialize` that returns server info and capabilities
- [x] 1.4 Implement SSE response formatting: `Content-Type: text/event-stream` with `event: message\ndata: {...}\n\n`
- [x] 1.5 Implement `Accept` header validation (require `application/json` and `text/event-stream`)
- [x] 1.6 Implement `Mcp-Session-Id` generation on initialize and pass-through on subsequent requests

## 2. Tool Definitions

- [x] 2.1 Define tool registry with name, description, JSON Schema input, and minimum role for each tool
- [x] 2.2 Implement `handle_tools_list` that filters tools by the caller's API key role and returns tool definitions
- [x] 2.3 Define input schemas for all 10 tools: list_jobs, get_job, create_job, trigger_job, list_executions, get_execution, list_agents, list_groups, list_events, get_system_stats

## 3. Tool Handlers — Read-Only

- [x] 3.1 Implement `list_jobs` handler: accepts optional group, status, search; returns job summaries via `Db::list_jobs`
- [x] 3.2 Implement `get_job` handler: accepts name or id; returns full job details via `Db::get_job`
- [x] 3.3 Implement `list_executions` handler: accepts optional status, limit; returns recent executions via `Db::list_all_executions`
- [x] 3.4 Implement `get_execution` handler: accepts id; returns full execution with output via `Db::get_execution`
- [x] 3.5 Implement `list_agents` handler: returns all agents via `Db::list_agents`
- [x] 3.6 Implement `list_groups` handler: returns group names via `Db::get_distinct_groups`
- [x] 3.7 Implement `list_events` handler: accepts optional limit; returns recent events via `Db::list_events`
- [x] 3.8 Implement `get_system_stats` handler: returns job/execution/agent/group counts

## 4. Tool Handlers — Mutations

- [x] 4.1 Implement `create_job` handler: accepts name, task, schedule, optional group/description/timeout; creates job via existing logic and returns job ID
- [x] 4.2 Implement `trigger_job` handler: accepts name or id; sends `SchedulerCommand::TriggerNow` and returns confirmation

## 5. Router Integration

- [x] 5.1 Add `mcp_server` module to `src/lib.rs`
- [x] 5.2 Register `POST /mcp` route on the authenticated router in `src/api/mod.rs`
- [ ] 5.3 Add `mcp_enabled: bool` to `ControllerConfig` with `KRONFORCE_MCP_ENABLED` env var (default true)
- [ ] 5.4 Conditionally register the `/mcp` route based on config

## 6. Documentation

- [ ] 6.1 Add MCP Server section to in-app docs explaining how to connect MCP clients
- [ ] 6.2 Add MCP Server section to docs/API.md with endpoint details and tool list
- [ ] 6.3 Update README to mention MCP server capability
- [ ] 6.4 Update CHANGELOG

## 7. Verify

- [x] 7.1 Run `cargo check` and `cargo test` — all tests pass
- [x] 7.2 Run `cargo clippy` — zero warnings
- [ ] 7.3 Test with the MCP test client: connect, discover tools, call list_jobs
