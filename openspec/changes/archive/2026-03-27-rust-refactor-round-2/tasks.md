## 1. Generic db_call() Helper

- [x] 1.1 Add `pub async fn db_call<F, T>()` to `src/db/mod.rs`
- [x] 1.2 Replace `spawn_blocking` boilerplate in `src/api/jobs.rs` with `db_call()`
- [x] 1.3 Replace in `src/api/agents.rs`
- [x] 1.4 Replace in `src/api/executions.rs`
- [x] 1.5 Replace in `src/api/events.rs`
- [x] 1.6 Replace in `src/api/settings.rs`
- [x] 1.7 Replace in `src/api/scripts.rs`
- [x] 1.8 Replace in `src/api/auth.rs`
- [x] 1.9 Replace in `src/api/variables.rs`
- [x] 1.10 Replace in `src/api/callbacks.rs`

## 2. Notification Dedup

- [x] 2.1 Add `notify_execution_complete()` to `executor/notifications.rs` — takes Db, exec status, job notification config, job name, stderr, and sends notification if warranted
- [x] 2.2 Replace notification logic in `executor/local.rs` with call to shared function
- [x] 2.3 Replace notification logic in `api/callbacks.rs` with call to shared function

## 3. Dynamic Query Builder

- [x] 3.1 Add `QueryFilters` struct with `add_status()`, `add_search()`, `where_sql()` to `db/helpers.rs`
- [x] 3.2 Refactor `count_jobs()` and `list_jobs()` in `db/jobs.rs` to use `QueryFilters`
- [x] 3.3 Refactor `count_all_executions()` and `list_all_executions()` in `db/executions.rs` to use `QueryFilters`

## 4. Transaction Wrapper

- [x] 4.1 Add `with_transaction()` method to `Db` struct
- [x] 4.2 Wrap `delete_job()` in a transaction (dependency check + delete)
- [x] 4.3 Wrap `dispatch_to_specific_agent()` execution insert + queue enqueue in a transaction

## 5. Split run_task() into Per-Type Functions

- [x] 5.1 Extract `run_shell_task()` from the Shell match arm
- [x] 5.2 Extract `run_sql_task()` from the Sql match arm
- [x] 5.3 Extract `run_ftp_task()` from the Ftp match arm
- [x] 5.4 Extract `run_http_task()` from the Http match arm (rename existing `run_http()`)
- [x] 5.5 Extract `run_file_push_task()` from the FilePush match arm
- [x] 5.6 Extract `run_kafka_task()` from the Kafka match arm
- [x] 5.7 Extract `run_rabbitmq_task()` from the Rabbitmq match arm
- [x] 5.8 Extract `run_mqtt_task()` from the Mqtt match arm
- [x] 5.9 Extract `run_redis_task()` from the Redis match arm
- [x] 5.10 Verify `run_task()` is now a thin dispatcher under 50 lines

## 6. Verification

- [x] 6.1 `cargo test --all` passes
- [x] 6.2 `cargo clippy --all-targets` clean
- [x] 6.3 `cargo fmt --all -- --check` clean
