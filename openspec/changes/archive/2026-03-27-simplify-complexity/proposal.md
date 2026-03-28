## Why

Several core functions in the codebase have grown to 100-200 lines with deep nesting, mixed responsibilities, and duplicated patterns. This makes them harder to read, test, and modify safely. The worst offenders are in the cron parser, executor, and scheduler — the most critical paths in the system.

## What Changes

- Refactor `CronSchedule::next_after()` (195 lines, 4+ nesting levels) by extracting repeated time-reset patterns and field-matching logic into helper methods
- Refactor `parse_field()` (73 lines) by splitting step, range, and list parsing into separate functions
- Refactor `Executor::execute_local()` (158 lines) by extracting post-execution processing (output rules, notifications, event logging) into a dedicated async function
- Refactor `dispatch_to_specific_agent()` (104 lines) by splitting custom agent queue dispatch from standard agent HTTP dispatch
- Refactor `build_job_response()` by extracting dependency evaluation into its own function
- Refactor `Scheduler::handle_command()` by extracting the complex `CancelExecution` logic into a dedicated function
- Extract shared auth middleware logic from `auth_middleware()` and `agent_auth_middleware()` into a common helper

## Capabilities

### New Capabilities

_(none — this is a refactor with no new behavioral capabilities)_

### Modified Capabilities

_(none — all changes are implementation-only, no spec-level behavior changes)_

## Impact

- `src/scheduler/cron_parser.rs` — `next_after()` and `parse_field()` refactored
- `src/executor/local.rs` — `execute_local()` refactored, post-execution logic extracted
- `src/executor/dispatch.rs` — `dispatch_to_specific_agent()` split by agent type
- `src/api/jobs.rs` — `build_job_response()` decomposed
- `src/scheduler/mod.rs` — `handle_command()` cancel logic extracted
- `src/api/auth.rs` — shared middleware extraction
- No API, schema, or behavioral changes
- Existing tests must continue to pass unchanged
