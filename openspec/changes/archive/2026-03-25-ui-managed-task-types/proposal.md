## Why

Task type definitions for custom agents are currently sent in the agent's registration payload, coupling the workflow definition to the agent code. This is backwards — the admin should define what a custom agent can do (task types, fields, forms) from the UI, and the agent code just handles whatever data it receives. This separation means you can reconfigure task types without restarting the agent, and non-developers can set up workflows through the dashboard.

## What Changes

- **Remove `task_types` from agent registration**: The registration payload no longer accepts task type definitions. The `task_types` field on the Agent struct is now managed exclusively by the controller.
- **Agent card detail panel**: Clicking a custom agent card opens an interactive config panel where admins can add, edit, and remove task type definitions with proper form controls (not raw JSON).
- **API endpoint for updating task types**: Add `PUT /api/agents/{id}/task-types` to save task type definitions from the UI.
- **Task type editor UI**: The config panel provides controls to:
  - Add/remove task types (name + description)
  - Add/remove fields per task type (name, label, type, required, placeholder, options)
  - Preview what the job creation form will look like
- **Update Python example**: Remove `task_types` from the registration call since it's now managed in the UI.

## Capabilities

### New Capabilities
- `ui-task-type-editor`: Interactive UI for managing custom agent task type definitions from the agent card

### Modified Capabilities
- `agent-task-definitions`: Task types no longer come from registration; managed by controller via API

## Impact

- **API**: New `PUT /api/agents/{id}/task-types` endpoint; registration ignores `task_types` field
- **Frontend**: Agent card click handler, task type editor panel with add/edit/remove controls
- **Python example**: Simplified registration (no `task_types`)
- **No database changes**: `task_types_json` column already exists on agents table
