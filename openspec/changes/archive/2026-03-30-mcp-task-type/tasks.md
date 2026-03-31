## 1. Dependencies

- [x] 1.1 ~~Add `rmcp` crate~~ Dropped rmcp — implemented raw JSON-RPC directly (no new dependencies)
- [x] 1.2 Verify `cargo check` compiles

## 2. Model — TaskType::Mcp Variant

- [x] 2.1 Add `McpTransport` enum (`Stdio`, `Http`) with serde rename_all to `src/db/models/task.rs`
- [x] 2.2 Add `Mcp { server: String, transport: McpTransport, tool: String, arguments: Option<serde_json::Value> }` variant to `TaskType` enum
- [x] 2.3 Verify existing task serialization tests still pass (the new variant is additive)

## 3. MCP Client Module

- [x] 3.1 Create `src/executor/tasks/mcp.rs` with `run_mcp_task` function
- [x] 3.2 Implement stdio transport: spawn server command via cross-platform shell, pipe stdin/stdout for JSON-RPC
- [x] 3.3 Implement the MCP initialization handshake: send `initialize`, receive response, send `notifications/initialized`
- [x] 3.4 Implement `tools/call`: send tool call request with name and arguments, await response
- [x] 3.5 Implement result mapping: join text content blocks as stdout, map `isError` to failed status
- [x] 3.6 Implement HTTP transport: POST JSON-RPC messages to server URL via reqwest
- [x] 3.7 Implement timeout wrapping via tokio::select
- [x] 3.8 Implement cancellation via cancel_rx in tokio::select
- [x] 3.9 Register the module in `src/executor/tasks/mod.rs`

## 4. Executor Integration

- [x] 4.1 Add `TaskType::Mcp` match arm in `run_task()` in `src/executor/local.rs`
- [x] 4.2 Variable substitution works on MCP arguments (handled by generic JSON substitution)

## 5. Tool Discovery API

- [x] 5.1 Create `src/api/mcp.rs` with `mcp_discover_tools` handler for `GET /api/mcp/tools`
- [x] 5.2 Implement discovery: connect to server, handshake, call `tools/list`, return tools array
- [x] 5.3 Register the `/api/mcp/tools` route in `src/api/mod.rs`
- [x] 5.4 Register the `mcp` module in `src/api/mod.rs`

## 6. Frontend — MCP Task Form

- [x] 6.1 Add "MCP Tool" option to the task type selector in the job create/edit modal
- [x] 6.2 Create the MCP task form panel: transport radios, server input, Discover Tools button, tool select, arguments textarea
- [x] 6.3 Implement `discoverMcpTools()` JS function that calls `GET /api/mcp/tools` and populates the tool dropdown
- [x] 6.4 Implement dynamic argument form (textarea with JSON input for now)
- [x] 6.5 Wire MCP form into `populateTaskForm()` (load) and `buildTaskFromForm()` (save)

## 7. Documentation

- [x] 7.1 Add MCP task type to the in-app docs page under Task Types
- [x] 7.2 Add MCP section to `docs/API.md`
- [x] 7.3 Add MCP to the in-app API Reference table

## 8. Verify

- [x] 8.1 Run `cargo check` and `cargo test` — 236 tests pass
- [x] 8.2 Run `cargo clippy` — zero warnings
- [ ] 8.3 Manually test with a simple MCP server
