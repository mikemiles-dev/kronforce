## 1. Backend — Chart Data API

- [x] 1.1 Add `get_chart_stats` method to `Db` in `src/db/executions.rs` that runs a single `SELECT status, COUNT(*) FROM executions GROUP BY status` query and returns a `HashMap<String, u32>` of execution outcome counts
- [x] 1.2 Add `get_task_type_counts` and `get_schedule_type_counts` methods to `Db` in `src/db/jobs.rs` that query job task_json and schedule_json columns and return type distribution maps
- [x] 1.3 Create `src/api/stats.rs` module with a `chart_stats` handler for `GET /api/stats/charts` that calls the three DB methods and returns the combined JSON response
- [x] 1.4 Register the `/api/stats/charts` route in `src/api/mod.rs` under the authenticated router

## 2. Frontend — Donut Chart Component

- [x] 2.1 Create `web/js/charts.js` with a `renderDonutChart(containerId, data, title)` function that generates SVG donut using `<circle>` stroke-dasharray segments, a center total label, and a legend
- [x] 2.2 Implement the 6-segment cap logic: sort categories by count descending, take top 5, group the rest into "Other"
- [x] 2.3 Implement the empty state: render a "No data" message when all values are zero or data is empty
- [x] 2.4 Define the color palette array mapping segment index to CSS custom properties (`--success`, `--danger`, `--warning`, `--info`, `--accent`, and a muted neutral)

## 3. Frontend — Dashboard Integration

- [x] 3.1 Add the chart cards row to `web/partials/views/dashboard.html` between the stats bar and the timeline card, with three `<div>` containers: `dash-chart-outcomes`, `dash-chart-tasks`, `dash-chart-schedules`
- [x] 3.2 Add `fetchChartStats()` function in `web/js/dashboard.js` that fetches `GET /api/stats/charts` and calls `renderDonutChart` for each of the three containers
- [x] 3.3 Call `fetchChartStats()` from `renderDashboard()` alongside the existing parallel data fetches
- [x] 3.4 Handle fetch failure gracefully — catch errors and show empty state in chart containers without breaking the rest of the dashboard

## 4. Styling

- [x] 4.1 Add `.dash-charts` grid CSS to `web/css/style.css` for the three-column chart card layout with responsive stacking on narrow viewports
- [x] 4.2 Add `.donut-chart`, `.donut-legend`, and `.donut-center-label` CSS classes for chart component styling
- [x] 4.3 Verify charts render correctly in both light and dark themes

## 5. Build & Test

- [x] 5.1 Verify `build.rs` picks up the new `web/js/charts.js` file in the asset bundling
- [x] 5.2 Run `cargo check` and `cargo test` to verify backend compiles and existing tests pass
- [x] 5.3 Manually test the dashboard with jobs in various states to verify all three charts render correctly
