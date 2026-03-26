## ADDED Requirements

### Requirement: Execution detail has a Compare button
The execution detail modal SHALL include a "Compare" button that lets the user select a previous execution of the same job and view a diff of the output.

#### Scenario: Clicking Compare
- **WHEN** the user clicks "Compare" in the execution detail modal
- **THEN** a dropdown appears listing recent executions of the same job (excluding the current one), showing execution ID prefix, status, and date

#### Scenario: No other executions to compare
- **WHEN** the job has only one execution
- **THEN** the Compare button is disabled or shows "No previous runs"

### Requirement: Side-by-side diff view
When two executions are selected for comparison, the UI SHALL display a side-by-side diff of their stdout, highlighting added lines (green), removed lines (red), and unchanged lines.

#### Scenario: Viewing a diff
- **WHEN** the user selects a previous execution for comparison
- **THEN** the modal shows a two-column view with the previous output on the left and the current output on the right, with changed lines highlighted

#### Scenario: Identical output
- **WHEN** the two executions have identical stdout
- **THEN** the diff view shows all lines as unchanged with a "No differences" indicator

#### Scenario: Large output truncation
- **WHEN** either execution's stdout exceeds 50KB
- **THEN** the diff view truncates to the first 50KB with a "truncated for diff" indicator and an option to show full output

### Requirement: Diff computed client-side
The diff SHALL be computed in the browser using a line-by-line comparison algorithm. No server-side diff computation or storage is needed.

#### Scenario: Client fetches both outputs
- **WHEN** the user selects an execution to compare against
- **THEN** the browser fetches the comparison execution via `GET /api/executions/{id}` and computes the diff locally
