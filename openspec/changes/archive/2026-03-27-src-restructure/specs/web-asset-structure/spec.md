## ADDED Requirements

### Requirement: Web assets live in a top-level web/ directory
All web UI source files (HTML, CSS, JavaScript) SHALL be stored in a `web/` directory at the project root, separate from Rust source code.

#### Scenario: Web directory structure
- **WHEN** the restructure is complete
- **THEN** the `web/` directory contains `index.html`, `css/style.css`, and multiple `.js` files under `js/`

#### Scenario: No web assets in src/
- **WHEN** looking at the `src/` directory
- **THEN** there is no `dashboard.html` or other HTML/CSS/JS files

### Requirement: HTML structure is a standalone file
The `web/index.html` file SHALL contain only HTML markup with `<!-- INJECT:CSS -->` and `<!-- INJECT:JS -->` placeholder comments where styles and scripts are injected at build time.

#### Scenario: HTML contains no inline styles or scripts
- **WHEN** examining `web/index.html`
- **THEN** there are no `<style>` blocks or `<script>` blocks with application code, only the injection placeholders

#### Scenario: Injection placeholders present
- **WHEN** `build.rs` processes `web/index.html`
- **THEN** it finds `<!-- INJECT:CSS -->` and replaces it with `<style>{contents of css/style.css}</style>`
- **AND** it finds `<!-- INJECT:JS -->` and replaces it with `<script>{concatenated JS files}</script>`

### Requirement: CSS extracted into a single stylesheet
All CSS from the original `dashboard.html` SHALL be extracted into `web/css/style.css` with no modifications to selectors, properties, or theme variables.

#### Scenario: CSS content preserved
- **WHEN** comparing the extracted CSS to the original
- **THEN** all CSS rules, media queries, and CSS custom properties are identical

### Requirement: JavaScript split into feature modules
The JavaScript from `dashboard.html` SHALL be split into multiple files under `web/js/`, organized by feature area.

#### Scenario: Core utilities in app.js
- **WHEN** the JS split is complete
- **THEN** `web/js/app.js` contains the API client (`api()`), routing (`showPage`, `handleRoute`), auth (`checkAuth`, `doLogin`, `doLogout`), display utilities (`toast`, `badge`, `esc`, `fmtDate`), and initialization code

#### Scenario: Jobs page in jobs.js
- **WHEN** the JS split is complete
- **THEN** `web/js/jobs.js` contains job listing, detail, creation, editing, deletion, bulk operations, and form submission functions

#### Scenario: Executions page in executions.js
- **WHEN** the JS split is complete
- **THEN** `web/js/executions.js` contains execution listing, detail, output rendering, diff comparison, and cancellation functions

#### Scenario: Agents page in agents.js
- **WHEN** the JS split is complete
- **THEN** `web/js/agents.js` contains agent listing, task type editor, unpairing, and agent selection functions

#### Scenario: Scripts page in scripts.js
- **WHEN** the JS split is complete
- **THEN** `web/js/scripts.js` contains script listing, editor, creation, saving, and deletion functions

#### Scenario: Events page in events.js
- **WHEN** the JS split is complete
- **THEN** `web/js/events.js` contains event listing, filtering, and rendering functions

#### Scenario: Variables page in variables.js
- **WHEN** the JS split is complete
- **THEN** `web/js/variables.js` contains variable CRUD and rendering functions

#### Scenario: Settings page in settings.js
- **WHEN** the JS split is complete
- **THEN** `web/js/settings.js` contains settings tabs, theme switching, API key management, notification config, and retention settings

#### Scenario: Modal and form utilities in modals.js
- **WHEN** the JS split is complete
- **THEN** `web/js/modals.js` contains modal open/close, cron builder, output rules editor, task form builder, and job form utilities

#### Scenario: Dashboard stats in dashboard.js
- **WHEN** the JS split is complete
- **THEN** `web/js/dashboard.js` contains dashboard overview rendering, stats fetching, and timeline chart functions

### Requirement: Build-time concatenation via build.rs
A `build.rs` script SHALL concatenate all web assets into a single HTML string at compile time, written to `OUT_DIR/dashboard.html`.

#### Scenario: build.rs produces valid HTML
- **WHEN** `cargo build` runs
- **THEN** `build.rs` reads `web/index.html`, injects CSS and JS at the placeholder comments, and writes the result to `$OUT_DIR/dashboard.html`

#### Scenario: JS load order is correct
- **WHEN** `build.rs` concatenates JavaScript files
- **THEN** `app.js` is concatenated first, followed by the remaining JS files, ensuring shared utilities are defined before page modules use them

#### Scenario: Cargo rebuilds on web asset changes
- **WHEN** any file in `web/` is modified
- **THEN** `build.rs` uses `cargo::rerun-if-changed` directives for all web files so cargo triggers a rebuild

#### Scenario: api/mod.rs references built output
- **WHEN** the dashboard is served
- **THEN** `api/mod.rs` uses `include_str!(concat!(env!("OUT_DIR"), "/dashboard.html"))` instead of `include_str!("../dashboard.html")`

### Requirement: Served HTML is identical to the original
The concatenated HTML served at `GET /` SHALL be functionally identical to the original `dashboard.html` — same CSS, same HTML structure, same JavaScript behavior.

#### Scenario: Visual regression check
- **WHEN** the restructured dashboard loads in a browser
- **THEN** all pages render identically to the original (dashboard, jobs, agents, executions, scripts, events, variables, settings, docs)

#### Scenario: All JavaScript functions work
- **WHEN** interacting with the restructured dashboard
- **THEN** job CRUD, execution viewing, agent management, script editing, variable management, settings, and all other features work as before
