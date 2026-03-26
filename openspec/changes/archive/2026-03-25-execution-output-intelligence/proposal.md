## Why

Execution output (stdout/stderr) is currently stored as raw text and only viewable in the execution detail modal. This data is valuable but unused — it contains metrics, status codes, error patterns, and state that could drive automation, monitoring, and change detection. Three features unlock this value: extracting structured data from output, triggering actions based on output patterns, and comparing output across runs to detect drift.

## What Changes

- **Output extraction rules**: Jobs can define extraction rules (regex with named groups, or JSON path expressions) that run against stdout after each execution. Extracted values are stored as key-value pairs on the execution record and displayed as structured fields in the execution detail. Configurable per-job in the edit modal.
- **Output pattern triggers**: Jobs can define output match patterns (substring or regex) with a severity level. When a pattern matches stdout or stderr, the system emits an event (e.g., `output.matched`) that can trigger event-driven jobs. This integrates with the existing event trigger system.
- **Output diff across runs**: The execution detail UI shows a "Compare" tab that lets users pick a previous execution and see a side-by-side or inline diff of the output. Useful for detecting configuration drift, regression, or unexpected changes in periodic job output.

## Capabilities

### New Capabilities
- `output-extraction`: Per-job extraction rules that parse structured values from execution output
- `output-triggers`: Pattern matching on output that emits events to trigger other jobs
- `output-diff`: Compare execution output across runs with visual diff

### Modified Capabilities

## Impact

- **Models**: New `output_rules` field on Job (extraction rules + pattern triggers)
- **Database**: Migration adds `output_rules_json` to jobs table, `extracted_json` to executions table
- **Executor**: After execution completes, run extraction rules and pattern matching against output
- **API**: Extracted values included in execution response; new endpoint for fetching previous execution output for diff
- **Frontend**: Extraction rules editor in job modal; extracted values display in execution detail; pattern trigger config; compare/diff tab in execution detail
- **Events**: New `output.matched` event kind for pattern triggers
