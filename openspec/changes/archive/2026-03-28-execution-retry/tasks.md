## 1. Database Migration

- [x] 1.1 Create `migrations/0004_execution_retry.sql` that adds `retry_max INTEGER DEFAULT 0`, `retry_delay_secs INTEGER DEFAULT 0`, `retry_backoff REAL DEFAULT 1.0` to `jobs` table, and `retry_of TEXT`, `attempt_number INTEGER DEFAULT 1` to `executions` table

## 2. Backend — Models

- [x] 2.1 Add `retry_max: u32`, `retry_delay_secs: u64`, `retry_backoff: f64` fields to `Job` struct in `src/db/models/job.rs` with `#[serde(default)]`
- [x] 2.2 Update `Job::from_row` to read the three new retry columns (positions 16, 17, 18 after group_name at 15)
- [x] 2.3 Add `retry_of: Option<Uuid>` and `attempt_number: u32` fields to `ExecutionRecord` struct in `src/db/models/execution.rs` with defaults
- [x] 2.4 Update `ExecutionRecord::from_row` to read `retry_of` and `attempt_number` columns
- [x] 2.5 Add `Retry { original_execution_id: Uuid, attempt: u32 }` variant to `TriggerSource` enum

## 3. Backend — DB Queries

- [x] 3.1 Update `Db::insert_job` and `Db::update_job` in `src/db/jobs.rs` to include `retry_max`, `retry_delay_secs`, `retry_backoff` in INSERT/UPDATE queries
- [x] 3.2 Update all job SELECT queries to include the three retry columns
- [x] 3.3 Update `Db::insert_execution` in `src/db/executions.rs` to include `retry_of` and `attempt_number`
- [x] 3.4 Update execution SELECT queries to include `retry_of` and `attempt_number`

## 4. Backend — Retry Logic

- [x] 4.1 Add a `calculate_retry_delay(retry_delay_secs, retry_backoff, attempt)` helper function that returns the delay in seconds capped at 3600
- [x] 4.2 Add a `should_retry(job, execution_status, attempt_number)` helper that returns true only for Failed/TimedOut when attempt < retry_max + 1
- [x] 4.3 Add retry scheduling to `Executor::handle_execution_complete` in `src/executor/local.rs` — after notifications, check should_retry and spawn a delayed re-execution via SchedulerCommand::RetryExecution
- [x] 4.4 Add retry scheduling to the callback handler in `src/api/callbacks.rs` for agent-dispatched jobs — same logic as local

## 5. Backend — API

- [x] 5.1 Add `retry_max`, `retry_delay_secs`, `retry_backoff` to `CreateJobRequest` and `UpdateJobRequest` in `src/api/jobs.rs`
- [x] 5.2 Wire retry fields into `create_job` and `update_job` handlers
- [x] 5.3 Add `retry_of` and `attempt_number` to the execution API response (already serialized via serde)

## 6. Frontend — Job Modal

- [x] 6.1 Add retry config fields to the Advanced tab in `web/partials/modals.html`: max retries (number), retry delay (number, seconds), backoff multiplier (number)
- [x] 6.2 Wire retry fields into modal open (populate from job data) and save (include in request body) in `web/js/modals.js`

## 7. Frontend — Execution Display

- [x] 7.1 Show "Attempt N/M" badge on execution list items and detail view when `attempt_number > 1` in `web/js/executions.js`
- [x] 7.2 Show "Retry of <execution_id>" link in execution detail when `retry_of` is set

## 8. Test Updates

- [x] 8.1 Update `make_job` in all test files to include the new retry fields with defaults (`retry_max: 0, retry_delay_secs: 0, retry_backoff: 1.0`)
- [x] 8.2 Update `make_execution` or equivalent in test files to include `retry_of: None, attempt_number: 1`

## 9. Verify

- [x] 9.1 Run `cargo check` and `cargo test` to verify compilation and all existing tests pass
- [x] 9.2 Run `cargo clippy` to verify no new warnings
