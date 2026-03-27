## MODIFIED Requirements

### Requirement: Each submodule under 300 lines
Each new submodule file SHALL be under 300 lines to ensure focused, navigable code. This limit does NOT apply to `web/` files (HTML, CSS, JS) which are organized by feature rather than line count, or to `executor/local.rs` which contains task execution for all task types.

#### Scenario: No oversized submodules
- **WHEN** all splits are complete
- **THEN** no individual Rust submodule file exceeds 300 lines, except `executor/local.rs` which may exceed this due to task type execution handlers

#### Scenario: Web files exempt from line limit
- **WHEN** examining web asset files
- **THEN** `web/css/style.css` and `web/js/*.js` files are not subject to the 300-line limit
