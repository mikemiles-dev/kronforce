## Context

The dashboard is a single-page view rendered by `renderDashboard()` in `web/js/dashboard.js`. It fetches jobs, events, and agents data in parallel, then renders eight sections sequentially into `#dashboard-view`: stats cards, three chart cards, execution timeline, recent executions, dependency map, recent events, and agents summary. All sections are always rendered and visible, requiring a long vertical scroll.

The HTML structure is in `web/partials/views/dashboard.html`. CSS uses `.dash-stats`, `.dash-charts`, `.dash-grid`, and `.card` classes. The existing `showPage()` routing system in `app.js` manages top-level view switching.

## Goals / Non-Goals

**Goals:**
- Organize dashboard cards into tabs to eliminate scrolling
- Keep stats cards always visible above the tab bar for at-a-glance health
- All data fetched once on dashboard load — tab switching is instant (show/hide only)
- Tab state persists during the session via a JS variable
- Clean, minimal tab bar that matches the existing UI aesthetic

**Non-Goals:**
- URL-based tab routing (hash fragments or query params) — keep it simple with JS state
- Lazy loading per tab (all data is already fetched upfront)
- User-configurable tab order or content
- Persisting tab selection across page reloads (localStorage)

## Decisions

### 1. Pure CSS show/hide via `data-tab` attributes

Each tab panel gets a `data-tab="overview"` attribute. Tab switching sets `display: none` on all panels, then `display: block` on the active one. The tab bar buttons use an `.active` class for highlighting.

**Alternatives considered:**
- CSS-only tabs with `:checked` radio inputs: Fragile, hard to set active tab from JS
- Full re-render per tab: Wasteful since data is already in memory
- Routing integration (hash-based): Over-engineered for an in-page layout change

### 2. Four tabs: Overview, Charts, Activity, Infrastructure

| Tab | Content |
|-----|---------|
| **Overview** (default) | Execution timeline (15 min) + Recent executions table |
| **Charts** | Three donut charts (outcomes, task types, schedule types) |
| **Activity** | Recent events list |
| **Infrastructure** | Agents summary + Dependency map |

**Rationale:** Groups content by what operators care about in different contexts — quick health check (Overview), workload distribution (Charts), audit trail (Activity), system topology (Infrastructure).

**Alternatives considered:**
- Two tabs (Summary / Details): Still too much per tab
- Five+ tabs: Too many tabs for the amount of content; agents and dep map are related enough to share a tab

### 3. Tab bar styling matches existing action bar pattern

The tab bar uses a horizontal flex row with pill-style buttons, similar to the existing status filter buttons in the jobs and executions views. Active tab gets `--accent` background color.

### 4. `renderDashboard()` renders all panels, then shows active tab

The existing `renderDashboard()` function continues to populate all sections. After rendering, a `showDashTab(tabName)` function shows the active panel. This keeps the data flow unchanged — tabs are purely a display concern.

## Risks / Trade-offs

- **All data still fetched upfront** → Acceptable. The total API payload is small (jobs list + events + agents). If this becomes a concern later, lazy loading per tab can be added without changing the tab UI.
- **No deep linking to specific tabs** → If a user shares a dashboard URL, it always opens to Overview. Acceptable for now; hash-based routing can be added later.
- **Content shift when switching tabs** → Panels have different heights. Use a minimum height on the tab content area to prevent layout jank.
