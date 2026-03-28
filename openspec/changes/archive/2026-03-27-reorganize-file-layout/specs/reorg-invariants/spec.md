## ADDED Requirements

### Requirement: File reorganization SHALL not change public API surface
All public types, functions, and modules that were accessible before the reorganization SHALL remain accessible at the same logical paths after re-exports are applied.

#### Scenario: Model types remain accessible via db::models
- **WHEN** `db/models.rs` is split into `db/models/` subdirectory files
- **THEN** all types previously available via `crate::db::models::*` SHALL still be importable via the same path

#### Scenario: Task functions remain accessible
- **WHEN** task implementations are moved from `executor/local.rs` to `executor/tasks/` files
- **THEN** the `run_task()` function SHALL remain publicly exported from the executor module

### Requirement: All existing tests SHALL pass without modification
No test file SHALL require changes as a result of this reorganization.

#### Scenario: Full test suite passes after reorganization
- **WHEN** all file moves and import updates are complete
- **THEN** `cargo test` SHALL pass with zero failures and no test modifications
