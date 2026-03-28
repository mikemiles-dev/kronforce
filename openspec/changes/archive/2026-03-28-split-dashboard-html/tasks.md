## 1. Create partial files from index.html

- [x] 1.1 Create `web/partials/` and `web/partials/views/` directories
- [x] 1.2 Extract login screen into `web/partials/login.html`
- [x] 1.3 Extract sidebar navigation into `web/partials/sidebar.html`
- [x] 1.4 Extract all 4 action bars into `web/partials/action-bars.html`
- [x] 1.5 Extract dashboard view into `web/partials/views/dashboard.html`
- [x] 1.6 Extract jobs list + job detail views into `web/partials/views/jobs.html`
- [x] 1.7 Extract executions view into `web/partials/views/executions.html`
- [x] 1.8 Extract agents view into `web/partials/views/agents.html`
- [x] 1.9 Extract scripts view into `web/partials/views/scripts.html`
- [x] 1.10 Extract events view into `web/partials/views/events.html`
- [x] 1.11 Extract variables view into `web/partials/views/variables.html`
- [x] 1.12 Extract settings view into `web/partials/views/settings.html`
- [x] 1.13 Extract docs view into `web/partials/views/docs.html`
- [x] 1.14 Extract dependency map view into `web/partials/views/map.html`
- [x] 1.15 Extract all modals into `web/partials/modals.html`

## 2. Update index.html to use include markers

- [x] 2.1 Replace extracted sections in `web/index.html` with `<!-- INCLUDE:partials/... -->` placeholders

## 3. Update build.rs to process includes

- [x] 3.1 Add `<!-- INCLUDE:path -->` processing to `build.rs` that reads and injects partial files
- [x] 3.2 Add `cargo::rerun-if-changed` for all partial files

## 4. Verification

- [x] 4.1 Run `cargo build` and verify the build succeeds
- [x] 4.2 Compare the bundled dashboard.html output to confirm it contains all expected content
