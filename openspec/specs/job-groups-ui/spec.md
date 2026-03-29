## ADDED Requirements

### Requirement: Groups page
The system SHALL provide a dedicated Groups page that displays all job groups as visual cards in a responsive grid. Each card SHALL show the group name, the number of jobs in that group, and a colored dot matching the group's badge color. The page SHALL include an "Ungrouped" card showing the count of jobs without a group.

#### Scenario: Groups page with multiple groups
- **WHEN** the user navigates to the Groups page and groups "ETL", "Monitoring", "Deploys" exist
- **THEN** three group cards are displayed plus an "Ungrouped" card, each showing the group name and job count

#### Scenario: Groups page with no groups
- **WHEN** all jobs are ungrouped
- **THEN** only the "Ungrouped" card is shown with the total job count

#### Scenario: Click group card navigates to filtered jobs
- **WHEN** the user clicks the "ETL" group card
- **THEN** the user is navigated to the Jobs page with the group filter set to "ETL"

#### Scenario: Click ungrouped card
- **WHEN** the user clicks the "Ungrouped" card
- **THEN** the user is navigated to the Jobs page with the group filter set to "ungrouped"

### Requirement: Group rename
Each group card SHALL have a rename button. Renaming a group SHALL update the group name on all jobs in that group via a single API call.

#### Scenario: Rename a group
- **WHEN** the user clicks rename on the "ETL" card and enters "Data Pipeline"
- **THEN** the system sends `PUT /api/jobs/rename-group` with `{"old_name": "ETL", "new_name": "Data Pipeline"}` and all jobs formerly in "ETL" now have group "Data Pipeline"

#### Scenario: Rename with invalid name
- **WHEN** the user enters a group name with invalid characters
- **THEN** the system returns 400 Bad Request and the rename is not applied

#### Scenario: Rename to existing group name
- **WHEN** the user renames "ETL" to "Monitoring" and "Monitoring" already exists
- **THEN** the jobs from "ETL" merge into the existing "Monitoring" group

### Requirement: Group delete
Each group card SHALL have a delete button. Deleting a group SHALL remove the group label from all jobs in that group, making them ungrouped.

#### Scenario: Delete a group
- **WHEN** the user clicks delete on the "ETL" card and confirms
- **THEN** all jobs in "ETL" have their group set to null and the "ETL" card disappears

#### Scenario: Delete requires confirmation
- **WHEN** the user clicks delete on a group card
- **THEN** a confirmation prompt is shown before proceeding

### Requirement: Rename group API endpoint
The system SHALL provide a `PUT /api/jobs/rename-group` endpoint that renames all jobs from one group to another. The endpoint SHALL require write access (admin or operator role).

#### Scenario: Successful rename
- **WHEN** an authenticated user with write access sends `PUT /api/jobs/rename-group` with `{"old_name": "ETL", "new_name": "Data Pipeline"}`
- **THEN** the system updates all jobs with `group_name = 'ETL'` to `group_name = 'Data Pipeline'` and returns `{"updated": N}`

#### Scenario: New name validation
- **WHEN** the new name exceeds 50 characters or contains invalid characters
- **THEN** the system returns 400 Bad Request

#### Scenario: Non-writer denied
- **WHEN** a viewer role user attempts the rename
- **THEN** the system returns 403 Forbidden

### Requirement: Sidebar groups entry
The sidebar SHALL include a "Groups" navigation entry positioned immediately after the "Jobs" entry. The entry SHALL be visually indented or styled to suggest it is a sub-section of Jobs.

#### Scenario: Groups entry visible in sidebar
- **WHEN** the page loads
- **THEN** a "Groups" button appears in the sidebar below "Jobs"

#### Scenario: Clicking Groups navigates to groups page
- **WHEN** the user clicks "Groups" in the sidebar
- **THEN** the Groups page is shown and the "Groups" sidebar entry is highlighted as active

### Requirement: Group field in main modal tab
The group input field SHALL be displayed in the first (main) tab of the job create/edit modal, positioned directly below the Name field. It SHALL no longer appear in the Advanced tab.

#### Scenario: Group field visible on modal open
- **WHEN** the user opens the create job modal
- **THEN** the group input with autocomplete is visible in the main tab without switching tabs

#### Scenario: Group field populated on edit
- **WHEN** the user opens the edit modal for a job in group "ETL"
- **THEN** the group input shows "ETL" in the main tab

### Requirement: Dashboard group summary
The dashboard Overview tab SHALL include a "Top Groups" section showing up to 5 groups with the most jobs, displayed as a compact list with group name, job count, and colored dot. Each entry SHALL link to the filtered jobs page for that group.

#### Scenario: Dashboard shows top groups
- **WHEN** the dashboard loads and groups exist
- **THEN** a "Top Groups" section shows the top 5 groups by job count

#### Scenario: Dashboard with no groups
- **WHEN** no jobs have groups assigned
- **THEN** the "Top Groups" section shows "No groups configured"

#### Scenario: Clicking a group entry navigates to jobs
- **WHEN** the user clicks "ETL (12 jobs)" in the top groups section
- **THEN** the user is navigated to the Jobs page filtered by group "ETL"
