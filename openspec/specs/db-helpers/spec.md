## ADDED Requirements

### Requirement: Generic async database call helper
The `Db` module SHALL export a `db_call()` async function that wraps `spawn_blocking` with proper error mapping, eliminating repetitive boilerplate in API handlers.

#### Scenario: Simple database read
- **WHEN** an API handler needs to call a database method
- **THEN** it uses `db_call(&state.db, |db| db.list_jobs(...)).await?` instead of manual clone/spawn_blocking/unwrap

#### Scenario: Spawned task panics
- **WHEN** a `db_call` closure panics
- **THEN** the error is mapped to `AppError::Internal` instead of crashing the server

### Requirement: Transaction wrapper for multi-step operations
The `Db` struct SHALL provide a `with_transaction()` method that executes a closure within a SQLite transaction, committing on success and rolling back on error.

#### Scenario: Atomic job deletion
- **WHEN** a job is deleted
- **THEN** the dependency check and delete happen within a single transaction

#### Scenario: Transaction rollback on error
- **WHEN** a multi-step operation fails partway through
- **THEN** the transaction is rolled back and no partial state is persisted

### Requirement: Shared query filter builder
The database layer SHALL provide a query filter helper that builds WHERE clauses with parameterized indices for status filtering and search, eliminating duplicated filter logic.

#### Scenario: Job listing with filters
- **WHEN** listing jobs with status and search filters
- **THEN** the filter builder generates correct WHERE clauses and parameter vectors

#### Scenario: Execution listing with filters
- **WHEN** listing executions with status and search filters
- **THEN** the same filter builder is reused

### Requirement: Per-task-type execution functions
The `run_task()` function SHALL delegate to individual per-task-type functions, with the main function acting as a thin dispatcher.

#### Scenario: Shell task execution
- **WHEN** a shell task is executed
- **THEN** `run_task()` delegates to a focused `run_shell_task()` function

#### Scenario: Adding a new task type
- **WHEN** a developer adds a new TaskType variant
- **THEN** they add a single focused function and one match arm in `run_task()`

### Requirement: Shared notification helper
Execution completion notification logic SHALL exist in a single shared function, not duplicated between local executor and agent callbacks.

#### Scenario: Local execution sends notification
- **WHEN** a locally-executed job completes and has notification config
- **THEN** the shared notification helper determines if notification is needed and sends it

#### Scenario: Agent callback sends notification
- **WHEN** an agent reports execution results with notification config
- **THEN** the same shared notification helper is used
