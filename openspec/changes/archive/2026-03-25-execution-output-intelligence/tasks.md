## 1. Database Migrations

- [x] 1.1 Add migration to add `output_rules_json TEXT` column to the `jobs` table
- [x] 1.2 Add migration to add `extracted_json TEXT` column to the `executions` table

## 2. Backend: Models and DB

- [x] 2.1 Add `OutputRules`, `ExtractionRule`, and `OutputTrigger` structs to `src/models.rs`
- [x] 2.2 Add `output_rules` field to the `Job` struct (deserialized from `output_rules_json`)
- [x] 2.3 Add `extracted` field to `ExecutionRecord` (deserialized from `extracted_json`)
- [x] 2.4 Update `row_to_job()` to read `output_rules_json` column
- [x] 2.5 Update `row_to_execution()` to read `extracted_json` column
- [x] 2.6 Update job INSERT/UPDATE queries to include `output_rules_json`
- [x] 2.7 Add `update_execution_extracted(id, extracted_json)` method to `db.rs`

## 3. Backend: Extraction and Trigger Engine

- [x] 3.1 Add `src/output_rules.rs` module with `run_extractions(stdout, rules) -> HashMap<String, String>` function
- [x] 3.2 Implement regex extraction — match pattern against stdout, capture group 1 or named group as value
- [x] 3.3 Implement simplified JSON path extraction — parse stdout as JSON, traverse dot-notation path
- [x] 3.4 Add `run_triggers(stdout, stderr, triggers) -> Vec<(pattern, severity)>` function that returns matched patterns
- [x] 3.5 Register the module in `src/lib.rs`

## 4. Backend: Post-Execution Processing

- [x] 4.1 After local execution completes in `executor.rs`, look up the job's output_rules and run extraction + triggers
- [x] 4.2 After agent callback in `api.rs` (`execution_result_callback`), look up the job's output_rules and run extraction + triggers
- [x] 4.3 Store extracted values via `update_execution_extracted()`
- [x] 4.4 Emit `output.matched` events for each matched trigger pattern via `db.log_event()`

## 5. Frontend: Extraction Rules Editor

- [x] 5.1 Add "Output Rules" subsection in the job modal Advanced section with "Extractions" and "Triggers" headers
- [x] 5.2 Render extraction rows: name input, pattern input, type dropdown (regex/jsonpath), remove button
- [x] 5.3 Add "Add Extraction" button that appends a new empty row
- [x] 5.4 Render trigger rows: pattern input, severity dropdown (error/warning/info/success), remove button
- [x] 5.5 Add "Add Trigger" button that appends a new empty row
- [x] 5.6 Update `buildTaskFromForm()` / `submitJobForm()` to collect output_rules from the editor and include in the job body
- [x] 5.7 Update `openEditModal()` to populate the extraction and trigger editors from existing job data

## 6. Frontend: Extracted Values Display

- [x] 6.1 In the execution detail modal (`showExecDetail`), render an "Extracted Values" section when `extracted` is present
- [x] 6.2 Display each key-value pair using `infoField()` with a distinct CSS class

## 7. Frontend: Output Diff

- [x] 7.1 Add a "Compare" button in the execution detail modal header
- [x] 7.2 Add `loadCompareExecutions(jobId, currentExecId)` — fetch recent executions for the job and render a dropdown
- [x] 7.3 Add `computeDiff(linesA, linesB)` — client-side line-by-line LCS diff algorithm returning add/remove/unchanged markers
- [x] 7.4 Add `renderDiffView(oldOutput, newOutput)` — render side-by-side diff with green (added) / red (removed) highlighting
- [x] 7.5 Add CSS for diff view (`.diff-add`, `.diff-remove`, `.diff-same`, two-column layout)
- [x] 7.6 Truncate diff inputs to 50KB with a "truncated for diff" indicator
