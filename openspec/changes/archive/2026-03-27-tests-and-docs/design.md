## Context

84 tests exist across 6 test files. Core scheduling, execution, notification, and API layers are untested. Doc comments exist on ~7 public items out of 150+.

## Goals / Non-Goals

**Goals:**
- Add tests for all untested utility/helper code that can be tested without mocking external systems
- Add doc comments to all public structs, enums, functions, and methods
- Reach meaningful coverage of the testable code paths

**Non-Goals:**
- Full integration tests for API endpoints (requires test HTTP server setup — separate effort)
- Tests requiring external services (SMTP, SMS webhooks)
- Tests for the Scheduler and Executor run loops (require complex async test harness)
- 100% line coverage — focus on meaningful, maintainable tests

## Decisions

### 1. Test scope: unit-testable code only

Focus on functions that can be tested with an in-memory DB or pure logic:
- Cron parser edge cases (complex expressions, boundary dates)
- Config parsing (env var defaults and overrides)
- Error mapping (AppError → HTTP status codes)
- QueryFilters builder
- Factory methods (ApiKey::bootstrap, ExecutionRecord::new)
- DAG dependency satisfaction
- process_post_execution integration (with in-memory DB)
- notify_execution_complete logic (should-notify decision, not actual send)

Skip: async executor loops, HTTP handler integration, email/SMS sending.

### 2. Doc comment style

Use `///` on every `pub` item. Keep it concise — one line for simple items, 2-3 for complex ones. Include `# Examples` only where the usage is non-obvious.

### 3. Test organization

- Expand existing test files where the module already has tests
- Create new test files for modules with zero tests: `cron_parser_tests.rs`, `config_tests.rs`, `error_tests.rs`, `helpers_tests.rs`
- Keep tests focused — one test per behavior, descriptive names

## Risks / Trade-offs

- **Doc comments add maintenance burden** — If code changes but docs don't, they become misleading. → Keep docs terse so they're less likely to drift.
- **Some tests are hard without mocks** — Notifications, scheduler ticks, HTTP handlers. → Skip these rather than write brittle tests.
