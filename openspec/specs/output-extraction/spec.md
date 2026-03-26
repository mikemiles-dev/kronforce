### Requirement: Jobs can define extraction rules
Jobs SHALL have an optional `output_rules` field containing an `extractions` array. Each extraction rule has a `name` (key for the extracted value), a `pattern` (regex or JSON path expression), and a `type` ("regex" or "jsonpath").

#### Scenario: Job with regex extraction rule
- **WHEN** a job is created with an extraction rule `{ name: "duration", pattern: "took (\\d+)ms", type: "regex" }`
- **THEN** the rule is stored in `output_rules_json` on the job record

#### Scenario: Job with jsonpath extraction rule
- **WHEN** a job is created with an extraction rule `{ name: "count", pattern: "$.results.count", type: "jsonpath" }`
- **THEN** the rule is stored in `output_rules_json` on the job record

#### Scenario: Maximum 10 extraction rules per job
- **WHEN** a job has more than 10 extraction rules configured
- **THEN** the system rejects the save with an error

### Requirement: Extraction runs after execution completes
After an execution finishes (locally or via agent callback), the system SHALL run the job's extraction rules against stdout and store extracted values as JSON on the execution record.

#### Scenario: Regex extraction captures a value
- **WHEN** an execution completes with stdout "Processing took 245ms" and the job has rule `{ name: "duration", pattern: "took (\\d+)ms", type: "regex" }`
- **THEN** the execution's `extracted_json` contains `{ "duration": "245" }`

#### Scenario: JSON path extraction from JSON output
- **WHEN** an execution completes with stdout `{"results": {"count": 42}}` and the job has rule `{ name: "count", pattern: "$.results.count", type: "jsonpath" }`
- **THEN** the execution's `extracted_json` contains `{ "count": "42" }`

#### Scenario: Pattern does not match
- **WHEN** an extraction rule pattern does not match the output
- **THEN** that key is omitted from `extracted_json` (no error)

#### Scenario: Job has no extraction rules
- **WHEN** a job has no output_rules or empty extractions array
- **THEN** no extraction is performed and `extracted_json` remains null

### Requirement: Extracted values displayed in execution detail
The execution detail modal SHALL display extracted values as labeled key-value fields when `extracted_json` is present.

#### Scenario: Viewing execution with extracted values
- **WHEN** the user opens an execution detail that has `extracted_json` with values
- **THEN** the values are displayed as a section titled "Extracted Values" with each key-value pair shown

#### Scenario: Execution with no extracted values
- **WHEN** the user opens an execution detail with null `extracted_json`
- **THEN** no "Extracted Values" section is shown

### Requirement: Extraction rules editor in job modal
The job create/edit modal SHALL include an "Output Extractions" subsection in the Advanced section with controls to add, edit, and remove extraction rules.

#### Scenario: Adding an extraction rule
- **WHEN** the user clicks "Add Extraction" in the job modal
- **THEN** a new row appears with inputs for name, pattern, and type dropdown (regex/jsonpath)

#### Scenario: Removing an extraction rule
- **WHEN** the user clicks the remove button on an extraction row
- **THEN** the row is removed from the editor
