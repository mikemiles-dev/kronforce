### Requirement: Jobs can define output trigger patterns
Jobs SHALL have an optional `triggers` array within `output_rules`. Each trigger has a `pattern` (substring or regex to match against stdout and stderr) and a `severity` level (error, warning, info, success).

#### Scenario: Job with output trigger
- **WHEN** a job is created with a trigger `{ pattern: "ERROR|FATAL", severity: "error" }`
- **THEN** the trigger is stored in `output_rules_json` on the job record

### Requirement: Matched patterns emit output.matched events
When an execution completes and a trigger pattern matches stdout or stderr, the system SHALL emit an `output.matched` event with the configured severity.

#### Scenario: Pattern matches stdout
- **WHEN** an execution completes with stdout containing "FATAL: disk full" and the job has trigger `{ pattern: "FATAL", severity: "error" }`
- **THEN** an event is emitted with kind `output.matched`, severity `error`, and message indicating which pattern matched in which job

#### Scenario: Pattern matches stderr
- **WHEN** an execution completes with stderr containing "WARNING: low memory" and the job has trigger `{ pattern: "WARNING", severity: "warning" }`
- **THEN** an event is emitted with kind `output.matched`, severity `warning`

#### Scenario: No pattern matches
- **WHEN** an execution completes and no trigger patterns match stdout or stderr
- **THEN** no `output.matched` event is emitted

#### Scenario: Multiple patterns match
- **WHEN** an execution output matches multiple trigger patterns
- **THEN** one event is emitted per matched pattern

### Requirement: Output events trigger event-driven jobs
The `output.matched` event SHALL be compatible with the existing event trigger system so that jobs with `schedule.type: "event"` and `kind_pattern: "output.matched"` or `kind_pattern: "output.*"` are triggered.

#### Scenario: Event-driven job reacts to output match
- **WHEN** an `output.matched` event is emitted with severity `error` and another job has `schedule: { type: "event", value: { kind_pattern: "output.matched", severity: "error" } }`
- **THEN** that job is triggered

### Requirement: Trigger pattern editor in job modal
The job create/edit modal SHALL include an "Output Triggers" subsection in the Advanced section with controls to add, edit, and remove trigger patterns.

#### Scenario: Adding a trigger pattern
- **WHEN** the user clicks "Add Trigger" in the job modal
- **THEN** a new row appears with inputs for pattern and severity dropdown

#### Scenario: Removing a trigger pattern
- **WHEN** the user clicks the remove button on a trigger row
- **THEN** the row is removed from the editor
