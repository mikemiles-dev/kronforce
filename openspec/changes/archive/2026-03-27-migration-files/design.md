## Context

`db/mod.rs` contains a `Vec<(i64, &str, &str)>` with 13 migration tuples — each is a version number, description, and SQL string. The SQL is applied sequentially on startup, tracked via a `schema_version` table. This works but the SQL is buried in Rust code, making it hard to review or manage.

## Goals / Non-Goals

**Goals:**
- Move migration SQL out of Rust code into standalone `.sql` files
- Consolidate all existing migrations into one init file (v0.1.0 is the baseline)
- Embed migration files at compile time via `build.rs` for single-binary deployment
- Keep the `schema_version` tracking table and version-based application logic

**Non-Goals:**
- Rollback/down migrations (keep it simple — forward only)
- Migration tooling CLI (just startup auto-apply)
- Changing the database schema itself

## Decisions

### 1. File structure

```
migrations/
  0001_init.sql
```

Future migrations:
```
migrations/
  0001_init.sql
  0002_v0.2.0_add_webhooks.sql
  0003_v0.2.1_fix_index.sql
```

The `NNNN` prefix is the migration version number (matches `schema_version.version`). The rest is descriptive. The init file gets version 13 (matching the current max) so existing databases with versions 1-13 skip it, and fresh databases get the full schema.

### 2. Embed at compile time

`build.rs` reads all `*.sql` files from `migrations/`, sorts by filename, and generates a Rust source file in `OUT_DIR`:

```rust
pub const MIGRATIONS: &[(i64, &str, &str)] = &[
    (13, "Initial schema (v0.1.0)", include_str!("../migrations/0001_init.sql")),
];
```

This keeps the same data structure the current `migrate()` function uses, just sourced from files instead of inline strings.

### 3. Migration versioning

`0001_init.sql` gets version **13** (the current max). This way:
- Fresh databases: apply version 13 (full schema)
- Existing databases at version ≤13: skip (already applied)
- Future migrations: start at version 14+

The version number is extracted from the filename prefix: `0001` maps to 13 for the init, future files use their actual number. Actually, simpler: embed the version in the SQL file as a comment header:

```sql
-- version: 13
-- description: Initial schema (v0.1.0)
CREATE TABLE IF NOT EXISTS jobs ( ...
```

`build.rs` parses these headers to build the migrations array.

### 4. migrate() stays mostly the same

The function signature and logic don't change — it still iterates `(version, description, sql)` tuples and applies those above `current_version`. The only change is the source: from inline `vec![]` to the generated constant.

## Risks / Trade-offs

- **Version 13 gap** — The init file is version 13, so versions 1-12 are "virtual" history. New users see only version 13 in `schema_version`. → Acceptable; the history was only useful during development.
- **SQL parsing in build.rs** — Extracting version/description from comment headers adds build complexity. → Keep the format strict and simple.
