### Requirement: Agent code grouped in src/agent/ folder module
The `agent_client.rs` and `agent_server.rs` files SHALL be moved into `src/agent/` as `client.rs` and `server.rs` with a `mod.rs` that re-exports their public items.

#### Scenario: External imports unchanged
- **WHEN** other modules import agent types via `crate::agent_client::*` or `crate::agent_server::*`
- **THEN** the imports are updated to `crate::agent::client::*` or `crate::agent::*` via re-exports, and all code compiles

#### Scenario: Agent folder structure
- **WHEN** the migration is complete
- **THEN** `src/agent/` contains `mod.rs`, `client.rs`, and `server.rs`

### Requirement: Database code split into src/db/ folder module
The `db.rs` file SHALL be split into `src/db/` with submodules for jobs, executions, agents, queue, events, keys, settings, and helpers.

#### Scenario: Db struct and migrations in mod.rs
- **WHEN** the db module is loaded
- **THEN** `mod.rs` contains the `Db` struct, `new()` constructor, and migration logic

#### Scenario: Query methods in domain submodules
- **WHEN** job-related queries are needed
- **THEN** they are defined as `impl Db` blocks in `db/jobs.rs`, not in `mod.rs`

#### Scenario: Row mapper helpers shared across submodules
- **WHEN** `row_to_job`, `row_to_execution`, `row_to_agent`, `row_to_api_key` are needed
- **THEN** they are in `db/helpers.rs` with `pub(super)` visibility

#### Scenario: External API unchanged
- **WHEN** other modules call `db.insert_job()`, `db.get_execution()`, etc.
- **THEN** all calls compile without changes because methods are still on `Db`

### Requirement: Executor code split into src/executor/ folder module
The `executor.rs` file SHALL be split into `src/executor/` with submodules for local execution, agent dispatch, and output processing.

#### Scenario: Executor struct in mod.rs
- **WHEN** the executor module is loaded
- **THEN** `mod.rs` contains the `Executor` struct, `new()`, and the `execute()` entry point

#### Scenario: Local execution in local.rs
- **WHEN** a job runs locally
- **THEN** `run_task`, `run_command`, `run_http`, `run_script` are defined in `executor/local.rs`

#### Scenario: Dispatch logic in dispatch.rs
- **WHEN** a job is dispatched to agents
- **THEN** `dispatch_to_agent`, `dispatch_to_any`, `dispatch_to_all`, `dispatch_to_tagged`, `dispatch_to_specific_agent` are in `executor/dispatch.rs`

#### Scenario: Output processing in output.rs
- **WHEN** post-execution output rules run
- **THEN** the output rules integration code is in `executor/output.rs`

### Requirement: API code split into src/api/ folder module
The `api.rs` file SHALL be split into `src/api/` with submodules for each domain's handlers plus auth middleware.

#### Scenario: Router assembly in mod.rs
- **WHEN** the API module is loaded
- **THEN** `mod.rs` contains `AppState`, the `router()` function that assembles all routes, and shared types

#### Scenario: Handlers grouped by domain
- **WHEN** a job endpoint is called
- **THEN** the handler is defined in `api/jobs.rs`, not in `mod.rs`

#### Scenario: Auth middleware in auth.rs
- **WHEN** authentication is checked
- **THEN** `auth_middleware`, `AuthUser`, API key hashing, and key management handlers are in `api/auth.rs`

#### Scenario: All routes still work
- **WHEN** any API endpoint is called after the restructure
- **THEN** it returns the same response as before

### Requirement: lib.rs updated with new module structure
The `lib.rs` file SHALL declare the new folder modules and remove the old flat file declarations.

#### Scenario: Module declarations match new structure
- **WHEN** the project compiles
- **THEN** `lib.rs` declares `pub mod api`, `pub mod db`, `pub mod executor`, `pub mod agent` (folder modules) instead of the old flat files

### Requirement: Each submodule under 300 lines
Each new submodule file SHALL be under 300 lines to ensure focused, navigable code.

#### Scenario: No oversized submodules
- **WHEN** all splits are complete
- **THEN** no individual submodule file exceeds 300 lines
