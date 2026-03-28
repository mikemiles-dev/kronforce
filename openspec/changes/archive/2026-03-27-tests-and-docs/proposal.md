## Why

The codebase has 89 tests covering ~30% of code by line count. Critical areas like the Scheduler, Executor, config parsing, error handling, and factory methods have zero tests. Doc comment coverage is ~5% — only 7 out of 150+ public items have `///` documentation. This makes the code harder to maintain and onboard new contributors.

## What Changes

- **Add tests** for untested areas: cron parser edge cases, config parsing, error mapping, QueryFilters, factory methods (ApiKey::bootstrap, ExecutionRecord::new), process_post_execution integration, and DAG deps_satisfied
- **Add doc comments** to all public structs, enums, functions, and methods in the core modules: models, db, executor, config, error, dag, scheduler, notifications, api

## Capabilities

### New Capabilities

### Modified Capabilities

## Impact

- **tests/** — new test files and expanded existing ones
- **src/db/models.rs** — doc comments on all public items
- **src/db/mod.rs** — doc comments on Db, db_call, with_transaction
- **src/executor/*.rs** — doc comments on public functions
- **src/config.rs** — doc comments
- **src/error.rs** — doc comments
- **src/dag.rs** — doc comments
- **src/scheduler/*.rs** — doc comments
- **No behavior changes** — only tests and documentation
