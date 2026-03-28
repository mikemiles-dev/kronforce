## Why

The codebase has grown organically and accumulated duplicated logic, unsafe string indexing that can panic in production, inconsistent error handling, and functions exceeding 300 lines. Key issues: output rules processing is duplicated between local executor and agent callbacks, bootstrap key creation is copy-pasted 3 times, string slicing without bounds checks can crash the server, and row mapping in the DB layer uses 150+ bare unwraps on column access.

## What Changes

- **Eliminate duplication:**
  - Extract shared output rules processing (extractions, triggers, assertions, variable write-back) into a reusable function used by both local executor and agent callbacks
  - Consolidate bootstrap API key generation into a single factory method on `ApiKey`
  - Extract DB query filter building into a helper

- **Fix safety issues:**
  - Replace unsafe string indexing (`key[..11]`) with bounds-checked alternatives
  - Replace bare `.unwrap()` calls in DB row mapping with proper error propagation
  - Add logging for silently ignored database errors (`let _ = db.update(...)`)
  - Replace `spawn_blocking(...).await.unwrap()` with proper error mapping

- **Improve structure with OOP patterns:**
  - Add `ExecutionRecord::new()` builder to replace repeated 14-field struct construction
  - Add `ApiKey::create()` factory method
  - Add `Event` factory methods for common event types (`output_matched`, etc.)
  - Extract `run_task` match arms into per-task-type methods

- **Reduce code smells:**
  - Break down long functions (execute_local at 195 lines, run_script at 345 lines)
  - Define constants for magic numbers (prefix length, timeout defaults, max output bytes)
  - Fix the `created_by` TODO in job creation

## Capabilities

### New Capabilities
- `error-handling-patterns`: Standards for error handling, unwrap usage, and silent failure prevention across the codebase

### Modified Capabilities

## Impact

- **src/executor/local.rs** — major refactor (output rules extraction, function breakdown)
- **src/api/callbacks.rs** — replace duplicated output rules logic with shared function call
- **src/bin/controller.rs** — replace bootstrap key logic with ApiKey factory
- **src/db/helpers.rs** — replace unwraps with error propagation
- **src/db/models.rs** — add builder/factory methods to ExecutionRecord, ApiKey, Event
- **src/api/auth.rs** — fix unsafe string indexing
- **Tests** — existing tests must still pass; new tests for factory methods
