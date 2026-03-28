## 1. Cron Parser

- [x] 1.1 Extract repeated time-reset logic in `CronSchedule::next_after()` into private helper methods on `CronSchedule`
- [x] 1.2 Split `parse_field()` into `parse_step_field()`, `parse_range_or_list()` helper functions
- [x] 1.3 Run `cargo test` to verify cron parser tests still pass

## 2. Executor

- [x] 2.1 Extract post-execution processing from `execute_local()` into `async fn handle_execution_complete()`
- [x] 2.2 Split `dispatch_to_specific_agent()` into `dispatch_via_queue()` (custom) and `dispatch_via_http()` (standard)
- [x] 2.3 Run `cargo test` to verify executor tests still pass

## 3. API Handlers

- [x] 3.1 Extract dependency evaluation from `build_job_response()` into `fn evaluate_deps()`
- [x] 3.2 Extract shared Bearer token validation from `auth_middleware()` and `agent_auth_middleware()` into `validate_bearer_token()`
- [x] 3.3 Run `cargo test` to verify API tests still pass

## 4. Scheduler

- [x] 4.1 Extract `CancelExecution` logic from `handle_command()` into a dedicated `async fn cancel_execution()`
- [x] 4.2 Run `cargo test` to verify scheduler tests still pass

## 5. Final Verification

- [x] 5.1 Run full `cargo test` suite to confirm all tests pass
- [x] 5.2 Run `cargo clippy` to ensure no new warnings introduced
