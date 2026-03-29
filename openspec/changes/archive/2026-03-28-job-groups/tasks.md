## 1. Database Migration

- [x] 1.1 Create `migrations/0003_job_groups.sql` that adds a nullable `group_name TEXT` column to the `jobs` table

## 2. Backend — Model and DB

- [x] 2.1 Add `group: Option<String>` field to the `Job` struct in `src/db/models/job.rs` (with `#[serde(default)]`)
- [x] 2.2 Update `Job::from_row` to read the `group_name` column from the database row
- [x] 2.3 Update `Db::insert_job` and `Db::update_job` in `src/db/jobs.rs` to include `group_name` in INSERT/UPDATE queries
- [x] 2.4 Add `group_name` to the SELECT column lists in `get_job`, `list_jobs`, and `get_all_jobs_for_dag` queries
- [x] 2.5 Add group filter support to `build_job_filters` — when `group` param is provided, add `group_name = ?` (or `group_name IS NULL` for "ungrouped")
- [x] 2.6 Add `Db::get_distinct_groups()` method that returns `Vec<String>` from `SELECT DISTINCT group_name FROM jobs WHERE group_name IS NOT NULL ORDER BY group_name`
- [x] 2.7 Add `Db::bulk_set_group()` method that updates `group_name` for a list of job UUIDs

## 3. Backend — API

- [x] 3.1 Add `group: Option<String>` to `CreateJobRequest` and `UpdateJobRequest` in `src/api/jobs.rs`
- [x] 3.2 Add `validate_group_name()` function (1-50 chars, alphanumeric + spaces/hyphens/underscores, empty string → None)
- [x] 3.3 Wire group into `create_job` and `update_job` handlers (validate, set on Job struct)
- [x] 3.4 Add `group` query parameter to `ListJobsQuery` and pass it to `build_job_filters`
- [x] 3.5 Add `list_groups` handler for `GET /api/jobs/groups` that returns `Json<Vec<String>>`
- [x] 3.6 Add `bulk_set_group` handler for `PUT /api/jobs/bulk-group` with write access check
- [x] 3.7 Register `/api/jobs/groups` (GET) and `/api/jobs/bulk-group` (PUT) routes in `src/api/mod.rs`

## 4. Frontend — Jobs List

- [x] 4.1 Add group filter `<select>` dropdown to the jobs action bar in `web/partials/action-bars.html` with id `group-filter`
- [x] 4.2 Add `fetchGroups()` function in `web/js/jobs.js` that fetches `GET /api/jobs/groups` and populates the dropdown with "All Groups", "Ungrouped", plus group names
- [x] 4.3 Add `groupFilter` to the jobs search state and wire the dropdown `onchange` to re-fetch jobs with `?group=<value>`
- [x] 4.4 Add group badge rendering in job table rows — a colored pill next to the job name using a hash-based color from a small palette
- [x] 4.5 Add "Set Group" bulk action button that prompts for a group name and calls `PUT /api/jobs/bulk-group`

## 5. Frontend — Job Modal

- [x] 5.1 Add a group text input field to the job create/edit modal in `web/partials/modals.html`
- [x] 5.2 Wire the group field into modal open (populate from job data) and save (include in request body)
- [x] 5.3 Add autocomplete suggestions for the group input from the cached groups list

## 6. Styling

- [x] 6.1 Add `.group-badge` CSS for the colored pill badge in `web/css/style.css`
- [x] 6.2 Add `groupColor(name)` JS helper that returns a consistent color from a hash of the group name

## 7. Verify

- [x] 7.1 Run `cargo check` and `cargo test` to verify compilation and all existing tests pass
- [x] 7.2 Run `cargo clippy` to verify no new warnings
