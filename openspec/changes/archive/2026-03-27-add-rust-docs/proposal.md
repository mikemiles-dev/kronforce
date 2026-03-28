## Why

The Kronforce codebase has inconsistent documentation coverage. Core modules like `error.rs`, `config.rs`, `dag.rs`, and `db/models.rs` are well-documented, but the majority of public items — particularly in the API handlers, database methods, agent protocol, and executor dispatch — lack doc comments entirely. This makes onboarding harder and increases the risk of misuse when extending or modifying the system. With ~120+ undocumented public items across the codebase, now is a good time to establish comprehensive documentation before the codebase grows further.

## What Changes

- Add `///` doc comments to all undocumented public structs, enums, traits, functions, and methods across the codebase
- Add `//!` module-level documentation to modules that lack it (e.g., `src/lib.rs`, `src/agent/mod.rs`)
- Focus areas by priority:
  - **API module** (`src/api/`): ~35+ handler functions and ~15+ request/response structs completely undocumented
  - **Database module** (`src/db/`): ~40+ impl methods across `agents.rs`, `jobs.rs`, `keys.rs`, `queue.rs`, `settings.rs`, `variables.rs`
  - **Agent module** (`src/agent/`): Protocol structs in `protocol.rs`, `AgentClient` methods, agent server handlers
  - **Executor module** (`src/executor/`): Dispatch methods, local execution functions, `ScriptStore`
  - **Scheduler module** (`src/scheduler/`): Cron parser internals (`FieldSpec`, `parse_field`)

## Capabilities

### New Capabilities
- `rust-doc-coverage`: Standards and requirements for Rust documentation coverage across all public items in the codebase

### Modified Capabilities

_(none — this is a documentation-only change with no behavioral modifications)_

## Impact

- All `src/**/*.rs` files with undocumented public items will be modified
- No behavioral changes — documentation only
- No API, dependency, or schema changes
