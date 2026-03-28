## 1. Factory Methods & Builders

- [x] 1.1 Add `ExecutionRecord::new(id, job_id, trigger)` constructor with defaults, plus builder methods `with_status()`, `with_agent_id()`, `with_task_snapshot()`
- [x] 1.2 Replace all inline `ExecutionRecord { ... }` construction in `executor/local.rs`, `executor/dispatch.rs`, and `api/callbacks.rs` with the new constructor
- [x] 1.3 Add `ApiKey::bootstrap(role, name, preset_key) -> (ApiKey, String)` factory that handles key generation, prefix extraction, and hashing
- [x] 1.4 Replace bootstrap key logic in `bin/controller.rs` (admin + agent blocks) and `create_admin_key()` with `ApiKey::bootstrap()`

## 2. Eliminate Duplicated Output Rules Processing

- [x] 2.1 Add `process_post_execution()` async function to `executor/output_rules.rs` that runs extractions, variable write-back, assertions, and triggers — returning `Vec<Event>`
- [x] 2.2 Refactor `executor/local.rs` post-execution block (lines ~90-183) to call `process_post_execution()`
- [x] 2.3 Refactor `api/callbacks.rs` post-execution block to call `process_post_execution()`
- [x] 2.4 Remove duplicated output rules code from both files

## 3. Safe String Indexing

- [x] 3.1 Add `const KEY_PREFIX_LEN: usize = 11` to `api/auth.rs` and use `.get(..KEY_PREFIX_LEN)` for prefix extraction
- [x] 3.2 Fix `api/auth.rs` `generate_api_key()` to use bounds-checked slicing
- [x] 3.3 Fix UUID short-string slicing in `executor/local.rs` and `api/callbacks.rs` to use `.get(..8)`
- [x] 3.4 Fix prefix extraction in `ApiKey::bootstrap()` factory to use bounds-checked slicing

## 4. DB Row Mapping Safety

- [x] 4.1 Replace `.unwrap()` calls on `row.get()` in `db/helpers.rs` `row_to_job()` with `?` operator
- [x] 4.2 Replace `.unwrap()` calls in `db/helpers.rs` `row_to_execution()` with `?` operator
- [x] 4.3 Replace `.unwrap()` calls in `db/helpers.rs` `row_to_agent()` with `?` operator
- [x] 4.4 Replace `.unwrap()` calls in `db/helpers.rs` `row_to_api_key()` with `?` operator
- [x] 4.5 Replace `Uuid::parse_str().unwrap()` and `serde_json::from_str().unwrap()` with error mapping in all row mapper functions

## 5. Silent Error Logging

- [x] 5.1 Replace `let _ = db.update_execution_extracted(...)` with `if let Err(e) = ... { tracing::warn!(...) }` in executor/local.rs
- [x] 5.2 Replace all other `let _ = db.*` silent failures in executor/local.rs post-execution block with warning logs
- [x] 5.3 Replace `let _ = db.*` silent failures in api/callbacks.rs with warning logs

## 6. spawn_blocking Error Propagation

- [x] 6.1 Add an `AppError::Internal(String)` variant if not already present
- [x] 6.2 Replace `.await.unwrap()?` on `spawn_blocking` calls with `.await.map_err(|e| AppError::Internal(e.to_string()))?` in API handlers

## 7. Named Constants

- [x] 7.1 Add `DEFAULT_SCRIPT_TIMEOUT_SECS` and `MAX_SCRIPT_OUTPUT_LINES` constants in `executor/local.rs`
- [x] 7.2 Replace bare magic numbers with the named constants
- [x] 7.3 Add `TIMESTAMP_FORMAT` constant and use it across files that format timestamps

## 8. Misc Cleanup

- [x] 8.1 Fix the `created_by: None` TODO in `api/jobs.rs` to use the auth context
- [x] 8.2 Clean up excessive `db.clone()` / `db2` naming in executor and callbacks — use descriptive names

## 9. Verification

- [x] 9.1 `cargo test --all` passes
- [x] 9.2 `cargo clippy --all-targets` has no warnings
- [x] 9.3 `cargo fmt --all -- --check` passes
