## Why

The entire Kronforce database layer uses a single `Arc<Mutex<Connection>>` for all operations. Every query — from the scheduler tick to API requests to background health checks — contends on the same mutex. Under concurrent load (multiple API clients, agent heartbeats, and scheduled jobs all hitting the DB simultaneously), this serialization becomes a bottleneck causing increased latency and potential timeouts. A connection pool allows multiple concurrent readers while maintaining write serialization through SQLite's WAL mode.

## What Changes

- Replace `Arc<Mutex<Connection>>` in `src/db/mod.rs` with an `r2d2` connection pool backed by `r2d2_sqlite`
- The `Db` struct changes from holding a single connection to holding a pool
- All `self.conn.lock().map_err(...)` calls across every `src/db/*.rs` file change to `self.pool.get().map_err(...)`
- The `with_transaction` helper updates to get a connection from the pool then start a transaction
- The `db_call` async wrapper remains unchanged — it still uses `spawn_blocking` but now each call gets its own connection from the pool instead of waiting on the mutex
- Pool configuration: min idle connections, max pool size, connection timeout — configurable via environment variables with sensible defaults
- WAL mode and foreign keys pragma set on each new connection via a `ConnectionCustomizer`

## Capabilities

### New Capabilities
- `connection-pooling`: Replace single-mutex SQLite connection with r2d2 connection pool for concurrent database access

### Modified Capabilities

## Impact

- **Dependencies**: New crates `r2d2` and `r2d2_sqlite` added to `Cargo.toml`
- **Backend**: `Db` struct internals change (pool instead of mutex). Every `src/db/*.rs` file needs mechanical replacement of lock calls. `db_call` and `with_transaction` updated.
- **Configuration**: New env vars `KRONFORCE_DB_POOL_SIZE` (default 8) and `KRONFORCE_DB_TIMEOUT_SECS` (default 5)
- **Tests**: Test helper `test_db()` in all test files updated to use pool-backed Db
- **No breaking changes**: External API is identical. The `Db` struct's public interface (`open`, query methods) stays the same.
- **No migration**: This is a runtime change, not a schema change.
