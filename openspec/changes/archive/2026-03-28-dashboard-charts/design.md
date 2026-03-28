## Context

The dashboard currently displays numeric stat cards, a bar-based execution timeline, recent executions/events lists, an agents summary, and a dependency map. All rendering is done with vanilla JS generating HTML/SVG strings — no chart libraries are used. The existing API already returns per-job execution counts (`succeeded`, `failed`, `total`) and task type info via `GET /api/jobs?per_page=100`, which the dashboard fetches on load in `renderDashboard()`.

## Goals / Non-Goals

**Goals:**
- Add three donut charts to the dashboard: execution outcome distribution, job task type distribution, and job schedule type distribution
- Provide a dedicated API endpoint so the frontend doesn't need to derive chart data from the full jobs list
- Keep the implementation consistent with the existing codebase style (vanilla JS, inline SVG, no external dependencies)
- Charts should be responsive and theme-aware (respect CSS custom properties for colors)

**Non-Goals:**
- Interactive drill-down or click-to-filter from chart segments (future enhancement)
- Historical/time-series chart data (the existing timeline already covers this)
- Animated transitions or chart library integration
- Per-agent or per-execution-level charts
- Configurable chart types or user preferences

## Decisions

### 1. Pure SVG donut charts via `<circle>` stroke-dasharray

Render donut charts using SVG `<circle>` elements with `stroke-dasharray` and `stroke-dashoffset` to draw segments. This is the simplest approach for pie/donut charts in SVG and requires no path math.

**Alternatives considered:**
- `<path>` arc segments: More complex math (`arc` commands), harder to maintain
- Canvas 2D: Loses SVG benefits (scalability, CSS theming, DOM inspection)
- Chart.js / D3: Adds external dependency, conflicts with the project's zero-dependency frontend approach

### 2. New `GET /api/stats/charts` endpoint

A single endpoint returns all three chart datasets in one response. This avoids three round-trips and lets the backend do the aggregation efficiently in SQL.

**Response shape:**
```json
{
  "execution_outcomes": { "succeeded": 120, "failed": 15, "timed_out": 3, "cancelled": 1, "running": 2 },
  "task_types": { "Shell": 8, "Http": 5, "Script": 3, "Sql": 2 },
  "schedule_types": { "Cron": 12, "OnDemand": 4, "OneShot": 1, "Event": 1 }
}
```

**Alternatives considered:**
- Derive from existing `/api/jobs` response: Works but wastes bandwidth fetching full job objects just for counts, and couples chart rendering to the jobs list page size
- Three separate endpoints: Unnecessary complexity for three small aggregations

### 3. Dashboard layout: chart row between stats and timeline

Add a new `dash-charts` grid row containing three equal-width chart cards, placed between the stats bar and the timeline. Each card has a title header and the donut chart with a legend.

**Alternatives considered:**
- Charts inside stat cards: Too small for readable segments
- Separate charts page: Reduces discoverability; dashboard is the right home

### 4. Color palette from CSS custom properties

Chart segment colors will use the existing CSS custom properties (`--success`, `--danger`, `--warning`, `--info`, `--accent`) plus a few additional muted colors for less common segments. This ensures charts automatically adapt to light/dark theme.

### 5. Backend aggregation via existing DB queries

The `get_execution_counts` pattern already exists per-job. The new endpoint will run a single `GROUP BY` query across all executions for outcome counts, and iterate the jobs list for task/schedule type counts (which is already loaded in memory for the jobs cache).

## Risks / Trade-offs

- **Stale data on long-open dashboards** → Charts refresh on each `renderDashboard()` call, which happens on page navigation. No auto-refresh, same as existing stat cards. Acceptable for now.
- **Empty state when no jobs/executions exist** → Show a centered "No data" message in each chart card, consistent with existing empty states.
- **Performance with many task types** → Donut charts with too many small segments become unreadable. Cap at 6 segments, grouping the rest into "Other". This keeps charts clean.
- **SVG rendering on very old browsers** → stroke-dasharray is widely supported (IE11+). Not a concern for the target audience (ops engineers).
