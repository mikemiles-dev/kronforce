## 1. Agent Module

- [x] 1.1 Add module-level `//!` doc to `src/agent/mod.rs`
- [x] 1.2 Add doc comments to all structs in `src/agent/protocol.rs` (AgentTask, TaskResult, HeartbeatPayload, etc.)
- [x] 1.3 Add doc comments to `AgentClient` struct and all methods in `src/agent/client.rs`
- [x] 1.4 Add doc comments to `AgentState` struct and route handlers in `src/agent/server.rs`

## 2. Database Module

- [x] 2.1 Add doc comments to all `Db` methods in `src/db/agents.rs`
- [x] 2.2 Add doc comments to all `Db` methods in `src/db/jobs.rs`
- [x] 2.3 Add doc comments to all `Db` methods in `src/db/executions.rs` (undocumented methods only)
- [x] 2.4 Add doc comments to all `Db` methods in `src/db/keys.rs`
- [x] 2.5 Add doc comments to all `Db` methods in `src/db/queue.rs`
- [x] 2.6 Add doc comments to all `Db` methods in `src/db/settings.rs`
- [x] 2.7 Add doc comments to all `Db` methods in `src/db/variables.rs`
- [x] 2.8 Add doc comments to `QueryFilters` struct and methods in `src/db/helpers.rs`
- [x] 2.9 Add doc comments to undocumented methods in `src/db/events.rs`

## 3. API Module

- [x] 3.1 Add module-level `//!` doc to `src/api/mod.rs` and doc comments to undocumented route handlers
- [x] 3.2 Add doc comments to all structs and functions in `src/api/agents.rs`
- [x] 3.3 Add doc comments to all structs and functions in `src/api/auth.rs`
- [x] 3.4 Add doc comments to `execution_result_callback` and structs in `src/api/callbacks.rs`
- [x] 3.5 Add doc comments to query structs and handlers in `src/api/events.rs`
- [x] 3.6 Add doc comments to query structs and handlers in `src/api/executions.rs`
- [x] 3.7 Add doc comments to request/response structs and handlers in `src/api/jobs.rs`
- [x] 3.8 Add doc comments to request structs and handlers in `src/api/scripts.rs`
- [x] 3.9 Add doc comments to functions in `src/api/settings.rs`
- [x] 3.10 Add doc comments to request structs and `validate_variable_name` in `src/api/variables.rs`

## 4. Executor Module

- [x] 4.1 Add doc comments to dispatch methods in `src/executor/dispatch.rs`
- [x] 4.2 Add doc comments to undocumented functions in `src/executor/local.rs`
- [x] 4.3 Add doc comments to `ScriptStore` struct and methods in `src/executor/scripts.rs`

## 5. Scheduler Module

- [x] 5.1 Add doc comments to `FieldSpec` enum, `parse_field`, and helper functions in `src/scheduler/cron_parser.rs`

## 6. Top-Level Module Docs

- [x] 6.1 Add `//!` module-level doc to `src/lib.rs`
