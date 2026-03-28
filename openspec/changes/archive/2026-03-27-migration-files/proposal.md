## Why

The database migration logic is hardcoded as 13 inline SQL strings in `src/db/mod.rs`. This makes migrations hard to review, test independently, and manage as the project grows. Moving to migration files establishes a clean foundation for version-based schema evolution.

## What Changes

- **Create `migrations/` directory** at the project root with SQL migration files
- **Consolidate all 13 existing migrations** into a single `migrations/0001_init.sql` that creates the full v0.1.0 schema
- **Update `build.rs`** to embed migration files at compile time (preserving single-binary deployment)
- **Refactor `db/mod.rs` `migrate()`** to read from embedded migration files instead of hardcoded strings
- **Future migrations** will follow the naming convention `NNNN_version_description.sql` (e.g., `0002_v0.2.0_add_webhooks.sql`)

## Capabilities

### New Capabilities
- `migration-system`: How database migrations are stored as files, embedded at compile time, and applied at startup

### Modified Capabilities

## Impact

- **migrations/** — new directory with `0001_init.sql`
- **src/db/mod.rs** — `migrate()` refactored to use embedded files
- **build.rs** — updated to embed migration files
- **No schema changes** — the resulting database is identical to current
