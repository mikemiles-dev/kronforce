## Context

Execution output (stdout/stderr) is stored as text in the `executions` table (up to 256KB per stream). After an execution completes, the result is written via `update_execution()` in `db.rs`. The callback handler in `api.rs` processes results from agents. For local executions, `executor.rs` writes results directly.

Jobs are stored with a `task_json` column and have various config fields. The event system emits events via `db.log_event()` which can trigger event-driven jobs matching `kind_pattern`.

## Goals / Non-Goals

**Goals:**
- Per-job extraction rules that parse values from stdout after each run
- Extracted values stored on the execution and displayed in the UI
- Per-job output pattern matchers that emit events when patterns are found
- Output diff view comparing any two executions of the same job
- All configuration done through the job edit modal

**Non-Goals:**
- Real-time streaming output analysis (only post-execution)
- Full-text search across all execution outputs
- Machine learning or AI-based output analysis
- Extraction from stderr (stdout only, to keep it simple)

## Decisions

### 1. Output rules stored as JSON on the Job

**Decision**: Add `output_rules_json TEXT` column to the `jobs` table via migration. The JSON contains two arrays:

```json
{
  "extractions": [
    { "name": "duration_ms", "pattern": "completed in (\\d+)ms", "type": "regex" },
    { "name": "record_count", "pattern": "$.results.count", "type": "jsonpath" }
  ],
  "triggers": [
    { "pattern": "ERROR|FATAL", "severity": "error" },
    { "pattern": "WARNING", "severity": "warning" }
  ]
}
```

**Rationale**: Keeps rules with the job that produces the output. No separate table needed. The two concerns (extraction and triggers) share the same lifecycle.

### 2. Extracted values stored as JSON on the Execution

**Decision**: Add `extracted_json TEXT` column to the `executions` table via migration. After execution completes, run extraction rules against stdout and store results as:

```json
{ "duration_ms": "245", "record_count": "1523" }
```

Values are always stored as strings. The UI can format them based on context.

**Rationale**: Storing on the execution keeps extracted data with its source. No separate table, no joins needed.

### 3. Extraction runs in the callback/completion handler

**Decision**: After an execution result is written (both local and agent callback), if the job has extraction rules, run them against stdout:
- **Regex**: Use Rust's `regex` crate. Named capture groups or group 1 becomes the value.
- **JSON path**: Parse stdout as JSON, evaluate the path expression. Use a simple dot-notation evaluator (no external crate) — e.g., `$.results.count` traverses the JSON object.

Run extraction and trigger matching in the same pass. Store extracted values, emit trigger events.

**Rationale**: Post-execution processing is simpler than inline. The regex crate is already available in the project.

### 4. Pattern triggers emit `output.matched` events

**Decision**: When a trigger pattern matches stdout or stderr, emit an event with kind `output.matched`, severity from the trigger config, and a message like `"Output pattern matched: 'ERROR' in job 'etl-pipeline'"`. This integrates with the existing event trigger system — other jobs can use `schedule.type: "event"` with `kind_pattern: "output.matched"` to react.

**Rationale**: Reuses the existing event infrastructure. No new dispatch mechanism needed. Users configure reactions the same way they configure any event-triggered job.

### 5. Output diff as a client-side comparison

**Decision**: The execution detail modal gets a "Compare" button that opens a dropdown of recent executions for the same job. Selecting one fetches that execution's stdout and renders a side-by-side diff. The diff is computed client-side using a simple line-by-line comparison algorithm (longest common subsequence).

No server-side diff computation or storage — the client fetches both outputs and diffs them.

**Rationale**: Keeps the server simple. Output is already available via `GET /api/executions/{id}`. A lightweight JS diff algorithm (30-50 lines) handles the comparison. Side-by-side rendering with colored additions/deletions is straightforward HTML.

### 6. Extraction rules editor in the job modal Advanced section

**Decision**: Add an "Output Rules" subsection in the Advanced section of the job create/edit modal. It has two parts:
- **Extractions**: List of rows with name + pattern + type (regex/jsonpath). Add/remove buttons.
- **Triggers**: List of rows with pattern + severity dropdown. Add/remove buttons.

Similar UX to the custom agent task type editor — structured form controls, not raw JSON.

**Rationale**: Matches the existing UI patterns for structured config (dependencies, task type editor). Users don't need to write JSON.

## Risks / Trade-offs

- **Regex performance** → Extraction runs after each execution. Malicious or complex regexes could be slow. Mitigated by using Rust's regex crate which guarantees linear time. Limit to 10 extraction rules per job.
- **JSON path is simplified** → Only dot-notation traversal (no array indexing, filters, or wildcards). Covers 90% of use cases. Users needing complex extraction can use regex instead.
- **Client-side diff for large output** → 256KB x 2 could be slow to diff in the browser. Mitigated by truncating the diff view to the first 50KB of each output with a "show full" option.
- **No stderr extraction** → Keeps the feature focused. Can be added later if needed.
