## MODIFIED Requirements

### Requirement: Custom agents register task type definitions
Custom agents SHALL register with the controller without providing task type definitions. The `task_types` field in the registration payload SHALL be ignored. Task type definitions are managed exclusively through the UI.

#### Scenario: Agent registers without task types
- **WHEN** a custom agent sends `POST /api/agents/register` without a `task_types` field
- **THEN** the controller registers the agent and preserves any existing `task_types_json` from a previous UI configuration

#### Scenario: Agent registration ignores task_types field
- **WHEN** a custom agent sends `POST /api/agents/register` with a `task_types` field
- **THEN** the controller ignores the `task_types` value and preserves the existing UI-managed definitions

#### Scenario: Re-registration preserves UI-configured task types
- **WHEN** a custom agent re-registers (e.g., after restart)
- **THEN** the stored task type definitions configured via the UI are not overwritten

## ADDED Requirements

### Requirement: API endpoint updates agent task types
The system SHALL provide `PUT /api/agents/{id}/task-types` to update an agent's task type definitions from the UI.

#### Scenario: Updating task types via API
- **WHEN** a PUT request is sent to `/api/agents/{id}/task-types` with a `task_types` JSON array
- **THEN** the agent's `task_types_json` column is updated with the provided definitions

#### Scenario: Clearing all task types
- **WHEN** a PUT request is sent with an empty `task_types` array
- **THEN** the agent's task types are cleared
