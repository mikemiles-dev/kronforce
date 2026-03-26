## ADDED Requirements

### Requirement: Custom agents register task type definitions
Custom agents SHALL be able to include a `task_types` array in their registration payload. Each entry SHALL have a `name`, optional `description`, and a `fields` array describing the form inputs for that task type.

#### Scenario: Agent registers with task type definitions
- **WHEN** a custom agent sends `POST /api/agents/register` with a `task_types` array containing one or more definitions
- **THEN** the controller stores the definitions as JSON in the `task_types_json` column of the agents table

#### Scenario: Standard agent ignores task_types field
- **WHEN** a standard agent sends `POST /api/agents/register` with or without a `task_types` field
- **THEN** the controller stores an empty array for `task_types_json`

#### Scenario: Re-registration updates task type definitions
- **WHEN** a custom agent re-registers with an updated `task_types` array
- **THEN** the stored definitions are replaced with the new ones

### Requirement: Task type definitions have field schemas
Each task type definition SHALL contain a `fields` array where each field has `name`, `label`, `field_type` (text/textarea/number/select/password), optional `required` flag, optional `placeholder`, and optional `options` array (for select fields).

#### Scenario: Text field definition
- **WHEN** a task type definition includes a field with `field_type: "text"`
- **THEN** the UI renders a text input with the specified label and placeholder

#### Scenario: Select field definition with options
- **WHEN** a task type definition includes a field with `field_type: "select"` and an `options` array
- **THEN** the UI renders a select dropdown with the specified value/label pairs

#### Scenario: Required field definition
- **WHEN** a task type definition includes a field with `required: true`
- **THEN** the UI marks the field as required and prevents form submission if empty

### Requirement: Database stores task type definitions
The agents table SHALL have a `task_types_json` column added via migration to store the serialized task type definitions.

#### Scenario: Migration adds column
- **WHEN** the controller starts with a database from a previous version
- **THEN** a migration adds the nullable `task_types_json` column to the agents table

#### Scenario: Agent API response includes task types
- **WHEN** `GET /api/agents/{id}` or `GET /api/agents` is called
- **THEN** the response includes a `task_types` array (deserialized from `task_types_json`, defaulting to empty array if null)

### Requirement: Custom TaskType variant stores arbitrary task data
The `TaskType` enum SHALL include a `Custom` variant with `agent_task_type` (string name) and `data` (arbitrary JSON object) fields.

#### Scenario: Creating a job with a custom task type
- **WHEN** a job is created with task type `custom`, `agent_task_type: "python"`, and `data: {"script": "print(1)"}`
- **THEN** the job stores a `Custom` task type with the provided type name and data

#### Scenario: Custom task dispatched to agent
- **WHEN** a job with a `Custom` task type is dispatched to a custom agent
- **THEN** the agent receives the full task JSON including `type: "custom"`, `agent_task_type`, and `data`

#### Scenario: Custom task cannot run locally
- **WHEN** a job with a `Custom` task type has target `Local` or no target
- **THEN** execution fails with an error indicating custom tasks require a custom agent

### Requirement: UI dynamically renders custom task forms
When creating a job in Custom Agent mode, the UI SHALL fetch the selected agent's task type definitions and render form fields dynamically based on the field schemas.

#### Scenario: Selecting a custom agent loads its task types
- **WHEN** the user selects a custom agent in the agent dropdown
- **THEN** the task type radio buttons update to show the agent's registered task types instead of the built-in types

#### Scenario: Selecting a custom task type renders dynamic fields
- **WHEN** the user selects one of the custom agent's task types
- **THEN** the form fields section renders inputs matching the field definitions (type, label, placeholder, required)

#### Scenario: Submitting a custom task form
- **WHEN** the user fills in the custom task form and clicks Save
- **THEN** a job is created with `TaskType::Custom { agent_task_type, data }` where `data` contains the field values as a JSON object
