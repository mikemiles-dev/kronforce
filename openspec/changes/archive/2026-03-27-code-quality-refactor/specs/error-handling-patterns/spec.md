## ADDED Requirements

### Requirement: No bare unwrap on database row access
Database row mapping functions SHALL use the `?` operator instead of `.unwrap()` when accessing column values, parsing UUIDs, or deserializing JSON from row data.

#### Scenario: Malformed UUID in database
- **WHEN** a database row contains an invalid UUID string
- **THEN** the query returns an error instead of panicking

#### Scenario: Malformed JSON in database
- **WHEN** a database row contains invalid JSON in a `_json` column
- **THEN** the query returns an error instead of panicking

#### Scenario: Missing column value
- **WHEN** a database row has a NULL in a non-optional column
- **THEN** the query returns an error instead of panicking

### Requirement: No unsafe string indexing
String slicing with hardcoded indices (e.g., `key[..11]`) SHALL use bounds-checked alternatives like `.get(..N)` to prevent panics on unexpectedly short strings.

#### Scenario: API key shorter than prefix length
- **WHEN** an API key string is shorter than the expected prefix length
- **THEN** the full string is used as the prefix instead of panicking

#### Scenario: UUID string slicing
- **WHEN** a UUID is formatted to string and sliced for display
- **THEN** the slicing uses `.get()` or equivalent bounds-checked method

### Requirement: Silently ignored errors log warnings
Database operations whose results are discarded with `let _ = ...` SHALL log a warning via `tracing::warn!` when the operation fails.

#### Scenario: Failed extraction update
- **WHEN** `update_execution_extracted` fails during post-execution processing
- **THEN** a warning is logged with the error details

#### Scenario: Failed assertion status update
- **WHEN** `fail_execution_assertion` fails during post-execution processing
- **THEN** a warning is logged with the error details

### Requirement: spawn_blocking errors propagate instead of panicking
Calls to `tokio::task::spawn_blocking(...).await.unwrap()` SHALL replace `.unwrap()` with error mapping that converts task join errors to `AppError::Internal`.

#### Scenario: Spawned task panics
- **WHEN** a `spawn_blocking` task panics internally
- **THEN** the caller receives an `AppError::Internal` error instead of the entire server panicking

### Requirement: Factory methods reduce struct construction boilerplate
Core structs with many fields SHALL provide constructor or factory methods to reduce repeated inline construction.

#### Scenario: Creating an ExecutionRecord
- **WHEN** code needs to create a new ExecutionRecord
- **THEN** it uses `ExecutionRecord::new(id, job_id, trigger)` with builder methods for optional fields instead of listing all 14 fields

#### Scenario: Creating a bootstrap ApiKey
- **WHEN** code needs to create an admin or agent API key during bootstrap
- **THEN** it uses `ApiKey::bootstrap(role, name, preset_key)` which returns `(ApiKey, raw_key_string)` instead of duplicating generation, prefix extraction, and hashing logic

### Requirement: Magic numbers replaced with named constants
Hardcoded numeric values with domain meaning SHALL be replaced with named constants.

#### Scenario: Key prefix length
- **WHEN** extracting an API key prefix
- **THEN** the code uses a named constant like `KEY_PREFIX_LEN` instead of the literal `11`

#### Scenario: Script timeout
- **WHEN** a Rhai script runs without a configured timeout
- **THEN** the code uses a named constant like `DEFAULT_SCRIPT_TIMEOUT_SECS` instead of a bare `60`

### Requirement: Duplicated output rules processing consolidated
The post-execution output rules logic (extractions, assertions, triggers, variable write-back, event creation) SHALL exist in a single shared function, not duplicated across local executor and agent callbacks.

#### Scenario: Local execution completes
- **WHEN** a locally-executed job finishes
- **THEN** the shared `process_post_execution` function handles all output rules

#### Scenario: Agent callback received
- **WHEN** an agent reports execution results via callback
- **THEN** the same shared `process_post_execution` function handles all output rules

#### Scenario: Output rules behavior unchanged
- **WHEN** comparing behavior before and after consolidation
- **THEN** extractions, assertions, triggers, variable write-back, and event emission produce identical results
