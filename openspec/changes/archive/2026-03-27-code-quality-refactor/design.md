## Context

The codebase has ~15K lines of Rust across 30+ files. It grew feature-by-feature, leading to duplicated logic (output rules processing appears twice, bootstrap key generation three times), unsafe string operations, inconsistent error handling, and large monolithic functions. The code works correctly but is increasingly hard to maintain.

## Goals / Non-Goals

**Goals:**
- Eliminate duplicated logic via shared functions and factory methods
- Fix string indexing that can panic in production
- Replace bare `.unwrap()` in DB row mapping with error propagation
- Add builder/factory patterns to reduce boilerplate struct construction
- Break down functions over ~150 lines
- Define constants for magic numbers
- Make silently-ignored errors at least log warnings

**Non-Goals:**
- Changing any external behavior, API responses, or database schema
- Introducing new dependencies (no ORM, no error crate like anyhow/thiserror)
- Rewriting working code just for style preferences
- Splitting files that are already well-scoped

## Decisions

### 1. Shared output rules processing function

Extract the duplicated output rules logic from `executor/local.rs` and `api/callbacks.rs` into a new function in `executor/output_rules.rs` (which already exists for extraction/trigger/assertion runners):

```rust
pub async fn process_post_execution(
    db: &Db,
    job: &Job,
    exec_id: Uuid,
    stdout: &str,
    stderr: &str,
    exec_status: ExecutionStatus,
) -> Vec<Event>
```

This function handles: run extractions → store extracted values → variable write-back → run assertions → fail execution if assertion fails → run triggers → return events. Both `local.rs` and `callbacks.rs` call this single function.

**Why here:** `output_rules.rs` already owns the extraction/trigger/assertion logic. Adding the orchestration function here keeps the responsibility cohesive.

### 2. ApiKey factory method

Add `ApiKey::bootstrap(role, name, preset_key)` that encapsulates key generation, prefix extraction, and hashing. Returns `(ApiKey, raw_key_string)`. The controller uses this for both admin and agent bootstrap, and for `--reset-admin-key`.

**Why not a separate module:** ApiKey is already in models. The factory method belongs on the struct itself.

### 3. ExecutionRecord::new() constructor

Add a constructor with sensible defaults (empty strings, None optionals) that takes only the required fields (id, job_id, trigger). Call sites then use builder-style `.with_status()`, `.with_agent_id()` etc. for the few fields that vary.

### 4. Safe string indexing via helper

Replace `key[..11]` with `key.get(..11).unwrap_or(&key).to_string()` or a shared helper `fn key_prefix(key: &str) -> String`. This prevents panics on short strings.

### 5. DB row mapping error handling

Replace `.unwrap()` on `row.get()` calls in `db/helpers.rs` with `?` operator. The row mapper functions already return `rusqlite::Result`, so this is a straightforward change. For `Uuid::parse_str` and `serde_json::from_str`, map errors to `rusqlite::Error::InvalidParameterName` or similar so they propagate through the same Result type.

### 6. Constants for magic numbers

Add to `executor/local.rs`:
```rust
const MAX_OUTPUT_BYTES: usize = 10 * 1024 * 1024; // already exists
const DEFAULT_SCRIPT_TIMEOUT_SECS: u64 = 60;
const MAX_SCRIPT_OUTPUT_LINES: usize = 1_000_000;
```

Add to `api/auth.rs`:
```rust
const KEY_PREFIX_LEN: usize = 11;
```

### 7. Log silently-ignored errors

Replace `let _ = db.update_foo(...)` with:
```rust
if let Err(e) = db.update_foo(...) {
    tracing::warn!("failed to update foo: {}", e);
}
```

This applies to ~10 sites in `local.rs` and `callbacks.rs`.

## Risks / Trade-offs

- **Large diff across many files** — The changes are mechanical and individually small, but touch many files. → Mitigate by doing one category at a time and running tests after each.
- **DB row mapping changes** — Changing unwrap to `?` changes error behavior from panic to returning an error. If any row data is actually malformed, this surfaces as an API error instead of a crash. → This is strictly better behavior.
- **spawn_blocking unwrap** — Removing `.unwrap()` after `spawn_blocking` requires deciding what to do when a spawned task panics. → Map to `AppError::Internal` with context.
