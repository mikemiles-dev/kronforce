## Why

Several modules have standalone functions that logically belong as methods on existing structs. For example, `agent/server.rs` has free handler functions that operate on `AgentState`, `db/helpers.rs` has `row_to_*` converters that should be `from_row()` constructors on their respective model types, and `executor/local.rs` has post-execution functions that belong on `Executor`. Moving these into `impl` blocks clarifies ownership, improves discoverability, and makes the code more idiomatic Rust.

## What Changes

- Move agent server handler functions (`execute_job`, `cancel_job`, `shutdown`) into `impl AgentState` methods
- Move `build_job_response()`, `compute_next_fire()`, and `evaluate_deps()` into `impl JobResponse`
- Move `row_to_job()`, `row_to_execution()`, `row_to_agent()`, `row_to_api_key()` into `from_row()` associated functions on their respective model types
- Move `handle_execution_complete()`, `run_output_rules()`, `send_execution_notifications()` into `impl Executor` methods
- Move `event_matches()` and `pattern_matches()` into `impl Scheduler` private methods

## Capabilities

### New Capabilities

_(none — this is an encapsulation refactor with no new behavioral capabilities)_

### Modified Capabilities

_(none — all changes are structural, no spec-level behavior changes)_

## Impact

- `src/agent/server.rs` — handlers become methods on `AgentState`
- `src/api/jobs.rs` — helpers become methods on `JobResponse`
- `src/db/helpers.rs` — row converters move to model types (may need adjustments in db module files that call them)
- `src/executor/local.rs` — post-execution functions become `Executor` methods
- `src/scheduler/mod.rs` — event matching becomes `Scheduler` methods
- No API, schema, or behavioral changes
- Existing tests must continue to pass unchanged
