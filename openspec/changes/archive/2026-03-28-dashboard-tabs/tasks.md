## 1. HTML — Restructure Dashboard Layout

- [x] 1.1 Add tab bar HTML after `#dash-stats` in `web/partials/views/dashboard.html` with four buttons: Overview, Charts, Activity, Infrastructure
- [x] 1.2 Wrap existing dashboard cards into four `<div data-tab="...">` panels: `overview` (timeline + recent execs), `charts` (three donut chart cards), `activity` (recent events), `infrastructure` (agents + dependency map)
- [x] 1.3 Keep `#dash-stats` outside and above the tab bar so it's always visible

## 2. CSS — Tab Bar and Panel Styling

- [x] 2.1 Add `.dash-tab-bar` CSS for horizontal flex layout with gap, matching the existing filter button style
- [x] 2.2 Add `.dash-tab-btn` and `.dash-tab-btn.active` CSS using `--accent` for active state
- [x] 2.3 Add `.dash-tab-panel` CSS with `display: none` default and `.dash-tab-panel.active` with `display: block`
- [x] 2.4 Add responsive rule for narrow viewports: `overflow-x: auto` on the tab bar for horizontal scrolling

## 3. JavaScript — Tab Switching Logic

- [x] 3.1 Add a `currentDashTab` session variable (default `'overview'`) in `web/js/dashboard.js`
- [x] 3.2 Add `showDashTab(tabName)` function that hides all `.dash-tab-panel` elements, shows the matching one, and updates `.dash-tab-btn.active` state
- [x] 3.3 Call `showDashTab(currentDashTab)` at the end of `renderDashboard()` to restore the active tab after re-rendering
- [x] 3.4 Wire `onclick` handlers on the tab buttons to call `showDashTab()`

## 4. Verify

- [x] 4.1 Run `cargo check` to verify build picks up HTML/CSS/JS changes
- [x] 4.2 Verify all four tabs show correct content and switching is instant with no API calls
- [x] 4.3 Verify stats cards remain visible across all tabs
