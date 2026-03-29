## 1. Backend — Rename Group API

- [x] 1.1 Add `Db::rename_group(old_name, new_name)` method in `src/db/jobs.rs` that executes `UPDATE jobs SET group_name = ?1 WHERE group_name = ?2` and returns the count of updated rows
- [x] 1.2 Add `rename_group` handler in `src/api/jobs.rs` for `PUT /api/jobs/rename-group` with write access check and group name validation
- [x] 1.3 Register the `/api/jobs/rename-group` route in `src/api/mod.rs`
- [x] 1.4 Add audit recording for `group.renamed` in the rename handler

## 2. Frontend — Groups Page

- [x] 2.1 Create `web/partials/views/groups.html` with a `<section id="groups-view">` containing a header and a `<div id="groups-grid">` container
- [x] 2.2 Create `web/js/groups.js` with `fetchGroupsPage()` that fetches `GET /api/jobs/groups` and `GET /api/jobs?per_page=100` to compute per-group job counts, then renders group cards with name, count, color dot, rename button, and delete button, plus an "Ungrouped" card
- [x] 2.3 Add `renameGroup(oldName)` function that prompts for new name and calls `PUT /api/jobs/rename-group`
- [x] 2.4 Add `deleteGroup(name)` function that confirms then calls `PUT /api/jobs/bulk-group` with null for all jobs in that group
- [x] 2.5 Add click handler on group cards to navigate to jobs page with group filter: `groupFilter = name; showPage('jobs');`

## 3. Frontend — Routing and Sidebar

- [x] 3.1 Add `'groups'` to the `ALL_VIEWS` array in `web/js/app.js`
- [x] 3.2 Add `groups` case to the `showPage()` function in `web/js/app.js` that calls `fetchGroupsPage()`
- [x] 3.3 Add "Groups" sidebar entry in `web/partials/sidebar.html` after the "Jobs" entry with a sub-entry style (slightly indented, smaller icon)

## 4. Frontend — Modal Improvement

- [x] 4.1 Move the group input field (`f-group` + `group-suggestions` datalist) from the Advanced tab to the main tab in `web/partials/modals.html`, positioned after the Name field and before the task type section
- [x] 4.2 Remove the group field from the Advanced tab to avoid duplication

## 5. Frontend — Dashboard Group Summary

- [x] 5.1 Add a "Top Groups" card to the dashboard Overview tab in `web/partials/views/dashboard.html` inside the overview tab panel
- [x] 5.2 Add `renderDashGroupSummary(jobs)` function in `web/js/dashboard.js` that computes top 5 groups from the jobs data and renders a compact list with group name, count, colored dot, and click-to-filter links
- [x] 5.3 Call `renderDashGroupSummary(jobs)` from `renderDashboard()` after the jobs data is fetched

## 6. Styling

- [x] 6.1 Add `.groups-grid` responsive grid CSS in `web/css/style.css` (similar to `dash-grid` but 3-4 columns)
- [x] 6.2 Add `.group-card` CSS for each card: background, border, padding, hover effect, flex layout with name/count and action buttons
- [x] 6.3 Add `.nav-tab-sub` CSS for the indented sidebar sub-entry

## 7. Verify

- [x] 7.1 Run `cargo check` and `cargo test` to verify compilation and all existing tests pass
- [x] 7.2 Run `cargo clippy` to verify no new warnings
