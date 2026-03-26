### Requirement: FilePush task type variant
The `TaskType` enum SHALL include a `FilePush` variant with fields: `filename` (original name), `destination` (path on agent), `content_base64` (base64-encoded file content), optional `permissions` (Unix mode string), and `overwrite` (boolean).

#### Scenario: Creating a file push job
- **WHEN** a job is created with task type `file_push`, filename `deploy.sh`, destination `/opt/scripts/deploy.sh`, and base64 content
- **THEN** the job stores a `FilePush` task type with all fields in `task_json`

#### Scenario: FilePush task dispatched to agent
- **WHEN** a file push job targets a standard agent
- **THEN** the agent receives the full task JSON including the base64 file content

### Requirement: Agent writes file to destination
When `run_task` processes a `FilePush` task, it SHALL decode the base64 content, create parent directories if needed, write the file to the destination path, and optionally set file permissions.

#### Scenario: File written successfully
- **WHEN** `run_task` processes a `FilePush` task with destination `/opt/configs/app.conf`
- **THEN** parent directories are created, the decoded content is written to the path, and stdout reports the filename, destination, and byte count

#### Scenario: File exists and overwrite is false
- **WHEN** the destination file already exists and `overwrite` is false
- **THEN** the execution fails with an error message indicating the file already exists

#### Scenario: File exists and overwrite is true
- **WHEN** the destination file already exists and `overwrite` is true
- **THEN** the file is overwritten with the new content

#### Scenario: Permissions set on Unix
- **WHEN** `permissions` is set to `"755"` on a Unix system
- **THEN** the file permissions are set to `rwxr-xr-x` after writing

#### Scenario: Invalid base64 content
- **WHEN** `content_base64` contains invalid base64
- **THEN** the execution fails with a decode error message

### Requirement: File size limited to 5MB
The system SHALL reject file push tasks where the original file content exceeds 5MB.

#### Scenario: Client-side validation
- **WHEN** the user selects a file larger than 5MB in the job form
- **THEN** an error is shown and the file is not uploaded

#### Scenario: Server-side validation
- **WHEN** a job is created via API with `content_base64` exceeding ~6.7MB (5MB encoded)
- **THEN** the job creation returns an error

### Requirement: File upload UI in job modal
The job modal SHALL show a file picker when the "File Push" task type is selected, with inputs for destination path, permissions, and overwrite toggle.

#### Scenario: Selecting File Push task type
- **WHEN** the user selects "File Push" from the task type radio buttons
- **THEN** the form shows a file input, destination path input, optional permissions input, and overwrite checkbox

#### Scenario: File selected shows metadata
- **WHEN** the user selects a file via the file picker
- **THEN** the form displays the filename and size, and reads the content as base64

#### Scenario: Submitting the form
- **WHEN** the user fills in the file push form and clicks Save
- **THEN** a job is created with `TaskType::FilePush` containing the base64-encoded file content

### Requirement: Task display hides base64 content
When displaying a `FilePush` task in the UI (job detail, execution detail, task snapshot), the system SHALL show the filename, destination, size, and permissions but NOT the raw base64 content.

#### Scenario: Job detail shows file metadata
- **WHEN** the user views a job with a FilePush task
- **THEN** the display shows filename, destination path, file size, and permissions — not the encoded content

#### Scenario: Execution task snapshot
- **WHEN** the user views an execution's task snapshot for a FilePush job
- **THEN** the `content_base64` field is replaced with a size indicator like `"[5.2 KB]"`
