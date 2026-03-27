## ADDED Requirements

### Requirement: Extraction rules can write to global variables
Each extraction rule SHALL support an optional `write_to_variable` field (string). When set, after extraction produces a value, the system SHALL upsert that value into the global variables table using the `write_to_variable` name as the key. The extracted value is still stored on the execution record as before.

#### Scenario: Extraction writes to a global variable
- **WHEN** a job has extraction rule `{ name: "latest_count", pattern: "count=(\\d+)", type: "regex", write_to_variable: "ITEM_COUNT" }` and the execution stdout contains `count=42`
- **THEN** the execution's `extracted_json` contains `{ "latest_count": "42" }` AND the global variable `ITEM_COUNT` is set to `42`

#### Scenario: Extraction with no write-back
- **WHEN** a job has extraction rule `{ name: "duration", pattern: "took (\\d+)ms", type: "regex" }` without `write_to_variable`
- **THEN** the extracted value is stored only on the execution record (existing behavior unchanged)

#### Scenario: Write-back creates variable if it does not exist
- **WHEN** `write_to_variable` is set to `NEW_VAR` and no variable named `NEW_VAR` exists
- **THEN** the variable is created with the extracted value

#### Scenario: Write-back updates existing variable
- **WHEN** `write_to_variable` is set to `EXISTING_VAR` and the variable already exists with a different value
- **THEN** the variable's value and `updated_at` are updated to the new extracted value

#### Scenario: Pattern does not match with write-back configured
- **WHEN** an extraction rule with `write_to_variable` set does not match the output
- **THEN** no variable write occurs and the existing variable value (if any) is unchanged

### Requirement: Write-to-variable field in extraction rule editor
The extraction rule editor in the job modal SHALL include an optional "Write to variable" text input on each extraction row. The input SHALL validate that the variable name matches the allowed pattern (`[A-Za-z0-9_]`).

#### Scenario: Adding write-back to an extraction rule
- **WHEN** the user adds an extraction rule and enters `DEPLOY_VERSION` in the "Write to variable" field
- **THEN** the extraction rule is saved with `write_to_variable: "DEPLOY_VERSION"`

#### Scenario: Leaving write-back empty
- **WHEN** the user leaves the "Write to variable" field empty
- **THEN** the extraction rule is saved without `write_to_variable` (null/omitted)
