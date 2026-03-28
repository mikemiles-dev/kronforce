## ADDED Requirements

### Requirement: Encapsulated methods SHALL preserve existing behavior
All functions moved into `impl` blocks SHALL produce identical outputs for all inputs. No public API signatures or response formats SHALL change.

#### Scenario: Existing tests pass without modification
- **WHEN** free functions are moved into `impl` blocks as methods
- **THEN** all existing tests in `cargo test` SHALL pass without any test file modifications

### Requirement: Row converters SHALL use from_row convention
Database row conversion functions SHALL be implemented as `from_row()` associated functions on their respective model types.

#### Scenario: Model types gain from_row constructors
- **WHEN** `row_to_job()`, `row_to_execution()`, `row_to_agent()`, `row_to_api_key()` are refactored
- **THEN** they SHALL become `Job::from_row()`, `ExecutionRecord::from_row()`, `Agent::from_row()`, `ApiKey::from_row()` respectively
- **AND** all call sites SHALL be updated to use the new associated function syntax
