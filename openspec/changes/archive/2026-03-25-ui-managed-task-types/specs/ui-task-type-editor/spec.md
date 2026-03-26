## ADDED Requirements

### Requirement: Custom agent card opens inline config panel
Clicking a custom agent card SHALL toggle an expandable detail section below the card containing the task type editor. Standard agent cards SHALL not have this behavior.

#### Scenario: Clicking a custom agent card
- **WHEN** the user clicks on an agent card where `agent_type = "custom"`
- **THEN** an inline config panel expands below the card showing the task type editor

#### Scenario: Clicking the card again closes the panel
- **WHEN** the user clicks the same custom agent card while the panel is open
- **THEN** the panel collapses

#### Scenario: Standard agent card is not clickable
- **WHEN** the user clicks on an agent card where `agent_type = "standard"`
- **THEN** nothing happens (no config panel)

### Requirement: Task type editor has structured form controls
The config panel SHALL render each task type as a collapsible section with editable name, description, and a list of field definitions with controls for name, label, type, required, and placeholder.

#### Scenario: Viewing existing task types
- **WHEN** the config panel opens for an agent with task types defined
- **THEN** each task type is shown as a section with its name, description, and field list

#### Scenario: Adding a new task type
- **WHEN** the user clicks "Add Task Type"
- **THEN** a new empty task type section appears with inputs for name and description

#### Scenario: Adding a field to a task type
- **WHEN** the user clicks "Add Field" within a task type section
- **THEN** a new field row appears with inputs for name, label, type dropdown, required checkbox, and placeholder

#### Scenario: Removing a task type
- **WHEN** the user clicks the delete button on a task type section
- **THEN** the task type section is removed from the editor

#### Scenario: Removing a field
- **WHEN** the user clicks the remove button on a field row
- **THEN** the field row is removed from the task type

#### Scenario: Select field type shows options input
- **WHEN** the user selects "select" from the field type dropdown
- **THEN** an additional textarea appears for entering options as value:label pairs

### Requirement: Save button persists task type definitions
The config panel SHALL have a Save button that sends the current task type definitions to `PUT /api/agents/{id}/task-types`.

#### Scenario: Saving task types
- **WHEN** the user clicks Save
- **THEN** the editor collects all task type definitions into a JSON array and sends a PUT request to the API

#### Scenario: Save success feedback
- **WHEN** the API returns success
- **THEN** a success toast is shown and the agent data is refreshed

#### Scenario: Save with validation
- **WHEN** a task type has no name
- **THEN** the save is prevented with an error toast indicating which task type needs a name
