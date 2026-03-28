## 1. New Tests — Cron Parser

- [x] 1.1 Test range expressions (e.g., `0 0 9-17 * * *`)
- [x] 1.2 Test step with range (e.g., `0 0 */3 * * *`)
- [x] 1.3 Test day-of-week scheduling
- [x] 1.4 Test monthly scheduling with specific day
- [x] 1.5 Test invalid expressions return errors

## 2. New Tests — Config

- [x] 2.1 Test ControllerConfig::from_env() with default values
- [x] 2.2 Test ControllerConfig::from_env() with custom env vars
- [x] 2.3 Test AgentConfig::from_env() defaults and overrides

## 3. New Tests — Error Handling

- [x] 3.1 Test AppError::NotFound maps to 404
- [x] 3.2 Test AppError::BadRequest maps to 400
- [x] 3.3 Test AppError::Unauthorized maps to 401
- [x] 3.4 Test AppError::Forbidden maps to 403
- [x] 3.5 Test AppError::Internal maps to 500
- [x] 3.6 Test AppError::Db maps to 500

## 4. New Tests — QueryFilters

- [x] 4.1 Test empty filters produce no WHERE clause
- [x] 4.2 Test add_status produces correct clause
- [x] 4.3 Test add_search with multiple columns
- [x] 4.4 Test add_limit_offset returns correct indices
- [x] 4.5 Test combined filters with AND

## 5. New Tests — Factory Methods

- [x] 5.1 Test ExecutionRecord::new() defaults
- [x] 5.2 Test ExecutionRecord builder chain (with_status, with_agent_id, etc.)
- [x] 5.3 Test ApiKey::bootstrap() with no preset (auto-generates)
- [x] 5.4 Test ApiKey::bootstrap() with preset key
- [x] 5.5 Test ApiKey::bootstrap() with short preset key (bounds-safe prefix)

## 6. New Tests — DAG

- [x] 6.1 Test deps_satisfied() with all dependencies met
- [x] 6.2 Test deps_satisfied() with unmet dependencies
- [x] 6.3 Test deps_satisfied() with time window constraints

## 7. New Tests — process_post_execution Integration

- [x] 7.1 Test extraction + variable write-back via process_post_execution with in-memory DB
- [x] 7.2 Test assertion failure updates execution status
- [x] 7.3 Test trigger generates events
- [x] 7.4 Test job with no output_rules returns empty events

## 8. Doc Comments — Models

- [x] 8.1 Add doc comments to all pub structs in src/db/models.rs (Job, Agent, ExecutionRecord, Event, ApiKey, Variable, etc.)
- [x] 8.2 Add doc comments to all pub enums in src/db/models.rs (TaskType, ScheduleKind, JobStatus, ExecutionStatus, etc.)
- [x] 8.3 Add doc comments to pub impl methods (ApiKey::bootstrap, ExecutionRecord::new, status as_str/from_str)

## 9. Doc Comments — Core Modules

- [x] 9.1 Add doc comments to src/db/mod.rs (Db struct, open, migrate, with_transaction, db_call)
- [x] 9.2 Add doc comments to src/executor/mod.rs (Executor struct, execute, cancel, substitute_variables)
- [x] 9.3 Add doc comments to src/executor/notifications.rs (all pub structs and functions)
- [x] 9.4 Add doc comments to src/config.rs (ControllerConfig, AgentConfig, from_env)
- [x] 9.5 Add doc comments to src/error.rs (AppError enum and variants)
- [x] 9.6 Add doc comments to src/dag.rs (DagResolver, deps_satisfied)
- [x] 9.7 Add doc comments to src/scheduler/mod.rs (Scheduler, SchedulerCommand)
- [x] 9.8 Add doc comments to src/api/mod.rs (AppState, router, PaginatedResponse)

## 10. Verification

- [x] 10.1 `cargo test --all` passes
- [x] 10.2 `cargo clippy --all-targets` clean
- [x] 10.3 `cargo doc --no-deps` builds without warnings
