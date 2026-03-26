## 1. Backend: TaskType Variant

- [x] 1.1 Add `FilePush { filename, destination, content_base64, permissions, overwrite }` variant to `TaskType` enum in `src/models.rs`
- [x] 1.2 Add server-side validation in job create handler (`api/jobs.rs`): reject if `content_base64.len() > 7_000_000`

## 2. Backend: run_task Handler

- [x] 2.1 Add `FilePush` match arm in `run_task()` in `executor/local.rs`
- [x] 2.2 Decode base64 content using `base64` engine (already available via data-encoding or add the `base64` crate)
- [x] 2.3 Create parent directories with `std::fs::create_dir_all`
- [x] 2.4 Check if file exists and `overwrite` is false — return failed status
- [x] 2.5 Write decoded content to destination path
- [x] 2.6 Set file permissions if provided (Unix only, `#[cfg(unix)]` with `PermissionsExt`)
- [x] 2.7 Return success with stdout: `"File written to {destination} ({size} bytes)"`

## 3. Frontend: File Push Form Fields

- [x] 3.1 Add "File Push" option to the task type radio buttons in the job modal
- [x] 3.2 Add `task-filepush-fields` div with: file input, destination path input, permissions input, overwrite checkbox
- [x] 3.3 Add `updateTaskFields()` handler to show/hide the file push fields
- [x] 3.4 Add file input `onchange` handler that reads the file via `FileReader`, base64-encodes it, and stores in a variable
- [x] 3.5 Show filename and size after file selection, validate 5MB limit client-side
- [x] 3.6 Update `buildTaskFromForm()` to build `FilePush` task object with base64 content
- [x] 3.7 Update `populateTaskForm()` to handle editing existing FilePush jobs (show filename/size, allow re-upload)

## 4. Frontend: Display

- [x] 4.1 Update `fmtTaskBadge()` to show a badge for `file_push` type
- [x] 4.2 Update `fmtTaskDetail()` to show filename, destination, size for FilePush tasks (not base64 content)
- [x] 4.3 In execution detail task snapshot rendering, replace `content_base64` with a size indicator

## 5. Python Example

- [x] 5.1 Update `examples/custom_agent.py` `execute_task()` to handle `file_push` task type (decode base64, write to destination)
