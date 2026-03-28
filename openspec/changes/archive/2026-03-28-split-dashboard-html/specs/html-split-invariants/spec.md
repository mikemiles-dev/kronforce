## ADDED Requirements

### Requirement: Bundled dashboard output SHALL be functionally identical
The final `dashboard.html` produced by `build.rs` SHALL render and behave identically to the pre-split version. All views, modals, action bars, and interactive elements SHALL work as before.

#### Scenario: Dashboard loads and renders all views
- **WHEN** the split partials are assembled by build.rs
- **THEN** the served dashboard at `/` SHALL display all 10 views, 4 action bars, 6 modals, and login screen without errors

#### Scenario: Build succeeds with partials
- **WHEN** `cargo build` is run after the HTML split
- **THEN** the build SHALL succeed and produce a valid `dashboard.html` in OUT_DIR

### Requirement: Partial files SHALL be self-contained HTML fragments
Each partial file SHALL contain a complete, well-formed HTML fragment that can be understood in isolation without needing to read surrounding context.

#### Scenario: Partial contains complete sections
- **WHEN** a view partial like `views/jobs.html` is opened
- **THEN** it SHALL contain complete `<section>` elements with all their child elements, not split mid-tag
