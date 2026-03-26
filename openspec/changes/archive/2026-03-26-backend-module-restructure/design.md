## Context

The codebase has 17 Rust source files all flat in `src/`. Three files exceed 1100 lines each: `api.rs` (1334), `db.rs` (1442), `executor.rs` (1122). Each contains multiple distinct responsibilities that would benefit from being in separate files.

Rust supports folder modules: `src/api.rs` can become `src/api/mod.rs` with submodules `src/api/jobs.rs`, etc. Public items re-exported from `mod.rs` maintain the same external API — `crate::api::AppState` works regardless of whether `AppState` is defined in `api.rs` or `api/mod.rs`.

## Goals / Non-Goals

**Goals:**
- Break large files into focused submodules (< 300 lines each)
- Group related files into folder modules
- Maintain identical public API — nothing changes for `lib.rs`, `bin/`, or cross-module imports
- Compilable and functional after each module migration (not a big-bang rewrite)

**Non-Goals:**
- Changing any behavior, APIs, or database schema
- Refactoring internal logic within functions
- Changing public type signatures or module paths
- Splitting `models.rs` (427 lines is manageable)

## Decisions

### 1. Folder module pattern with re-exports

**Decision**: Each split module uses the folder pattern (`src/api/mod.rs`) with submodules as private `mod` items. `mod.rs` re-exports everything needed by external callers via `pub use`.

Example for api:
```rust
// src/api/mod.rs
mod jobs;
mod executions;
mod agents;
mod events;
mod auth;
mod settings;
mod scripts;
mod callbacks;

pub use self::jobs::*;  // not needed — handlers are private, only router is public

pub struct AppState { ... }
pub fn router(state: AppState) -> Router { ... }
```

Submodules access shared types via `use super::*` or explicit `use super::AppState`.

**Rationale**: This is the standard Rust pattern for breaking up large modules. Re-exports keep the same `crate::api::AppState` path.

### 2. Split by domain, not by layer

**Decision**: Split files by domain (jobs, agents, executions) rather than by layer (handlers, queries, models). Each domain file contains all the code for that domain within its parent module.

For `api/`: `jobs.rs` has all job handlers. For `db/`: `jobs.rs` has all job queries.

**Rationale**: When working on a feature (e.g., agents), you want all related code nearby. Domain splitting is more intuitive than layer splitting for a project of this size.

### 3. Migrate one module at a time

**Decision**: Migrate in order: `agent/` first (simplest — just rename and re-export), then `db/`, then `executor/`, then `api/` (most complex — has the router assembly).

Each migration is a self-contained change that compiles and runs.

**Rationale**: Incremental migration reduces risk. If something breaks, the scope of the change is clear.

### 4. Shared types stay in parent mod.rs

**Decision**: Types used across submodules within a folder (e.g., `AppState`, `AuthUser` in api/) stay in `mod.rs`. Submodules import them via `use super::*`.

For `db/`: The `Db` struct stays in `mod.rs` with the connection setup and migrations. Query methods are `impl Db` blocks in submodules.

**Rationale**: Avoids circular dependencies between submodules. The parent module owns the shared types, submodules add behavior.

### 5. Helper functions stay in a helpers submodule for db

**Decision**: `row_to_job`, `row_to_execution`, `row_to_agent`, `row_to_api_key` move to `db/helpers.rs`. These are used across multiple query submodules so they need to be in a shared location.

Mark them `pub(super)` so they're visible to sibling submodules but not outside `db/`.

**Rationale**: Row mappers are used by jobs.rs, executions.rs, agents.rs, and keys.rs. A shared helpers module avoids duplication.

## Risks / Trade-offs

- **Large diff** → Mitigated by doing one module at a time. Git will show file renames clearly.
- **Import path changes in submodules** → `use crate::models::*` becomes `use crate::models::*` (unchanged). Only internal imports like `use super::AppState` change.
- **IDE navigation** → More files means more tabs, but each file is focused. Tree view in IDE groups them naturally.
