## Why

Several files have grown well beyond a comfortable size, making navigation and modification difficult. `executor/local.rs` is 1,412 lines containing 14 different task type implementations in a single file. `db/models.rs` is 652 lines with 50+ types spanning unrelated domains. The re-export pattern in `lib.rs` adds confusion about where modules actually live. Reorganizing now — before the codebase grows further — will improve maintainability.

## What Changes

- Split `executor/local.rs` into per-task-type modules under `executor/tasks/` (shell, sql, ftp, http, messaging, file_push, script), keeping only the dispatch function and shared types in `local.rs`
- Split `db/models.rs` into domain-specific model files under `db/models/` (task, job, execution, agent, event, auth, variable), with a `mod.rs` that re-exports everything so downstream code is unaffected
- Clean up `lib.rs` re-exports: remove the `pub use` aliases that re-export sub-modules at the crate root, and update all call sites to use the canonical paths (e.g., `crate::executor::notifications` instead of `crate::notifications`)

## Capabilities

### New Capabilities

_(none — this is a file layout reorganization with no new behavioral capabilities)_

### Modified Capabilities

_(none — all changes are structural, no spec-level behavior changes)_

## Impact

- `src/executor/local.rs` — split into ~8 files under `src/executor/tasks/`
- `src/db/models.rs` — split into ~7 files under `src/db/models/`
- `src/lib.rs` — re-export aliases removed
- All files that use `crate::protocol`, `crate::models`, `crate::notifications`, `crate::output_rules`, `crate::scripts`, or `crate::cron_parser` will need import path updates
- No API, schema, or behavioral changes
- Existing tests must continue to pass unchanged
