## Context

Job groups were implemented as a simple `group_name` column on the jobs table with basic UI: a filter dropdown in the jobs action bar, a colored badge on job rows, and a text input buried in the Advanced tab of the job modal. The backend already has `GET /api/jobs/groups` (distinct group names), `PUT /api/jobs/bulk-group` (bulk assign), and group filtering on `GET /api/jobs?group=X`.

The sidebar currently has a flat list of pages (Dashboard, Jobs, Executions, Map, Agents, Scripts, Events, Variables, Docs, Settings). There are no sub-entries or expandable sections.

The existing `showPage()` function in `app.js` manages view switching via the `ALL_VIEWS` array and toggling display on `<section id="X-view">` elements.

## Goals / Non-Goals

**Goals:**
- Dedicated Groups page with visual group cards showing job counts
- Sidebar "Groups" entry under Jobs for direct navigation
- Move group field to the main tab of job create/edit modal for better visibility
- Group rename (bulk update all jobs in a group)
- Group delete (remove group label from all jobs)
- Dashboard group summary in the Overview tab

**Non-Goals:**
- Nested groups or group hierarchies
- Group-level permissions or role-based access
- Drag-and-drop job assignment to groups
- Group descriptions, icons, or custom colors (keep it simple — just a name)

## Decisions

### 1. Groups page as a new view in the routing system

Add `groups` to `ALL_VIEWS`, create `web/partials/views/groups.html` and `web/js/groups.js`. The page renders group cards in a responsive grid. Each card shows the group name, job count, a color dot (same hash-based color as badges), and rename/delete action buttons.

Clicking a group card navigates to the Jobs page with that group pre-filtered.

### 2. Sidebar sub-entry via indented button

Rather than implementing a collapsible tree (complex), add "Groups" as a regular nav button right after "Jobs" with slightly smaller text and left indent to visually suggest hierarchy. Uses `onclick="showPage('groups')"`.

**Alternatives considered:**
- Collapsible sidebar section: Too complex for one sub-entry. Would need expand/collapse state management.
- Dropdown from Jobs button: Non-standard, hard to discover.

### 3. Group field moved to main tab (above schedule)

In the job create/edit modal, move the group `<input>` + `<datalist>` from the Advanced tab to the first tab ("Task" tab), placed right under the Name field. This makes it immediately visible without switching tabs.

### 4. Rename via `PUT /api/jobs/rename-group`

New endpoint:
```json
PUT /api/jobs/rename-group
{"old_name": "ETL", "new_name": "Data Pipeline"}
```

Executes `UPDATE jobs SET group_name = ?1 WHERE group_name = ?2`. Returns `{"updated": N}`. Requires write access.

**Alternatives considered:**
- Client-side rename (fetch all jobs, update each): Too many API calls, race conditions.
- Dedicated groups table with rename cascade: Over-engineered for a label field.

### 5. Delete group = bulk ungroup

"Delete group" calls `PUT /api/jobs/bulk-group` with `group: null` for all jobs in that group. No new endpoint needed — reuse existing bulk-group API.

### 6. Dashboard group summary as a simple list

Add a "Top Groups" section in the Overview tab showing the top 5 groups by job count as a compact list (group name + count). Links each to the filtered jobs page. Uses data already available from the jobs list fetch.

## Risks / Trade-offs

- **Sidebar length** → Adding another entry makes the sidebar longer. Acceptable — the Groups entry is logically grouped with Jobs and small.
- **Group rename has no undo** → The rename is immediate and affects all jobs. Acceptable for an admin operation. The audit log captures it.
- **Groups page re-fetches on every visit** → `GET /api/jobs/groups` is fast (single distinct query). No caching needed.
