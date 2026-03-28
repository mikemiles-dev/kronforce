## Why

The dashboard's `web/index.html` is 1,461 lines containing all 10 page views, 4 action bars, 6 modals, and embedded documentation in a single file. This makes it difficult to navigate, edit specific views, or work on multiple UI areas without merge conflicts. The JavaScript is already split into 10 separate files — the HTML should follow the same pattern.

## What Changes

- Split `web/index.html` into partial HTML files organized by view/component:
  - `web/partials/sidebar.html` — navigation sidebar
  - `web/partials/action-bars.html` — the 4 action bar sections (jobs, agents, events, executions)
  - `web/partials/views/dashboard.html` — dashboard view
  - `web/partials/views/jobs.html` — jobs list + job detail views
  - `web/partials/views/executions.html` — executions view
  - `web/partials/views/agents.html` — agents view
  - `web/partials/views/scripts.html` — scripts view + editor
  - `web/partials/views/events.html` — events view
  - `web/partials/views/variables.html` — variables view
  - `web/partials/views/settings.html` — settings view
  - `web/partials/views/docs.html` — embedded documentation
  - `web/partials/views/map.html` — dependency map view
  - `web/partials/modals.html` — all modal overlays
  - `web/partials/login.html` — login screen
- Update `build.rs` to read all partial files and inject them at marked placeholders in a slimmed-down `index.html` shell
- Keep the final bundled output identical — the served HTML is unchanged

## Capabilities

### New Capabilities

_(none — this is a build-time restructuring with no runtime changes)_

### Modified Capabilities

_(none — the served dashboard HTML is identical before and after)_

## Impact

- `web/index.html` — reduced to a ~30-line shell with placeholder markers
- `web/partials/` — new directory with ~14 partial HTML files
- `build.rs` — updated to read and inject partial files alongside CSS/JS
- No runtime, API, or behavioral changes
- The final bundled `dashboard.html` output is byte-identical
