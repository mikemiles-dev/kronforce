## Why

After the first code quality refactor, several significant patterns remain: `run_task()` is 334 lines with a 12-arm match, 66 `spawn_blocking` calls repeat identical boilerplate across every API handler, notification logic is duplicated between local executor and callbacks, dynamic query building is copy-pasted 4 times, and multi-step DB operations lack transaction safety.

## What Changes

- **`db_call()` helper** — generic async wrapper for `db.clone() + spawn_blocking + error mapping`, eliminating boilerplate in all API handlers
- **Notification dedup** — extract shared execution notification logic into a reusable function
- **Dynamic query builder** — extract filter/pagination building into a shared helper for jobs and executions
- **Transaction wrappers** — add transaction support for multi-step operations (dispatch, delete)
- **TaskExecutor per-type functions** — split the 334-line `run_task()` match into individual functions per task type

## Capabilities

### New Capabilities
- `db-helpers`: Shared database call patterns, query builders, and transaction wrappers

### Modified Capabilities

## Impact

- **src/api/*.rs** — all handlers simplified via `db_call()`
- **src/executor/local.rs** — `run_task()` split into per-type functions, notification logic extracted
- **src/api/callbacks.rs** — notification logic replaced with shared function
- **src/db/*.rs** — query builder helper, transaction support added
- **No external behavior changes**
