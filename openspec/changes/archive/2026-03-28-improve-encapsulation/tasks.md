## 1. Row Converters to Model from_row()

- [x] 1.1 Move `row_to_job()` from `src/db/helpers.rs` to `Job::from_row()` in `src/db/models/job.rs` and update all call sites
- [x] 1.2 Move `row_to_execution()` from `src/db/helpers.rs` to `ExecutionRecord::from_row()` in `src/db/models/execution.rs` and update all call sites
- [x] 1.3 Move `row_to_agent()` from `src/db/helpers.rs` to `Agent::from_row()` in `src/db/models/agent.rs` and update all call sites
- [x] 1.4 Move `row_to_api_key()` from `src/db/helpers.rs` to `ApiKey::from_row()` in `src/db/models/auth.rs` and update all call sites
- [x] 1.5 Move shared parse helpers (`parse_uuid`, `parse_datetime`, `parse_json`) to a shared location accessible by model files
- [x] 1.6 Run `cargo check` to verify row converter migration compiles

## 2. Executor Post-Execution Methods

- [x] 2.1 Move `handle_execution_complete()`, `run_output_rules()`, and `send_execution_notifications()` into `impl Executor` methods using `&self` for db/sched_tx access
- [x] 2.2 Run `cargo check` to verify executor encapsulation compiles

## 3. Scheduler Event Matching Methods

- [x] 3.1 Move `event_matches()` and `pattern_matches()` into `impl Scheduler` as private associated functions
- [x] 3.2 Run `cargo check` to verify scheduler encapsulation compiles

## 4. Job Response Builder

- [x] 4.1 Move `build_job_response()` to `JobResponse::from_job()`, and `compute_next_fire()` and `evaluate_deps()` to private associated functions on `JobResponse`
- [x] 4.2 Update all call sites from `build_job_response(job, db)` to `JobResponse::from_job(job, db)`
- [x] 4.3 Run `cargo check` to verify job response encapsulation compiles

## 5. Final Verification

- [x] 5.1 Run full `cargo test` suite to confirm all tests pass
- [x] 5.2 Run `cargo clippy` to ensure no new warnings
