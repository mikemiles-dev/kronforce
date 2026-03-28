## Context

The codebase has 76 raw SQL queries across 9 DB files, 66 `spawn_blocking` calls with identical boilerplate, a 334-line match statement in `run_task()`, duplicated notification logic, and no transaction isolation for multi-step operations.

## Goals / Non-Goals

**Goals:**
- Reduce API handler boilerplate with a generic DB call wrapper
- Consolidate duplicated notification sending logic
- Extract dynamic query filter building into a shared helper
- Add transaction support for atomic multi-step operations
- Split `run_task()` into focused per-task-type functions

**Non-Goals:**
- Introducing an ORM
- Changing any API responses or external behavior
- Adding new features

## Decisions

### 1. Generic `db_call()` helper

Add to `src/db/mod.rs`:

```rust
pub async fn db_call<F, T>(db: &Db, f: F) -> Result<T, AppError>
where
    F: FnOnce(&Db) -> Result<T, AppError> + Send + 'static,
    T: Send + 'static,
{
    let db = db.clone();
    tokio::task::spawn_blocking(move || f(&db))
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?
}
```

API handlers change from:
```rust
let db = state.db.clone();
let result = tokio::task::spawn_blocking(move || db.list_jobs(...)).await.unwrap()?;
```
To:
```rust
let result = db_call(&state.db, |db| db.list_jobs(...)).await?;
```

### 2. Notification helper function

Add `notify_execution_complete()` to `executor/notifications.rs` that takes `Db`, execution status, job notification config, job name, stdout/stderr, and handles the should-notify logic, subject/body formatting, and send call. Both `local.rs` and `callbacks.rs` call this single function.

### 3. Dynamic query builder helper

Add a `QueryFilters` struct and builder to `db/helpers.rs`:

```rust
pub(super) struct QueryFilters {
    pub where_clauses: Vec<String>,
    pub params: Vec<String>,
}

impl QueryFilters {
    pub fn new() -> Self { ... }
    pub fn add_status(&mut self, status: &str) { ... }
    pub fn add_search(&mut self, query: &str, columns: &[&str]) { ... }
    pub fn where_sql(&self) -> String { ... }
}
```

Used by `count_jobs`, `list_jobs`, `count_all_executions`, `list_all_executions`.

### 4. Transaction wrapper on Db

Add `with_transaction()` to `Db`:

```rust
pub fn with_transaction<F, T>(&self, f: F) -> Result<T, AppError>
where
    F: FnOnce(&rusqlite::Transaction) -> Result<T, AppError>,
{
    let mut conn = self.conn.lock().unwrap();
    let tx = conn.transaction().map_err(AppError::Db)?;
    let result = f(&tx)?;
    tx.commit().map_err(AppError::Db)?;
    Ok(result)
}
```

Applied to: `delete_job` (check deps + delete), `dispatch_to_specific_agent` (insert execution + enqueue).

### 5. Per-task-type execution functions

Split `run_task()` match arms into individual functions:
- `run_shell_task()`, `run_sql_task()`, `run_ftp_task()`, `run_http_task()`
- `run_kafka_task()`, `run_rabbitmq_task()`, `run_mqtt_task()`, `run_redis_task()`
- `run_file_push_task()`

Each takes its specific variant fields and returns `CommandResult`. The `run_task()` function becomes a thin dispatcher. Script execution already has `run_script()`.

## Risks / Trade-offs

- **`db_call` closure ergonomics** — closures need `move` and owned values. Some call sites may need extra clones. → Accept this; the API simplification is worth it.
- **Transaction locking** — `with_transaction` holds the mutex for the entire transaction. → Already the case with current single-query locking. SQLite is single-writer anyway.
- **Large diff** — touching every API handler. → Changes are mechanical and individually simple.
