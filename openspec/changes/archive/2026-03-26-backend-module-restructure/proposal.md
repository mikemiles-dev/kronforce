## Why

The backend source is a flat collection of large files: `api.rs` (1334 lines), `db.rs` (1442 lines), and `executor.rs` (1122 lines) each contain multiple responsibilities. As features have grown (custom agents, output rules, settings, task type definitions), these files have become harder to navigate and modify. All source files sit flat in `src/` with no logical grouping.

## What Changes

- **Split `api.rs`** into a folder module `src/api/` with submodules:
  - `mod.rs` — router assembly and shared state
  - `jobs.rs` — job CRUD, trigger, listing handlers
  - `executions.rs` — execution listing, detail, cancel handlers
  - `agents.rs` — agent registration, listing, task types, queue polling, heartbeat
  - `events.rs` — event listing handler
  - `auth.rs` — middleware, API key management, auth_me
  - `settings.rs` — settings CRUD
  - `scripts.rs` — script CRUD handlers
  - `callbacks.rs` — execution result callback handler

- **Split `db.rs`** into a folder module `src/db/` with submodules:
  - `mod.rs` — Db struct, connection, migrations
  - `jobs.rs` — job insert/update/delete/get/list/count queries
  - `executions.rs` — execution insert/update/get/list/count queries
  - `agents.rs` — agent upsert/get/list/expire/heartbeat/task_types queries
  - `queue.rs` — job queue enqueue/dequeue/complete/stale cleanup
  - `events.rs` — event insert/list/count
  - `keys.rs` — API key CRUD
  - `settings.rs` — settings get/set/purge
  - `helpers.rs` — row_to_job, row_to_execution, row_to_agent, row_to_api_key helpers

- **Split `executor.rs`** into a folder module `src/executor/` with submodules:
  - `mod.rs` — Executor struct, execute entry point
  - `local.rs` — local execution (run_task, run_command, run_http, run_script)
  - `dispatch.rs` — agent dispatch (dispatch_to_agent, dispatch_to_any, dispatch_to_all, dispatch_to_tagged)
  - `output.rs` — post-execution output rules processing

- **Group agent code** into `src/agent/`:
  - `client.rs` (from agent_client.rs)
  - `server.rs` (from agent_server.rs)
  - `mod.rs` — re-exports

- **Keep small files in place**: `models.rs`, `config.rs`, `error.rs`, `protocol.rs`, `cron_parser.rs`, `dag.rs`, `scripts.rs`, `output_rules.rs` stay as-is (all under 430 lines)

## Capabilities

### New Capabilities
- `module-structure`: Reorganized backend source into folder modules with clear separation of concerns

### Modified Capabilities

## Impact

- **All Rust source files**: api.rs, db.rs, executor.rs, agent_client.rs, agent_server.rs are replaced by folder modules
- **lib.rs**: Updated module declarations
- **bin/*.rs**: Import paths may change
- **No functional changes**: Pure refactor — all behavior, APIs, and tests remain identical
- **No database changes**: No migrations
