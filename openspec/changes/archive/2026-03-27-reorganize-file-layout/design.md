## Context

The codebase has three structural problems: `executor/local.rs` (1,412 lines) contains 14 task type implementations in one file, `db/models.rs` (652 lines) contains 50+ types spanning unrelated domains, and `lib.rs` has confusing re-export aliases that make module ownership unclear. All `crate::X` import paths that rely on these re-exports need to be updated.

## Goals / Non-Goals

**Goals:**
- Split `executor/local.rs` so each task type lives in its own file under `executor/tasks/`
- Split `db/models.rs` into domain-specific files under `db/models/`
- Remove re-export aliases from `lib.rs` and update all import paths to canonical locations
- Keep all public API surface identical — downstream code using `crate::models::*` should still work via `db::models` mod re-exports
- All tests pass without modification

**Non-Goals:**
- Changing any function signatures, types, or behavior
- Reorganizing the API module (acceptable size, clear structure)
- Creating new abstractions or traits for task dispatch
- Splitting `db/executions.rs` or `executor/notifications.rs` (acceptable size)

## Decisions

### 1. Use `executor/tasks/` subdirectory with a `mod.rs` re-exporting everything

Create `src/executor/tasks/mod.rs` that declares submodules and re-exports all public items. The `run_task()` function stays in `local.rs` as the dispatcher, but each task type function moves to its own file.

**Why:** This preserves the existing `pub use local::run_task` export from `executor/mod.rs`. Each task file is self-contained with its own imports. The `tasks/mod.rs` re-exports mean `local.rs` only needs `use super::tasks::*` to call all task functions.

**Alternative considered:** Putting task files directly in `executor/` (e.g., `executor/shell.rs`). Rejected because it would mix task implementations with executor-level concerns (dispatch, notifications, scripts).

### 2. Use `db/models/` subdirectory with wildcard re-exports

Convert `db/models.rs` into `db/models/mod.rs` that declares submodules and does `pub use task::*; pub use job::*;` etc. This means all existing `use crate::models::*` imports continue to work unchanged.

**Why:** Zero impact on downstream code. The `pub use` in `mod.rs` makes the split invisible to consumers. Each domain file (task.rs, job.rs, etc.) contains only related types.

**Split boundaries:**
- `task.rs`: TaskType, TaskTypeDefinition, TaskFieldDefinition, FtpDirection, FtpProtocol
- `job.rs`: Job, JobStatus, ScheduleKind, CronExpression, Dependency, AgentTarget, OutputRules, ExtractionRule, AssertionRule, TriggerRule, EventTriggerConfig, JobNotificationConfig
- `execution.rs`: ExecutionRecord, ExecutionStatus, TriggerSource
- `agent.rs`: Agent, AgentStatus, AgentType
- `event.rs`: Event, EventSeverity
- `auth.rs`: ApiKey, ApiKeyRole
- `variable.rs`: Variable

### 3. Remove lib.rs re-exports and update all import paths

Remove these lines from `lib.rs`:
```rust
pub use agent::protocol;
pub use db::models;
pub use executor::notifications;
pub use executor::output_rules;
pub use executor::scripts;
pub use scheduler::cron_parser;
```

Update all call sites to use canonical paths:
- `crate::protocol::X` → `crate::agent::protocol::X`
- `crate::models::X` → `crate::db::models::X` (but `use crate::models::*` already works via `db::models` mod)
- `crate::notifications::X` → `crate::executor::notifications::X`
- `crate::output_rules::X` → `crate::executor::output_rules::X`
- `crate::scripts::X` → `crate::executor::scripts::X`
- `crate::cron_parser::X` → `crate::scheduler::cron_parser::X`

**Why:** The re-exports create ambiguity about module ownership. Using canonical paths makes it immediately clear where code lives.

### 4. Order of work: models → tasks → re-exports

Do models first (simplest, no function moves), then tasks (largest but contained), then re-exports (touches many files but mechanical find-replace).

**Why:** Each step is independently compilable and testable. Models split has zero downstream impact due to re-exports. Tasks split is self-contained in the executor module. Re-exports are the riskiest (many files) so they go last when everything else is stable.

## Risks / Trade-offs

- **Merge conflicts with in-flight work** → Low risk; this is structural, not behavioral. Conflicts would be in import paths which are easy to resolve.
- **Missed import update** → Mitigated by `cargo check` after each step. The compiler will catch any broken paths.
- **Larger diff** → Unavoidable for file moves. Each step is independently reviewable.
- **New files increase count** → ~15 new files, but each is focused and navigable. Net reduction in cognitive load.
