## Why

The dashboard currently renders all content in a single vertical scroll — stats cards, three chart cards, execution timeline, recent executions, dependency map, recent events, and agents summary. On typical screens this requires significant scrolling to see all sections, and operators often only care about one or two sections at a time. Organizing the dashboard into tabs reduces visual clutter and lets users jump directly to the information they need.

## What Changes

- Add a tab bar below the stats cards with tabs: **Overview**, **Charts**, **Activity**, **Infrastructure**
- **Overview** tab shows the stats cards (always visible above tabs), execution timeline, and recent executions — the most commonly needed at-a-glance info
- **Charts** tab shows the three donut charts (execution outcomes, task types, schedule types)
- **Activity** tab shows recent events and the execution timeline with a longer default window
- **Infrastructure** tab shows agents summary and the dependency map
- Stats cards remain above the tab bar, always visible regardless of active tab
- Active tab is remembered during the session (resets on page reload)
- Tab selection does not trigger additional API calls — all data is fetched once on dashboard load, tabs just show/hide sections

## Capabilities

### New Capabilities
- `dashboard-tabs`: Tab navigation component for the dashboard, including tab bar rendering, content panel switching, and layout restructuring of existing dashboard cards into tab groups

### Modified Capabilities

## Impact

- **Frontend**: `web/partials/views/dashboard.html` restructured with tab containers. `web/js/dashboard.js` updated with tab switching logic. New CSS for tab bar styling in `web/css/style.css`.
- **Backend**: No changes — all data is already fetched by the existing `renderDashboard()` function.
- **No breaking changes**: The same data is displayed, just reorganized into tabs.
