## Context

The `Db` struct in `src/db/mod.rs` wraps `Arc<Mutex<Connection>>`. Every database operation across the entire codebase calls `self.conn.lock().map_err(...)` to acquire the mutex, runs the query, and releases on drop. The async wrapper `db_call` uses `spawn_blocking` to avoid blocking the tokio runtime, but all spawned tasks still contend on the same mutex.

SQLite in WAL mode supports concurrent readers with a single writer. The current mutex serializes even readers unnecessarily. An r2d2 pool gives each `spawn_blocking` call its own connection, enabling true concurrent reads.

There are ~50 `self.conn.lock().map_err(...)` calls across 10 files in `src/db/`: `mod.rs`, `jobs.rs`, `executions.rs`, `agents.rs`, `events.rs`, `keys.rs`, `settings.rs`, `variables.rs`, `queue.rs`, `audit.rs`.

## Goals / Non-Goals

**Goals:**
- Replace single mutex with r2d2 connection pool
- Enable concurrent read operations
- Maintain write serialization via SQLite's WAL mode (handled by SQLite itself)
- Configurable pool size and timeout
- WAL mode and foreign keys pragma applied to every connection in the pool
- Minimal API surface change — `Db::open` returns a pool-backed Db, all query methods work the same

**Non-Goals:**
- Switching to a different database (Postgres, etc.)
- Adding async SQLite (e.g., `tokio-rusqlite`) — `spawn_blocking` + synchronous rusqlite is the established pattern
- Read replicas or multi-database setups
- Connection-level query caching or prepared statement caching

## Decisions

### 1. Use `r2d2` + `r2d2_sqlite` crates

`r2d2` is the standard Rust connection pool. `r2d2_sqlite` provides the `SqliteConnectionManager` adapter for rusqlite connections.

```toml
r2d2 = "0.8"
r2d2_sqlite = "0.25"
```

**Alternatives considered:**
- `deadpool-sqlite`: Async-native but adds tokio dependency to DB layer. Overkill since we already use `spawn_blocking`.
- Hand-rolled pool with `Vec<Connection>`: Reinventing the wheel — r2d2 is battle-tested and tiny.
- `sqlx`: Full rewrite to async queries. Way too invasive for this change.

### 2. `Db` struct holds `r2d2::Pool<SqliteConnectionManager>`

```rust
#[derive(Clone)]
pub struct Db {
    pool: r2d2::Pool<r2d2_sqlite::SqliteConnectionManager>,
}
```

Each method calls `self.pool.get()` to get a pooled connection. The connection is returned to the pool when dropped. No more `Arc<Mutex<...>>`.

### 3. Connection initialization via `ConnectionCustomizer`

Every new connection in the pool needs WAL mode and foreign keys enabled. r2d2 supports this via a custom `ManageConnection` or by running init SQL on the manager:

```rust
let manager = SqliteConnectionManager::file(path)
    .with_init(|c| {
        c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")
    });
```

### 4. Pool configuration

| Env Var | Default | Description |
|---------|---------|-------------|
| `KRONFORCE_DB_POOL_SIZE` | `8` | Maximum connections in the pool |
| `KRONFORCE_DB_TIMEOUT_SECS` | `5` | Seconds to wait for a connection before erroring |

8 connections is appropriate for a single-process SQLite app — SQLite WAL supports one writer + many readers, and 8 covers typical API concurrency without over-allocating file handles.

### 5. Mechanical replacement pattern

Every file follows the same transformation:

```rust
// Before:
let conn = self.conn.lock()
    .map_err(|e| AppError::Internal(format!("lock poisoned: {e}")))?;

// After:
let conn = self.pool.get()
    .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
```

The `conn` variable type changes from `MutexGuard<Connection>` to `r2d2::PooledConnection<SqliteConnectionManager>`, but both deref to `Connection`, so all query code works unchanged.

### 6. `with_transaction` gets a connection from the pool

```rust
pub fn with_transaction<F, T>(&self, f: F) -> Result<T, AppError>
where
    F: FnOnce(&rusqlite::Transaction) -> Result<T, AppError>,
{
    let mut conn = self.pool.get()
        .map_err(|e| AppError::Internal(format!("pool error: {e}")))?;
    let tx = conn.transaction().map_err(AppError::Db)?;
    let result = f(&tx)?;
    tx.commit().map_err(AppError::Db)?;
    Ok(result)
}
```

### 7. `migrate` runs on a single connection

Migration runs once at startup before the pool is used, so it gets a connection from the pool and holds it for the duration. No concurrency concern.

### 8. Test `Db::open(":memory:")` uses pool size 1

In-memory SQLite databases are per-connection. With a pool of >1 connections, each would get a different in-memory database. For tests, the pool size is forced to 1 to maintain the current test behavior.

## Risks / Trade-offs

- **SQLite write contention** → SQLite still allows only one writer at a time. Under heavy write load, pool connections will queue on SQLite's internal write lock (returning `SQLITE_BUSY`). This is the same behavior as before — the bottleneck just moves from Rust mutex to SQLite's lock. The `busy_timeout` pragma (set to 5000ms by default) handles this gracefully.
- **In-memory DB testing** → Pool size must be 1 for `:memory:` databases. This is handled in `Db::open` by detecting the `:memory:` path.
- **Connection overhead** → Each pool connection holds an open file descriptor. 8 connections × 1 FD = negligible.
- **Migration concurrency** → Migration must complete before the pool serves requests. Already the case since `migrate()` is called before starting the HTTP server.
