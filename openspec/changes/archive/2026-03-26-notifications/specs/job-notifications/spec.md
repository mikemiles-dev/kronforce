## ADDED Requirements

### Requirement: Jobs have per-job notification configuration
Each job SHALL have an optional `notifications` field with toggles for when to notify and optional recipient overrides.

#### Scenario: Job with failure notification enabled
- **WHEN** a job is created with `notifications: { on_failure: true }`
- **THEN** the config is stored in `notifications_json` on the job record

#### Scenario: Job with no notification config
- **WHEN** a job has no `notifications` field or it is null
- **THEN** no notifications are sent for that job's executions

#### Scenario: Job with recipient overrides
- **WHEN** a job has `notifications: { on_failure: true, recipients: { emails: ["specific@example.com"] } }`
- **THEN** failure notifications are sent to `specific@example.com` instead of the global recipients

### Requirement: Notifications sent on execution failure
When an execution finishes with status `failed` or `timed_out` and the job has `on_failure: true`, the system SHALL send a notification.

#### Scenario: Job fails and notification is sent
- **WHEN** an execution completes with status `failed` and the job has `on_failure: true`
- **THEN** a notification is sent with subject `[Kronforce] Job 'name' failed` and body including job name, status, timestamp, and stderr excerpt

#### Scenario: Job fails but notifications disabled
- **WHEN** an execution completes with status `failed` and the job has no notification config
- **THEN** no notification is sent

### Requirement: Notifications sent on execution success
When an execution finishes with status `succeeded` and the job has `on_success: true`, the system SHALL send a notification.

#### Scenario: Job succeeds and notification is sent
- **WHEN** an execution completes with status `succeeded` and the job has `on_success: true`
- **THEN** a notification is sent with subject `[Kronforce] Job 'name' succeeded`

### Requirement: Notifications sent on assertion failure
When an output assertion fails (flipping status from succeeded to failed) and the job has `on_assertion_failure: true`, the system SHALL send a notification.

#### Scenario: Assertion fails and notification is sent
- **WHEN** an execution's output assertion fails and the job has `on_assertion_failure: true`
- **THEN** a notification is sent with subject `[Kronforce] Job 'name' assertion failed` including the assertion failure messages

### Requirement: Notification toggles in job modal
The job create/edit modal SHALL include notification checkboxes in the Advanced section.

#### Scenario: Enabling failure notifications
- **WHEN** the user checks "Notify on failure" in the job modal
- **THEN** the job is saved with `notifications.on_failure: true`

#### Scenario: Adding recipient overrides
- **WHEN** the user enters email addresses in the notification recipients field
- **THEN** those addresses override the global recipients for this job's notifications
