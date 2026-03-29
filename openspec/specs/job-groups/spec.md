## ADDED Requirements

### Requirement: Group field on jobs
The `Job` model SHALL have an optional `group` field (nullable string). The field SHALL be stored in a `group_name` column on the `jobs` table added via migration. Jobs without a group SHALL have `group_name` set to NULL.

#### Scenario: Create job with group
- **WHEN** a user creates a job via `POST /api/jobs` with `"group": "ETL"` in the request body
- **THEN** the job is created with `group` set to `"ETL"` and the group appears in the job response

#### Scenario: Create job without group
- **WHEN** a user creates a job via `POST /api/jobs` without a `group` field
- **THEN** the job is created with `group` set to null

#### Scenario: Update job group
- **WHEN** a user updates a job via `PUT /api/jobs/{id}` with `"group": "Monitoring"`
- **THEN** the job's group is changed to `"Monitoring"`

#### Scenario: Remove job from group
- **WHEN** a user updates a job via `PUT /api/jobs/{id}` with `"group": null` or `"group": ""`
- **THEN** the job's group is set to null (ungrouped)

#### Scenario: Existing jobs unaffected by migration
- **WHEN** the migration adds the `group_name` column
- **THEN** all existing jobs have `group_name` set to NULL

### Requirement: Group name validation
Group names SHALL be 1-50 characters and contain only alphanumeric characters, spaces, hyphens, and underscores. An empty string SHALL be treated as null (remove from group).

#### Scenario: Valid group name
- **WHEN** a user sets a job's group to `"ETL Pipeline"`
- **THEN** the group is accepted and saved

#### Scenario: Group name too long
- **WHEN** a user sets a job's group to a string longer than 50 characters
- **THEN** the system returns 400 Bad Request with an error message

#### Scenario: Group name with invalid characters
- **WHEN** a user sets a job's group to `"ETL/Pipeline!"`
- **THEN** the system returns 400 Bad Request with an error message

#### Scenario: Empty string treated as null
- **WHEN** a user sets a job's group to `""`
- **THEN** the job's group is set to null (ungrouped)

### Requirement: List distinct groups endpoint
The system SHALL provide a `GET /api/jobs/groups` endpoint that returns an array of distinct group names across all jobs, sorted alphabetically. The endpoint SHALL require authentication.

#### Scenario: Multiple groups exist
- **WHEN** jobs have groups "ETL", "Monitoring", and "Deploys"
- **THEN** `GET /api/jobs/groups` returns `["Deploys", "ETL", "Monitoring"]`

#### Scenario: No groups exist
- **WHEN** no jobs have a group assigned
- **THEN** `GET /api/jobs/groups` returns `[]`

#### Scenario: Duplicate groups collapsed
- **WHEN** three jobs have group "ETL" and two have group "Monitoring"
- **THEN** `GET /api/jobs/groups` returns `["ETL", "Monitoring"]`

### Requirement: Filter jobs by group
The `GET /api/jobs` endpoint SHALL accept an optional `group` query parameter. When provided, only jobs matching that group SHALL be returned. The special value `ungrouped` SHALL match jobs with no group.

#### Scenario: Filter by group name
- **WHEN** a user requests `GET /api/jobs?group=ETL`
- **THEN** only jobs with `group` equal to `"ETL"` are returned

#### Scenario: Filter for ungrouped jobs
- **WHEN** a user requests `GET /api/jobs?group=ungrouped`
- **THEN** only jobs with `group` set to null are returned

#### Scenario: No group filter
- **WHEN** a user requests `GET /api/jobs` without a `group` parameter
- **THEN** all jobs are returned regardless of group

#### Scenario: Group filter combined with other filters
- **WHEN** a user requests `GET /api/jobs?group=ETL&status=scheduled`
- **THEN** only jobs with group "ETL" AND status "scheduled" are returned

### Requirement: Bulk group assignment
The system SHALL provide a `PUT /api/jobs/bulk-group` endpoint that assigns a group to multiple jobs at once. The endpoint SHALL require write access (admin or operator role).

#### Scenario: Assign group to multiple jobs
- **WHEN** a user sends `PUT /api/jobs/bulk-group` with `{"job_ids": ["id1", "id2"], "group": "ETL"}`
- **THEN** both jobs' group is set to "ETL"

#### Scenario: Remove group from multiple jobs
- **WHEN** a user sends `PUT /api/jobs/bulk-group` with `{"job_ids": ["id1"], "group": null}`
- **THEN** the job's group is set to null

#### Scenario: Non-writer denied
- **WHEN** a viewer role user attempts `PUT /api/jobs/bulk-group`
- **THEN** the system returns 403 Forbidden

### Requirement: Group badge in jobs list UI
The jobs list page SHALL display a colored badge next to each job's name showing its group. The badge color SHALL be derived from the group name (consistent hash) so the same group always gets the same color. Ungrouped jobs SHALL show no badge.

#### Scenario: Job with group displayed
- **WHEN** the jobs list renders a job with group "ETL"
- **THEN** a colored pill badge with text "ETL" appears next to the job name

#### Scenario: Ungrouped job displayed
- **WHEN** the jobs list renders a job with no group
- **THEN** no group badge is shown

### Requirement: Group filter dropdown in jobs UI
The jobs page action bar SHALL include a group filter dropdown populated from `GET /api/jobs/groups`. The dropdown SHALL include "All Groups" (no filter) and "Ungrouped" options in addition to the group names.

#### Scenario: Dropdown shows all groups
- **WHEN** the jobs page loads and groups "ETL", "Monitoring" exist
- **THEN** the dropdown shows: "All Groups", "Ungrouped", "ETL", "Monitoring"

#### Scenario: Selecting a group filters the list
- **WHEN** the user selects "ETL" from the group dropdown
- **THEN** the jobs list re-fetches with `?group=ETL` and shows only ETL jobs

#### Scenario: Selecting "All Groups" clears the filter
- **WHEN** the user selects "All Groups"
- **THEN** the jobs list re-fetches without a group filter

### Requirement: Group field in job create/edit modal
The job creation and edit modals SHALL include a text input for the group name with autocomplete suggestions from existing groups.

#### Scenario: Create job with group via modal
- **WHEN** a user fills in "ETL" in the group field and creates the job
- **THEN** the job is created with group "ETL"

#### Scenario: Edit job group via modal
- **WHEN** a user opens the edit modal for a job and changes the group field
- **THEN** the job's group is updated on save
