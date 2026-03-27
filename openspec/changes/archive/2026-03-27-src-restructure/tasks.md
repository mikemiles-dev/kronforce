## 1. Web Directory Setup

- [x] 1.1 Create `web/`, `web/css/`, `web/js/` directories
- [x] 1.2 Extract CSS from `dashboard.html` (lines ~7-2194) into `web/css/style.css`
- [x] 1.3 Create `web/index.html` with HTML structure only, replacing CSS block with `<!-- INJECT:CSS -->` and JS block with `<!-- INJECT:JS -->`

## 2. JavaScript Split

- [x] 2.1 Extract `web/js/app.js` — API client (`api()`), routing (`showPage`, `handleRoute`), auth (`checkAuth`, `doLogin`, `doLogout`), display utils (`toast`, `badge`, `esc`, `fmtDate`, `fmtDate`), search filter factory, health check, time range picker, initialization/bootstrap code
- [x] 2.2 Extract `web/js/dashboard.js` — dashboard stats rendering, execution timeline chart, recent activity panels
- [x] 2.3 Extract `web/js/jobs.js` — job list, detail, create/edit form, delete, copy, bulk operations, sort, pagination, task form builder, task detail formatter
- [x] 2.4 Extract `web/js/executions.js` — execution list, detail, output rendering (text/json/html tabs), output diff, cancel, poll for result
- [x] 2.5 Extract `web/js/agents.js` — agent list, detail, task type editor, unpairing, agent name caching, agent select population
- [x] 2.6 Extract `web/js/scripts.js` — script list, editor, create, save, delete, syntax highlighting
- [x] 2.7 Extract `web/js/events.js` — event list, filtering, rendering, event icons
- [x] 2.8 Extract `web/js/variables.js` — variable CRUD, table rendering, add form, inline edit
- [x] 2.9 Extract `web/js/settings.js` — settings tabs, theme switching, retention, API key management, notification config, test notification
- [x] 2.10 Extract `web/js/modals.js` — modal open/close, cron builder (buildCronFromUI, parseCronToUI, switchCronMode), output rules editor (extraction/trigger/assertion rows), job notification config, wizard, empty states

## 3. Build System

- [x] 3.1 Create `build.rs` that reads `web/index.html`, injects CSS at `<!-- INJECT:CSS -->` and concatenated JS at `<!-- INJECT:JS -->`, writes to `$OUT_DIR/dashboard.html`
- [x] 3.2 Add `cargo::rerun-if-changed` directives for all files in `web/`
- [x] 3.3 Ensure `app.js` is concatenated first, remaining JS files after
- [x] 3.4 Update `src/api/mod.rs` to use `include_str!(concat!(env!("OUT_DIR"), "/dashboard.html"))` instead of `include_str!("../dashboard.html")`

## 4. Cleanup

- [x] 4.1 Delete `src/dashboard.html`
- [x] 4.2 Update `.dockerignore` if needed (exclude `web/` from docker build context if not needed at runtime)
- [x] 4.3 Update `.gitignore` if needed

## 5. Verification

- [x] 5.1 Verify `cargo build` succeeds and produces a working binary
- [x] 5.2 Verify `cargo test --all` passes
- [x] 5.3 Verify `cargo clippy --all-targets` has no warnings
- [x] 5.4 Verify the served HTML at `GET /` is functionally identical (spot-check pages: dashboard, jobs, agents, variables, settings)
