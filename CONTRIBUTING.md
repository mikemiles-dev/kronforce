# Contributing to Kronforce

## Coding Standards

These rules apply to all Rust code in the project. They exist to prevent the classes of bugs we've already had to clean up.

### Error Handling

**No `.unwrap()` in production code.** Use `?`, `.ok_or_else()`, `.unwrap_or_default()`, or `.unwrap_or()` instead.

```rust
// Bad
let conn = pool.get().unwrap();

// Good
let conn = pool.get().map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
```

**`.expect()` is allowed only during startup** — when the process should crash rather than run in a broken state (e.g., can't open the database, can't parse the config, can't bind the port).

```rust
// OK — startup, process should die if this fails
let db = Db::open(&config.db_path).expect("failed to open database");

// Not OK — request handler, should return an error
let job = db.get_job(id).expect("job not found");  // Don't do this
```

**`.unwrap()` is fine in tests.** Tests should panic on unexpected failures.

### No `unsafe`

There is no reason to use `unsafe` in this codebase. All dependencies that need it (SQLite, crypto, HTTP) handle it internally. If you think you need `unsafe`, you don't.

### No `panic!`

Use `return Err(...)` or `tracing::error!` instead. The scheduler and API server must never crash from user input or database state.

### Checked Math

Use `.checked_add()`, `.checked_mul()`, `.saturating_add()`, etc. for any arithmetic that could overflow, especially:
- Timeout calculations
- Retry delay with exponential backoff
- Pagination offsets
- Duration conversions (seconds ↔ milliseconds)

```rust
// Bad
let delay = base_delay * 2u64.pow(attempt);

// Good
let delay = base_delay.saturating_mul(2u64.saturating_pow(attempt));
```

### String Handling

**No `format!()` for SQL queries.** Use parameterized queries (`?1`, `?2`) with `rusqlite::params![]`. The only exception is dynamic WHERE clause construction via `QueryFilters` in `src/db/helpers.rs`, which parameterizes values but builds clause structure dynamically.

**No raw string concatenation for shell commands from user input.** Use `shell_escape()` from `src/executor/utils.rs` for any user-provided value injected into a shell command.

### Error Types

All fallible functions in the `src/db/` and `src/api/` layers return `Result<T, AppError>`. Map external errors with descriptive context:

```rust
// Bad
conn.execute(sql, params).map_err(AppError::Db)?;  // OK but minimal

// Better — when the context helps debugging
conn.execute(sql, params)
    .map_err(|e| AppError::Internal(format!("failed to update job {}: {e}", job.id)))?;
```

### Naming

- **Files**: `snake_case.rs`, one module per concern
- **Functions**: `snake_case`, verb-first (`get_job`, `insert_execution`, `run_sql_task`)
- **Types**: `PascalCase` (`JobResponse`, `ExecutionRecord`, `ConnectionType`)
- **Constants**: `SCREAMING_SNAKE_CASE` (`MAX_GROUP_NAME_LEN`, `DEFAULT_GROUP_NAME`)
- **Database columns**: `snake_case` (`group_name`, `created_at`, `task_snapshot_json`)
- **API routes**: `/api/kebab-case` (`/api/jobs`, `/api/pipeline-schedule`, `/api/connections`)
- **JSON fields**: `snake_case` (matches Rust struct fields via serde)

### Database

- **Named columns**: Use `col(row, "column_name")` from `src/db/helpers.rs`, never positional `row.get(N)`.
- **Migrations**: Add a new file `migrations/NNNN_description.sql` with `-- version: N` and `-- description:` headers. Never modify existing migrations.
- **Blocking calls**: Always run DB operations inside `tokio::task::spawn_blocking` or use the `db_call` helper. SQLite is synchronous; blocking the async runtime causes timeouts.

### API Layer

- **Auth checks first**: Every mutating handler should check `role.can_write()` before doing anything.
- **Audit logging**: All create/update/delete operations on sensitive resources (jobs, keys, connections, variables, scripts) must emit an audit event via `db.record_audit()`.
- **Pagination**: Use the `paginate()` and `paginated_response()` helpers from `src/api/mod.rs`.
- **Input validation**: Validate names, lengths, and formats before touching the DB. Return `AppError::BadRequest` with a clear message.

### Frontend (JavaScript)

- **No frameworks** — vanilla JS, no build step, all files concatenated by `build.rs`
- **No global namespace pollution** — use `let`/`const`, prefix globals with purpose (e.g., `allConnections`, `groupsViewMode`)
- **Escape user content** — always use `esc()` when inserting user data into HTML
- **API calls** — use the `api()` helper, never raw `fetch` (it handles auth, 401 redirect, 429 toast)

### Testing

- **Shared fixtures**: Use `tests/common/mod.rs` for `test_db()`, `make_job()`, `make_execution()`. Don't duplicate constructors in individual test files.
- **Integration tests**: Each test gets a fresh in-memory SQLite DB via `test_db()`.
- **JS tests**: In `web/tests/test_*.js`, run with `node`. CI runs them automatically.
- **Naming**: `test_feature_behavior` (e.g., `test_connection_crud`, `test_pipeline_schedule_deserialization`)

### Clippy

All code must pass `cargo clippy --all-targets` with zero warnings. CI enforces this.

```bash
cargo clippy --all-targets  # must produce zero warnings
```

### Formatting

All code must pass `cargo fmt --all -- --check`. Run `cargo fmt` before committing. CI checks this.

```bash
cargo fmt --all -- --check  # CI check (fails on unformatted code)
cargo fmt                   # auto-format before committing
```

## Pull Request Guidelines

1. Keep PRs focused — one feature or fix per PR
2. Add tests for new functionality
3. Update docs if the change is user-facing (CHANGELOG, README, API.md, in-app docs)
4. Run `cargo test --all && cargo clippy --all-targets && cargo fmt --check` before pushing
5. JS changes: run `node web/tests/test_*.js` to verify frontend tests pass
