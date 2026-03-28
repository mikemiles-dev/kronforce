### Requirement: All public structs SHALL have doc comments
Every `pub` and `pub(crate)` struct definition SHALL have a `///` doc comment describing its purpose.

#### Scenario: Struct with no doc comment
- **WHEN** a public struct exists without a `///` doc comment
- **THEN** a doc comment SHALL be added describing what the struct represents and its role in the system

#### Scenario: Struct with fields that need context
- **WHEN** a public struct has fields whose purpose is not obvious from the field name and type
- **THEN** the struct's doc comment SHALL briefly describe the key fields

### Requirement: All public enum types SHALL have doc comments
Every `pub` and `pub(crate)` enum definition SHALL have a `///` doc comment. Enum variants that are not self-explanatory SHALL also have doc comments.

#### Scenario: Enum with no doc comment
- **WHEN** a public enum exists without a `///` doc comment
- **THEN** a doc comment SHALL be added describing the enum's purpose and when each variant applies

#### Scenario: Enum variants with non-obvious meaning
- **WHEN** an enum variant's meaning is not clear from its name alone
- **THEN** the variant SHALL have an inline `///` doc comment

### Requirement: All public functions and methods SHALL have doc comments
Every `pub` and `pub(crate)` function and method SHALL have a `///` doc comment describing what it does.

#### Scenario: API handler function with no doc comment
- **WHEN** a public API handler function in `src/api/` lacks a doc comment
- **THEN** a doc comment SHALL be added describing the endpoint's purpose, expected input, and response behavior

#### Scenario: Database method with no doc comment
- **WHEN** a public method on `Db` in `src/db/` lacks a doc comment
- **THEN** a doc comment SHALL be added describing what the method queries or mutates and what it returns

#### Scenario: Agent/executor/scheduler function with no doc comment
- **WHEN** a public function in `src/agent/`, `src/executor/`, or `src/scheduler/` lacks a doc comment
- **THEN** a doc comment SHALL be added describing the function's behavior

### Requirement: Module-level documentation SHALL exist for all modules
Every module (`mod.rs` or top-level module file) SHALL have a `//!` module-level doc comment describing the module's purpose and responsibilities.

#### Scenario: Module file with no module-level doc
- **WHEN** a module file (e.g., `src/lib.rs`, `src/agent/mod.rs`) lacks a `//!` comment
- **THEN** a `//!` doc comment SHALL be added at the top of the file describing the module's role

### Requirement: Doc comments SHALL follow existing codebase style
Documentation SHALL follow the concise, declarative style established in `src/error.rs` and `src/db/models.rs`.

#### Scenario: New doc comment style consistency
- **WHEN** a new doc comment is written
- **THEN** it SHALL use a one-line summary for simple items and a multi-line format (summary + blank line + details) for complex items

#### Scenario: No behavioral code changes
- **WHEN** doc comments are added to a file
- **THEN** no runtime behavior, function signatures, or type definitions SHALL be modified
