## 1. Split db/models.rs into domain files

- [x] 1.1 Create `src/db/models/` directory and `mod.rs` with submodule declarations and `pub use` re-exports
- [x] 1.2 Move task-related types (TaskType, TaskTypeDefinition, TaskFieldDefinition, FtpDirection, FtpProtocol) to `src/db/models/task.rs`
- [x] 1.3 Move job-related types (Job, JobStatus, ScheduleKind, CronExpression, Dependency, AgentTarget, OutputRules, ExtractionRule, AssertionRule, TriggerRule, EventTriggerConfig, JobNotificationConfig) to `src/db/models/job.rs`
- [x] 1.4 Move execution types (ExecutionRecord, ExecutionStatus, TriggerSource) to `src/db/models/execution.rs`
- [x] 1.5 Move agent types (Agent, AgentStatus, AgentType) to `src/db/models/agent.rs`
- [x] 1.6 Move event types (Event, EventSeverity) to `src/db/models/event.rs`
- [x] 1.7 Move auth types (ApiKey, ApiKeyRole) to `src/db/models/auth.rs`
- [x] 1.8 Move Variable to `src/db/models/variable.rs`
- [x] 1.9 Delete the old `src/db/models.rs` file and run `cargo check`

## 2. Split executor/local.rs into task-type modules

- [x] 2.1 Create `src/executor/tasks/mod.rs` with submodule declarations and re-exports
- [x] 2.2 Move `run_shell_task()` and shell helpers to `src/executor/tasks/shell.rs`
- [x] 2.3 Move `run_sql_task()` and SQL helpers to `src/executor/tasks/sql.rs`
- [x] 2.4 Move `run_ftp_task()` and FTP helpers to `src/executor/tasks/ftp.rs`
- [x] 2.5 Move `run_http_task()` and HTTP helpers to `src/executor/tasks/http.rs`
- [x] 2.6 Move messaging tasks (run_kafka_task, run_rabbitmq_task, run_mqtt_task, run_redis_task) to `src/executor/tasks/messaging.rs`
- [x] 2.7 Move `run_file_push_task()` to `src/executor/tasks/file_push.rs`
- [x] 2.8 Move `run_script_task()` to `src/executor/tasks/script.rs`
- [x] 2.9 Update `local.rs` to import from `tasks::*` and keep only `run_task()`, `execute_local()`, shared types, and post-execution functions
- [x] 2.10 Run `cargo check` to verify task split compiles

## 3. Clean up lib.rs re-exports and update import paths

- [x] 3.1 Remove `pub use` re-export aliases from `src/lib.rs`
- [x] 3.2 Update all `crate::protocol::` imports to `crate::agent::protocol::`
- [x] 3.3 Update all `crate::models::` imports to `crate::db::models::`
- [x] 3.4 Update all `crate::notifications::` imports to `crate::executor::notifications::`
- [x] 3.5 Update all `crate::output_rules::` imports to `crate::executor::output_rules::`
- [x] 3.6 Update all `crate::scripts::` imports to `crate::executor::scripts::`
- [x] 3.7 Update all `crate::cron_parser::` imports to `crate::scheduler::cron_parser::`
- [x] 3.8 Run `cargo check` to verify all import paths resolve

## 4. Final Verification

- [x] 4.1 Run full `cargo test` suite to confirm all tests pass
- [x] 4.2 Run `cargo clippy` to ensure no new warnings
