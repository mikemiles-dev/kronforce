## Why

As the number of jobs grows, the flat jobs list becomes hard to navigate. Operators managing dozens or hundreds of jobs need a way to organize them into logical groups (e.g., "ETL", "Monitoring", "Deployments") so they can quickly find, filter, and manage related jobs together. Job groups provide this organizational structure without changing how jobs execute.

## What Changes

- Add an optional `group` field to jobs — a simple string label (e.g., "ETL", "Monitoring", "Deploys")
- Group is set on job creation/update via API and UI
- Jobs list page gets a group filter dropdown and visual group badges on each job row
- Dashboard charts include a new "Job Groups" donut chart showing job count per group
- Jobs with no group are shown as "Ungrouped"
- Groups are derived from job data — no separate groups table. Creating a job with a new group name automatically makes that group available
- Add `GET /api/jobs/groups` endpoint returning the list of distinct group names for filter dropdowns
- Bulk action: assign selected jobs to a group

## Capabilities

### New Capabilities
- `job-groups`: Optional group label on jobs, group-based filtering in the jobs list, group filter dropdown, group badge display, bulk group assignment, and groups API endpoint

### Modified Capabilities

## Impact

- **Database**: New `group_name` column on `jobs` table via migration (nullable TEXT, no index needed for low cardinality)
- **Backend**: `Job` struct gets `group: Option<String>`. `CreateJobRequest` and `UpdateJobRequest` get optional `group` field. New `GET /api/jobs/groups` endpoint. Existing job list/filter queries gain group filter support.
- **Frontend**: Jobs page gets group filter dropdown, group badge column, bulk "Set Group" action. Dashboard charts tab gets a 4th donut chart. Job create/edit modal gets a group input field.
- **No breaking changes**: Group is optional and defaults to null. Existing jobs and API calls are unaffected.
