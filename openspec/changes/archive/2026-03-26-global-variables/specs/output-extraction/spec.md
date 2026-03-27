## MODIFIED Requirements

### Requirement: Jobs can define extraction rules
Jobs SHALL have an optional `output_rules` field containing an `extractions` array. Each extraction rule has a `name` (key for the extracted value), a `pattern` (regex or JSON path expression), a `type` ("regex" or "jsonpath"), and an optional `write_to_variable` (string naming a global variable to update with the extracted value).

#### Scenario: Job with regex extraction rule
- **WHEN** a job is created with an extraction rule `{ name: "duration", pattern: "took (\\d+)ms", type: "regex" }`
- **THEN** the rule is stored in `output_rules_json` on the job record

#### Scenario: Job with jsonpath extraction rule
- **WHEN** a job is created with an extraction rule `{ name: "count", pattern: "$.results.count", type: "jsonpath" }`
- **THEN** the rule is stored in `output_rules_json` on the job record

#### Scenario: Maximum 10 extraction rules per job
- **WHEN** a job has more than 10 extraction rules configured
- **THEN** the system rejects the save with an error

#### Scenario: Job with extraction rule that writes to a variable
- **WHEN** a job is created with an extraction rule `{ name: "count", pattern: "$.results.count", type: "jsonpath", write_to_variable: "RESULT_COUNT" }`
- **THEN** the rule is stored in `output_rules_json` including the `write_to_variable` field
