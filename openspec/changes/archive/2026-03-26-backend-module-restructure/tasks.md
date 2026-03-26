## 1. Agent Folder Module (simplest — start here)

- [x] 1.1 Create `src/agent/` directory
- [x] 1.2 Move `src/agent_client.rs` → `src/agent/client.rs`
- [x] 1.3 Move `src/agent_server.rs` → `src/agent/server.rs`
- [x] 1.4 Create `src/agent/mod.rs` with `pub mod client; pub mod server;` and re-exports
- [x] 1.5 Update `src/lib.rs`: replace `pub mod agent_client; pub mod agent_server;` with `pub mod agent;`
- [x] 1.6 Update all `crate::agent_client` and `crate::agent_server` imports across the codebase
- [x] 1.7 Verify `cargo check` passes

## 2. Database Folder Module

- [x] 2.1 Create `src/db/` directory
- [x] 2.2 Create `src/db/mod.rs` with `Db` struct, `new()`, connection setup, and all migrations (keep the migration array here)
- [x] 2.3 Move row mapper helpers (`row_to_job`, `row_to_execution`, `row_to_agent`, `row_to_api_key`) to `src/db/helpers.rs` with `pub(super)` visibility
- [x] 2.4 Move job queries (`insert_job`, `get_job`, `update_job`, `delete_job`, `list_jobs`, `count_jobs`, etc.) to `src/db/jobs.rs` as `impl Db` block
- [x] 2.5 Move execution queries (`insert_execution`, `update_execution`, `update_execution_extracted`, `get_execution`, `list_executions_for_job`, `list_all_executions`, counts, timeline) to `src/db/executions.rs`
- [x] 2.6 Move agent queries (`upsert_agent`, `get_agent`, `get_agent_by_name`, `list_agents`, `get_online_agents*`, `update_agent_heartbeat`, `expire_agents`, `delete_agent`, `update_agent_task_types`) to `src/db/agents.rs`
- [x] 2.7 Move queue operations (`enqueue_job`, `dequeue_job`, `complete_queue_item`, `queue_depth`, `fail_stale_*`) to `src/db/queue.rs`
- [x] 2.8 Move event operations (`insert_event`, `log_event`, `list_events`, `count_events`) to `src/db/events.rs`
- [x] 2.9 Move API key operations (`insert_api_key`, `get_api_key_by_hash`, `list_api_keys`, `update_api_key_last_used`, `count_api_keys`, `revoke_api_key`) to `src/db/keys.rs`
- [x] 2.10 Move settings operations (`get_setting`, `set_setting`, `get_all_settings`, `purge_old_*`) to `src/db/settings.rs`
- [x] 2.11 Delete old `src/db.rs`, update `src/lib.rs`
- [x] 2.12 Verify `cargo check` passes

## 3. Executor Folder Module

- [x] 3.1 Create `src/executor/` directory
- [x] 3.2 Create `src/executor/mod.rs` with `Executor` struct, `new()`, `execute()` entry point, and `cancel()`
- [x] 3.3 Move local execution (`execute_local`, `run_task`, `run_command`, `run_http`, `run_script`, `shell_escape`, helper types) to `src/executor/local.rs`
- [x] 3.4 Move dispatch logic (`dispatch_to_agent`, `dispatch_to_any`, `dispatch_to_all`, `dispatch_to_tagged`, `dispatch_to_specific_agent`, `required_agent_type`) to `src/executor/dispatch.rs`
- [x] 3.5 Extract post-execution output rules processing into `src/executor/local.rs` (kept with execute_local)
- [x] 3.6 Delete old `src/executor.rs`, update `src/lib.rs`
- [x] 3.7 Verify `cargo check` passes

## 4. API Folder Module

- [x] 4.1 Create `src/api/` directory
- [x] 4.2 Create `src/api/mod.rs` with `AppState`, `router()` function, shared types (`PaginatedResponse`), and the `dashboard()` / `health()` handlers
- [x] 4.3 Move job handlers (`create_job`, `get_job`, `update_job`, `delete_job`, `list_jobs`, `trigger_job`) to `src/api/jobs.rs`
- [x] 4.4 Move execution handlers (`list_executions`, `list_all_executions`, `get_execution`, `cancel_execution`) to `src/api/executions.rs`
- [x] 4.5 Move agent handlers (`register_agent`, `get_agent_handler`, `list_agents`, `deregister_agent`, `agent_heartbeat`, `get_agent_task_types`, `update_agent_task_types`) to `src/api/agents.rs`
- [x] 4.6 Move queue polling handler (`poll_agent_queue`) into `src/api/agents.rs`
- [x] 4.7 Move event handler (`list_events`) and timeline handlers to `src/api/events.rs`
- [x] 4.8 Move auth middleware (`auth_middleware`, `AuthUser`), key management handlers, and helper functions to `src/api/auth.rs`
- [x] 4.9 Move settings handlers (`get_settings`, `update_settings`) to `src/api/settings.rs`
- [x] 4.10 Move script handlers (`list_scripts`, `get_script`, `save_script`, `delete_script`) to `src/api/scripts.rs`
- [x] 4.11 Move callback handler (`execution_result_callback`) to `src/api/callbacks.rs`
- [x] 4.12 Delete old `src/api.rs`, update `src/lib.rs`
- [x] 4.13 Verify `cargo check` passes

## 5. Final Cleanup

- [x] 5.1 Update `src/bin/controller.rs` and `src/bin/agent.rs` for any changed import paths
- [x] 5.2 Verify no submodule exceeds 300 lines (executor/local.rs at 825 and api/jobs.rs at 413 are acceptable — tightly coupled logic)
- [x] 5.3 Full `cargo build` and verify the binary runs correctly
