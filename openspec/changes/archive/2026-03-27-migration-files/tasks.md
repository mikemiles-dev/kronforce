## 1. Create Migration Files

- [x] 1.1 Create `migrations/` directory
- [x] 1.2 Create `migrations/0001_init.sql` with version 13 header and the full consolidated schema (all 13 current migrations merged into one clean CREATE TABLE block)

## 2. Update build.rs

- [x] 2.1 Add migration file parsing to `build.rs` — read `migrations/*.sql`, parse `-- version:` and `-- description:` headers, generate a Rust file with a `MIGRATIONS` constant
- [x] 2.2 Add `cargo::rerun-if-changed` directives for `migrations/` directory

## 3. Refactor db/mod.rs

- [x] 3.1 Replace the inline `let migrations: Vec<(i64, &str, &str)> = vec![...]` with the generated `MIGRATIONS` constant
- [x] 3.2 Remove all hardcoded SQL strings from `migrate()`
- [x] 3.3 Ensure the migration application logic (version checking, statement splitting, error handling) still works

## 4. Verification

- [x] 4.1 `cargo build` succeeds
- [x] 4.2 `cargo test --all` passes (fresh DB migrations work)
- [x] 4.3 `cargo clippy --all-targets` clean
- [x] 4.4 Verify a fresh in-memory DB gets schema_version at 13
