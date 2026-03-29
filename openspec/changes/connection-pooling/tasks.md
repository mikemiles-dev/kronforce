## 1. Dependencies

- [x] 1.1 Add `r2d2 = "0.8"` and `r2d2_sqlite = "0.25"` to `Cargo.toml`

## 2. Configuration

- [x] 2.1 Add `db_pool_size: u32` and `db_timeout_secs: u64` to `ControllerConfig` in `src/config.rs`, parsing from `KRONFORCE_DB_POOL_SIZE` (default 8) and `KRONFORCE_DB_TIMEOUT_SECS` (default 5)

## 3. Core — Db Struct and Pool Setup

- [x] 3.1 Replace `conn: Arc<Mutex<Connection>>` with `pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>` in the `Db` struct in `src/db/mod.rs`
- [x] 3.2 Rewrite `Db::open` to create a `SqliteConnectionManager` with `with_init` for WAL mode, foreign keys, and busy_timeout pragmas, then build an `r2d2::Pool` with configured size and timeout. Force pool size to 1 for `:memory:` paths.
- [x] 3.3 Update `Db::migrate` to use `self.pool.get()` instead of `self.conn.lock()`
- [x] 3.4 Update `Db::with_transaction` to use `self.pool.get()` instead of `self.conn.lock()`

## 4. Mechanical Replacement — DB Module Files

Replace all `self.conn.lock().map_err(|e| AppError::Internal(format!("lock poisoned: {e}")))` with `self.pool.get().map_err(|e| AppError::Internal(format!("pool error: {e}")))` in each file:

- [x] 4.1 `src/db/jobs.rs` — replace all lock calls with pool.get()
- [x] 4.2 `src/db/executions.rs` — replace all lock calls with pool.get()
- [x] 4.3 `src/db/agents.rs` — replace all lock calls with pool.get()
- [x] 4.4 `src/db/events.rs` — replace all lock calls with pool.get()
- [x] 4.5 `src/db/keys.rs` — replace all lock calls with pool.get()
- [x] 4.6 `src/db/settings.rs` — replace all lock calls with pool.get()
- [x] 4.7 `src/db/variables.rs` — replace all lock calls with pool.get()
- [x] 4.8 `src/db/queue.rs` — replace all lock calls with pool.get()
- [x] 4.9 `src/db/audit.rs` — replace all lock calls with pool.get()

## 5. Controller Startup

- [x] 5.1 Update `src/bin/controller.rs` to pass pool config to `Db::open` (or read from `ControllerConfig`)

## 6. Remove Old Imports

- [x] 6.1 Remove `use std::sync::{Arc, Mutex}` from `src/db/mod.rs` and add `r2d2` / `r2d2_sqlite` imports

## 7. Verify

- [x] 7.1 Run `cargo check` to verify compilation
- [x] 7.2 Run `cargo test` to verify all 236 tests pass
- [x] 7.3 Run `cargo clippy` to verify no new warnings
