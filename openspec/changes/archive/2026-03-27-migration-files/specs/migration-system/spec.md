## ADDED Requirements

### Requirement: Migrations stored as SQL files
Database migrations SHALL be stored as individual `.sql` files in a `migrations/` directory at the project root, with a version and description header comment.

#### Scenario: Migration file format
- **WHEN** a migration file exists at `migrations/0001_init.sql`
- **THEN** it begins with `-- version: 13` and `-- description: Initial schema (v0.1.0)` comment headers followed by SQL statements

#### Scenario: Future migration file
- **WHEN** a new migration is added for version 0.2.0
- **THEN** it is created as `migrations/0002_v0.2.0_description.sql` with appropriate version and description headers

### Requirement: Migrations embedded at compile time
Migration files SHALL be embedded into the binary at compile time via `build.rs`, preserving single-binary deployment.

#### Scenario: build.rs processes migration files
- **WHEN** `cargo build` runs
- **THEN** `build.rs` reads all `.sql` files from `migrations/`, parses their version/description headers, and generates a Rust constant array

#### Scenario: Cargo rebuilds on migration changes
- **WHEN** a migration file in `migrations/` is added or modified
- **THEN** `build.rs` triggers a rebuild via `cargo::rerun-if-changed` directives

### Requirement: Existing databases are compatible
The init migration SHALL use the same version number as the current max (13) so existing databases skip it.

#### Scenario: Fresh database
- **WHEN** the controller starts with no database
- **THEN** migration version 13 is applied, creating the full schema

#### Scenario: Existing database at version 13
- **WHEN** the controller starts with a database already at version 13
- **THEN** no migrations are applied

#### Scenario: Future migration applied
- **WHEN** the controller starts with a database at version 13 and a migration at version 14 exists
- **THEN** only version 14 is applied

### Requirement: No hardcoded SQL in Rust source
After this change, `src/db/mod.rs` SHALL NOT contain inline SQL migration strings. All migration SQL SHALL live in `migrations/*.sql` files.

#### Scenario: Reviewing db/mod.rs
- **WHEN** examining the migrate() function
- **THEN** it references the generated migrations constant, not inline SQL strings
