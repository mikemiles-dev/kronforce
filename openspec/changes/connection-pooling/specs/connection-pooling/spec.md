## ADDED Requirements

### Requirement: Connection pool replaces single mutex
The `Db` struct SHALL use an `r2d2::Pool<SqliteConnectionManager>` instead of `Arc<Mutex<Connection>>`. Each database operation SHALL acquire a connection from the pool via `pool.get()` and return it when the operation completes.

#### Scenario: Concurrent read operations
- **WHEN** multiple API requests query the database simultaneously
- **THEN** each request gets its own pooled connection and reads execute concurrently without blocking each other

#### Scenario: Connection returned to pool after use
- **WHEN** a database operation completes
- **THEN** the pooled connection is returned to the pool for reuse by other operations

#### Scenario: Pool exhaustion
- **WHEN** all connections in the pool are in use and a new request needs a connection
- **THEN** the request waits up to the configured timeout, then returns an error if no connection becomes available

### Requirement: Connection initialization
Every connection created by the pool SHALL have WAL mode and foreign keys enabled. These pragmas SHALL be applied automatically when the connection is created, not on each use.

#### Scenario: New connection has WAL mode
- **WHEN** the pool creates a new connection
- **THEN** `PRAGMA journal_mode=WAL` is set on that connection

#### Scenario: New connection has foreign keys enabled
- **WHEN** the pool creates a new connection
- **THEN** `PRAGMA foreign_keys=ON` is set on that connection

#### Scenario: Busy timeout set
- **WHEN** the pool creates a new connection
- **THEN** `PRAGMA busy_timeout=5000` is set to handle write contention gracefully

### Requirement: Pool configuration via environment variables
The connection pool SHALL be configurable via environment variables with sensible defaults.

#### Scenario: Default pool size
- **WHEN** `KRONFORCE_DB_POOL_SIZE` is not set
- **THEN** the pool uses a maximum of 8 connections

#### Scenario: Custom pool size
- **WHEN** `KRONFORCE_DB_POOL_SIZE=16` is set
- **THEN** the pool uses a maximum of 16 connections

#### Scenario: Default connection timeout
- **WHEN** `KRONFORCE_DB_TIMEOUT_SECS` is not set
- **THEN** the pool waits up to 5 seconds for an available connection

#### Scenario: Custom connection timeout
- **WHEN** `KRONFORCE_DB_TIMEOUT_SECS=10` is set
- **THEN** the pool waits up to 10 seconds for an available connection

### Requirement: Transaction support via pool
The `with_transaction` method SHALL acquire a connection from the pool, start a transaction, execute the closure, and commit. The connection SHALL be returned to the pool after the transaction completes or rolls back.

#### Scenario: Successful transaction
- **WHEN** a transactional operation succeeds
- **THEN** the transaction is committed and the connection is returned to the pool

#### Scenario: Failed transaction
- **WHEN** a transactional operation returns an error
- **THEN** the transaction is rolled back and the connection is returned to the pool

### Requirement: Migration runs on a pooled connection
The `migrate` method SHALL acquire a single connection from the pool and run all pending migrations on it. Migration SHALL complete before the application starts serving requests.

#### Scenario: Migration succeeds
- **WHEN** the controller starts and calls `db.migrate()`
- **THEN** all pending migrations are applied using a connection from the pool

#### Scenario: Migration with pool size 1
- **WHEN** running against an in-memory database with pool size 1
- **THEN** migration succeeds and subsequent operations use the same connection

### Requirement: In-memory database uses pool size 1
When the database path is `:memory:`, the pool size SHALL be forced to 1 regardless of configuration. This ensures all operations share the same in-memory database, which is required for tests.

#### Scenario: In-memory database for tests
- **WHEN** `Db::open(":memory:")` is called
- **THEN** the pool is created with max size 1

#### Scenario: File-based database uses configured size
- **WHEN** `Db::open("kronforce.db")` is called
- **THEN** the pool uses the configured pool size (default 8)

### Requirement: Error mapping consistency
Pool connection errors SHALL be mapped to `AppError::Internal` with a descriptive message, consistent with the existing error handling pattern.

#### Scenario: Pool get error
- **WHEN** `pool.get()` fails (timeout or pool closed)
- **THEN** the error is mapped to `AppError::Internal("pool error: ...")`

#### Scenario: Existing query errors unchanged
- **WHEN** a SQL query fails after successfully getting a connection
- **THEN** the error is mapped to `AppError::Db(...)` as before
