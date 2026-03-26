## Context

The `TaskType` enum in `models.rs` has 6 variants (Shell, Sql, Ftp, Http, Script, Custom). Each has a corresponding form in the job modal. `run_task` in `executor/local.rs` handles each variant. The task is serialized as JSON in the `task_json` column.

Standard agents receive tasks via HTTP POST to `/execute` and run them through the same `run_task` function. Custom agents receive the raw task JSON in the queue and handle it in their own code.

## Goals / Non-Goals

**Goals:**
- New `FilePush` task type variant
- File upload in the job modal with client-side base64 encoding
- Agent writes the file to the specified destination path
- Works with both standard and custom agents
- 5MB file size limit

**Non-Goals:**
- File retrieval from agents (pull)
- Directory sync or multi-file push
- Streaming large files (chunked transfer)
- File versioning or history

## Decisions

### 1. FilePush task type with base64 content in task JSON

**Decision**: Add a `FilePush` variant to `TaskType`:

```rust
FilePush {
    filename: String,
    destination: String,
    content_base64: String,
    permissions: Option<String>,  // e.g., "644", "755"
    overwrite: bool,
}
```

The file content is base64-encoded and stored directly in `task_json`. This means the file is persisted in SQLite alongside the job — no separate file storage needed.

**Rationale**: Simplest approach. No new endpoints, no file store, no multipart upload. The 5MB limit keeps the encoded content (~6.7MB base64) within reasonable SQLite row sizes. The task snapshot captures the exact file that was deployed.

### 2. Client-side file reading and encoding

**Decision**: The job modal uses the browser's `FileReader` API to read the file and convert to base64. The encoded string is included in the task JSON submitted via the existing `POST /api/jobs` endpoint.

```javascript
const reader = new FileReader();
reader.onload = () => {
    const base64 = btoa(reader.result);
    // Include in task object
};
reader.readAsBinaryString(file);
```

**Rationale**: No server-side upload endpoint needed. The existing job creation API handles it. Client-side encoding is fast for files under 5MB.

### 3. Agent-side file write in run_task

**Decision**: Add a `FilePush` handler in `run_task` that:
1. Base64-decodes the content
2. Creates parent directories if needed (`std::fs::create_dir_all`)
3. Checks if file exists and `overwrite` is false → fail
4. Writes the file
5. Sets permissions if provided (Unix only, via `std::os::unix::fs::PermissionsExt`)
6. Returns stdout: `"File written to {destination} ({size} bytes)"`

**Rationale**: Standard `std::fs` operations. No external dependencies. Permission setting is platform-conditional.

### 4. File size validation

**Decision**: Validate file size both client-side (in the file picker) and server-side (in the job creation handler). Reject files over 5MB with a clear error message.

Server-side check: After deserializing the task JSON, if the variant is `FilePush`, check that `content_base64.len()` < 7_000_000 (5MB base64-encoded is ~6.67MB).

**Rationale**: Client-side validation prevents uploading large files. Server-side validation is a safety net against API calls bypassing the UI.

### 5. Display in job form and execution detail

**Decision**:
- Job form: Show filename, size, and a "Change file" button after upload. Don't show the raw base64.
- Execution detail: Show the filename and destination in the task snapshot, but truncate `content_base64` to avoid rendering megabytes of encoded data.
- Job detail: Show filename + destination + size, not the content.

**Rationale**: Raw base64 in the UI would be unusable. Show metadata only.

## Risks / Trade-offs

- **SQLite row size** → 5MB base64 (~6.7MB) per task is large for SQLite but within its capabilities. WAL mode handles it well. The task snapshot on each execution doubles the storage for file push jobs.
- **Task snapshot bloat** → Each execution snapshots the full task including the file content. For frequently-triggered file push jobs, this could grow the DB quickly. Mitigated by the data retention purge.
- **No streaming** → The entire file must fit in memory (both client and server). 5MB is fine for configs and scripts but not for large binaries. Acceptable limitation.
