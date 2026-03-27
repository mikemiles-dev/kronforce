## ADDED Requirements

### Requirement: Loose Rust files remain in src/ root
Standalone Rust files under 500 lines that have no sub-modules SHALL remain as flat files in `src/` rather than being wrapped in unnecessary directories.

#### Scenario: Small files stay flat
- **WHEN** the restructure is complete
- **THEN** `config.rs`, `error.rs`, `dag.rs`, `protocol.rs`, `models.rs`, `cron_parser.rs`, `scheduler.rs`, `notifications.rs`, `output_rules.rs`, and `scripts.rs` remain directly in `src/`

#### Scenario: No empty wrapper directories
- **WHEN** examining `src/` after restructure
- **THEN** there are no single-file module directories (e.g., no `src/config/mod.rs` wrapping what was `src/config.rs`)

### Requirement: dashboard.html removed from src/
The `src/dashboard.html` file SHALL be removed after its content is migrated to `web/`.

#### Scenario: No HTML in src/
- **WHEN** the restructure is complete
- **THEN** `src/dashboard.html` does not exist
