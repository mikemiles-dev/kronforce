## Why

There's no built-in way to push a file to an agent. Users deploying configs, scripts, or small binaries to remote machines need to either use FTP (requires a server on the agent) or wrap file content in a shell command (awkward, size-limited). A dedicated "file push" task type lets users upload a file in the job creation form and specify where it should land on the agent — simple, direct, and integrated with the existing job targeting system.

## What Changes

- **New `FilePush` task type**: A built-in task type with fields: uploaded file content (stored as base64 in the task JSON), destination path on the agent, file permissions (optional), and whether to overwrite existing files.
- **File upload in job modal**: When the user selects the "File Push" task type, the form shows a file picker, destination path input, and optional permissions field. The file content is read client-side and base64-encoded into the task JSON.
- **Standard agent handling**: The standard agent's `run_task` function writes the decoded file content to the destination path on the agent's filesystem.
- **Custom agent handling**: Custom agents receive the base64 file content in `task.data` and handle it however they want.
- **File size limit**: Cap at 5MB to keep the task JSON manageable in SQLite. Show a clear error in the UI if the file exceeds the limit.

## Capabilities

### New Capabilities
- `file-push-task`: New FilePush task type with file upload UI, base64 storage, and agent-side file write

### Modified Capabilities

## Impact

- **Models**: New `FilePush` variant in `TaskType` enum
- **Executor**: `run_task` handles `FilePush` by writing decoded content to disk
- **Frontend**: File picker in job modal for the File Push task type, base64 encoding client-side
- **No database changes**: File content stored as base64 in existing `task_json` column
- **No new endpoints**: Uses existing job creation API
