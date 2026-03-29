## Context

Jobs are stored in the `jobs` table with a flat structure — no grouping or categorization field. The jobs list UI shows all jobs in a single sortable, filterable table with status filter buttons and text search. The `CreateJobRequest` and `UpdateJobRequest` structs define what fields can be set via API. The `Job` struct has 15 fields and is serialized to JSON for API responses.

Existing filtering uses `QueryFilters` helpers in `src/db/helpers.rs` which build WHERE clauses dynamically. The jobs list API supports `status` and `search` query params.

## Goals / Non-Goals

**Goals:**
- Add a lightweight group label to organize jobs
- Filter jobs by group in both API and UI
- Auto-discover groups from existing job data (no admin setup)
- Visual group badges in the jobs list
- Bulk assign jobs to a group
- Group distribution donut chart on dashboard

**Non-Goals:**
- Nested/hierarchical groups (folders within folders) — one level is sufficient
- Group-level permissions or access control
- Group-level scheduling or bulk execution
- Mandatory grouping — group remains optional
- Group descriptions, colors, or metadata — keep it a simple string

## Decisions

### 1. Simple nullable `group_name` column on jobs table

```sql
ALTER TABLE jobs ADD COLUMN group_name TEXT;
```

No separate groups table. Groups are just the set of distinct `group_name` values across all jobs. This is the simplest possible model — no foreign keys, no orphan cleanup, no CRUD endpoints for groups themselves.

**Alternatives considered:**
- Separate `groups` table with FK: Over-engineered for a label. Requires group CRUD, orphan handling, ordering. No benefit for single-level grouping.
- Tags (array field): More flexible but harder to filter in SQLite, and the UI concept is "one group per job" not "many tags per job".
- JSON array column: SQLite JSON queries are slower and harder to index.

### 2. `GET /api/jobs/groups` returns distinct group names

A simple query: `SELECT DISTINCT group_name FROM jobs WHERE group_name IS NOT NULL ORDER BY group_name`. Returns `string[]`. Used by the frontend to populate the group filter dropdown.

### 3. Group filter added to existing job list query

Add `group` query parameter to `GET /api/jobs`. The `QueryFilters` helper gets a new `add_eq("group_name", value)` call. Filtering by `group=Ungrouped` maps to `WHERE group_name IS NULL`.

### 4. Group name validation

Group names must be 1-50 characters, alphanumeric plus spaces, hyphens, and underscores. Same style as job name validation. Empty string is treated as null (remove from group).

### 5. Bulk group assignment via `PUT /api/jobs/bulk-group`

```json
{
  "job_ids": ["uuid1", "uuid2"],
  "group": "ETL"
}
```

Setting `group` to `null` or `""` removes jobs from their group. This reuses the existing bulk action pattern in the UI (checkbox selection + action button).

### 6. Frontend group filter as a dropdown in the action bar

A `<select>` dropdown in the jobs action bar, next to the existing status filter buttons. Options populated from `GET /api/jobs/groups` plus "All" and "Ungrouped". Selecting a group re-fetches the jobs list with `?group=<name>`.

### 7. Group badge in job list rows

A small colored pill badge showing the group name next to the job name in the table. Uses a hash of the group name to pick a consistent accent color from a small palette.

## Risks / Trade-offs

- **No index on group_name** → With <10k jobs, a full scan on a TEXT column is negligible. If needed later, `CREATE INDEX idx_jobs_group ON jobs(group_name)` is trivial.
- **Group renaming requires updating each job** → No cascade rename. Acceptable for a lightweight feature. Can bulk-update via API if needed.
- **Distinct query for group list** → Runs on every filter dropdown open. Fast with small job counts. Could cache if it becomes slow.
