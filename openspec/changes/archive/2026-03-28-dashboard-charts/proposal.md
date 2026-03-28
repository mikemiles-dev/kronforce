## Why

The Kronforce dashboard currently shows only numeric stat cards and a bar-based timeline chart. There is no visual breakdown of job composition (by task type, schedule type) or execution outcomes (success vs failure ratios). Pie/donut charts would give operators an immediate visual read on system health and workload distribution without needing to scan through tables.

## What Changes

- Add a **pie/donut chart for execution outcomes** (succeeded, failed, timed out, cancelled) on the dashboard, computed from aggregate execution counts already available from the jobs API
- Add a **pie/donut chart for job task type distribution** (Shell, HTTP, Script, SQL, FTP, Kafka, etc.) showing how many jobs use each task type
- Add a **pie/donut chart for job schedule type distribution** (Cron, OnDemand, OneShot, Event) showing the mix of scheduling strategies
- Add a new API endpoint to return chart-ready aggregate stats so the frontend doesn't need to re-derive them from the full jobs list
- Implement charts using pure SVG rendering in vanilla JS (no external chart library) to match the existing codebase approach used by the timeline chart and dependency map
- Add a new row of chart cards to the dashboard layout between the stats bar and the timeline

## Capabilities

### New Capabilities
- `dashboard-charts`: SVG-based pie/donut chart rendering for the dashboard, including chart data aggregation API endpoint, frontend chart component, and dashboard layout integration

### Modified Capabilities

## Impact

- **Backend**: New API endpoint (`GET /api/stats/charts`) in `src/api/` that aggregates job/execution data into chart-friendly payloads. Queries existing DB functions — no new tables or migrations needed.
- **Frontend**: New JS file `web/js/charts.js` for SVG pie/donut chart rendering. Updated `web/partials/views/dashboard.html` for chart card layout. Updated `web/js/dashboard.js` to fetch and render chart data.
- **CSS**: Minor additions to `web/css/style.css` for chart card styling and responsive layout.
- **Build**: `build.rs` will pick up the new JS file automatically via the existing asset bundling.
- **Dependencies**: None — pure SVG rendering, no new crates or libraries.
