## Why

Job groups were added as a backend feature with basic UI support (filter dropdown, badge on job rows, text input in the Advanced tab). But the current experience is minimal — there's no dedicated page to browse groups at a glance, the group field is buried in the Advanced tab of the modal, and there's no sidebar entry to navigate to groups. Operators need better discoverability and a first-class groups experience to actually use the feature.

## What Changes

- Add a **Groups page** showing all groups as cards with job counts, click to filter jobs by that group
- Add a **sidebar sub-entry** under Jobs for "Groups" to navigate directly to the groups page
- Move the **group field** in the job create/edit modal from the Advanced tab to the main (first) tab, right under the Name field, for better visibility
- Add **group rename** capability — a button on each group card to rename the group (bulk-updates all jobs in that group)
- Add **group delete** capability — removes the group label from all jobs in that group (sets to ungrouped)
- Show a **group summary card** on the dashboard Overview tab with top groups and job counts

## Capabilities

### New Capabilities
- `job-groups-ui`: Dedicated groups page, sidebar navigation, group rename/delete, improved modal placement, and dashboard group summary

### Modified Capabilities

## Impact

- **Backend**: New `PUT /api/jobs/rename-group` endpoint for bulk rename. No new tables — still uses `group_name` column on jobs.
- **Frontend**: New `web/partials/views/groups.html` for the groups page. New `web/js/groups.js` for group page logic. Updated sidebar, modal, and dashboard. Updated `web/js/app.js` for routing.
- **CSS**: Group card styles added to `web/css/style.css`.
- **No breaking changes**: Additive UI changes only.
