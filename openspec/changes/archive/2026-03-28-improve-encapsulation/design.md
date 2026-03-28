## Context

The codebase has several modules where free functions logically belong as methods on existing structs. The most prominent examples are agent server handlers operating on `AgentState`, database row converters that should be `from_row()` constructors, post-execution functions that belong on `Executor`, and job response helpers that belong on `JobResponse`. Moving them into `impl` blocks is idiomatic Rust and improves discoverability.

## Goals / Non-Goals

**Goals:**
- Move handler functions in `agent/server.rs` into `impl AgentState`
- Move `row_to_*` converters from `db/helpers.rs` into `from_row()` associated functions on model types
- Move `build_job_response()` and helpers into `impl JobResponse`
- Move post-execution free functions into `impl Executor`
- Move `event_matches()` and `pattern_matches()` into `impl Scheduler`
- Preserve all existing behavior — identical outputs for all inputs

**Non-Goals:**
- Creating new abstractions, traits, or wrapper types
- Changing function signatures beyond `self` parameter addition
- Refactoring the output_rules or notifications modules (acceptable as module-level utilities)
- Moving `validate_variable_name()` (only called once, acceptable as-is)

## Decisions

### 1. Agent server handlers: keep as free functions, not impl methods

Axum route handlers work best as free functions with `State` extractors. Moving them into `impl AgentState` would require changing the routing syntax and loses no encapsulation since axum's `State` pattern already binds them. The `router()` function already constructs the router with state.

**Why:** Axum's design pattern expects free functions. Fighting the framework would add complexity. The handlers are already grouped in `server.rs` which provides sufficient encapsulation.

**Alternative considered:** `impl AgentState` methods — rejected because axum's `post(AgentState::method)` syntax requires `FromRequest` gymnastics that add boilerplate without benefit.

### 2. Row converters: move to `from_row()` associated functions on model types

Move each `row_to_*` function to a `from_row()` associated function in the corresponding model file under `db/models/`. Keep them `pub(crate)` since they're only used by the db module.

**Why:** `Job::from_row(row)` is more discoverable than `row_to_job(row)`. It follows Rust convention for constructors. The model files in `db/models/` are the natural home since they define the types.

**Complication:** The model files are in `db/models/` but `from_row` needs `rusqlite` types. This is acceptable since models already depend on `serde` — adding a `rusqlite` dependency for the conversion is a reasonable coupling within the db crate.

### 3. Job response helpers: move to `impl JobResponse`

Move `build_job_response()` → `JobResponse::from_job()`, `compute_next_fire()` → `JobResponse::compute_next_fire()`, `evaluate_deps()` → `JobResponse::evaluate_deps()`.

**Why:** All three functions exist solely to construct `JobResponse`. Making them associated functions groups them logically and makes the API clearer.

### 4. Post-execution functions: move to `impl Executor`

Move `handle_execution_complete()`, `run_output_rules()`, and `send_execution_notifications()` into the `Executor` impl block. They already take `&Db` and `&mpsc::Sender` which are fields on `Executor`, so they can use `&self` instead.

**Why:** These functions are exclusively called from `Executor::execute_local()`. Making them methods avoids passing `db` and `sched_tx` as parameters since they're already on `self`.

### 5. Event matching: move to `impl Scheduler`

Move `event_matches()` and `pattern_matches()` into `impl Scheduler` as private methods (they don't need `&self`, so they become associated functions).

**Why:** These are private implementation details of `Scheduler::handle_event()`. Encapsulating them clarifies scope.

### 6. Order of work: models → executor → scheduler → api

Start with model `from_row()` (most impactful, touches db helpers), then executor (self-contained), scheduler (small), and api jobs (small). Each step is independently compilable.

## Risks / Trade-offs

- **Axum handler compatibility** → Mitigated by Decision 1: keeping handlers as free functions avoids framework friction.
- **Circular dependency in model from_row** → Model files will need `rusqlite` in scope. This is acceptable since they're within the `db` module already.
- **Method count on Executor** → Adding 3 methods increases `Executor`'s surface, but they're private and clearly scoped to post-execution lifecycle.
