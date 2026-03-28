## Context

Seven functions across the scheduler, executor, and API modules have grown to 60-200 lines with deep nesting and mixed responsibilities. The cron parser's `next_after()` is the worst at 195 lines with 4+ nesting levels and repeated time-reset patterns. The executor's `execute_local()` runs at 158 lines, mixing execution lifecycle with output rule processing, notifications, and event logging in nested spawned tasks. Auth middleware has two nearly identical implementations.

## Goals / Non-Goals

**Goals:**
- Reduce maximum function length to ~50 lines or fewer per extracted function
- Eliminate 3+ levels of nesting by extracting inner logic into named functions
- Remove duplicated patterns (time resets in cron, auth middleware logic)
- Preserve all existing behavior — every refactored function must produce identical results
- Keep all existing tests passing without modification

**Non-Goals:**
- Adding new abstractions like traits or generics where simple function extraction suffices
- Changing any public API signatures or response formats
- Refactoring database query patterns or the `db_call` helper
- Optimizing performance — this is purely about readability
- Changing the `run_task()` match dispatch; while large, each arm is a simple delegation

## Decisions

### 1. Extract helper methods on existing structs rather than introducing new types

For `CronSchedule::next_after()`, add private helper methods on `CronSchedule` and `FieldSpec` rather than creating new wrapper structs. The repeated pattern of "find next match, if none advance to next unit and reset lower units" can be expressed as a method.

**Why:** Keeps the refactoring minimal. A `TimeCarryover` struct would add indirection without reducing the actual logic. Helper methods on the existing struct are easier to understand in context.

**Alternative considered:** State machine pattern — rejected because the cron matching algorithm is inherently imperative and a state machine would add complexity without improving clarity.

### 2. Extract post-execution processing into a standalone async function

For `execute_local()`, extract the spawned closure's body (output rules, notifications, event logging) into `async fn handle_execution_complete(db, exec, sched_tx)`.

**Why:** The inner closure is 120+ lines doing 4 distinct things. Extracting it makes each responsibility testable and the main function readable. The function takes ownership of the cloned values it needs, matching the existing ownership pattern.

### 3. Split `dispatch_to_specific_agent()` by agent type

Create `dispatch_via_queue()` for custom agents and `dispatch_via_http()` for standard agents, called from the existing function.

**Why:** The two paths share only the execution record creation. After that, they diverge completely (queue insert vs. HTTP POST + response handling). Splitting makes each path independently understandable.

### 4. Extract shared auth middleware into a common function

Create `validate_bearer_token(db, headers) -> Result<Option<ApiKey>>` used by both middleware functions. Each middleware calls this, then applies its own permission check.

**Why:** The token extraction, hashing, lookup, and last-used update are identical between the two middlewares. Only the permission check differs (agent role vs. any role).

### 5. Extract dependency evaluation from `build_job_response()`

Create `fn evaluate_deps(db, deps) -> (bool, Vec<DepStatus>)` to isolate the dependency satisfaction logic.

**Why:** The closure with mutable `all_satisfied` and nested database lookups is the main source of complexity. Extracting it makes `build_job_response()` a simple assembly function.

### 6. Extract cancel logic from `handle_command()`

Create `async fn cancel_execution(...)` for the `CancelExecution` match arm, which has nested if-lets checking local vs. remote cancellation.

**Why:** This arm is 37 lines with 3 levels of nesting. Other arms are 1-5 lines. Extracting it makes the match statement scannable.

## Risks / Trade-offs

- **Accidental behavior change** → Mitigated by keeping all existing tests and running `cargo test` after each refactored function. Each extraction should be a pure move with no logic changes.
- **Increased function count** → Acceptable trade-off for reduced per-function complexity. Each new function has a single clear purpose.
- **Merge conflicts with in-flight work** → Low risk since these are internal implementation changes, not API surface changes.
