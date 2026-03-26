## 1. Backend: API and Registration Changes

- [x] 1.1 Add `PUT /api/agents/{id}/task-types` endpoint that accepts `{ task_types: [...] }` and updates `task_types_json`
- [x] 1.2 Add `update_agent_task_types(id, task_types)` method to `src/db.rs`
- [x] 1.3 Update `register_agent()` to ignore `task_types` from registration and preserve existing `task_types_json` across re-registrations

## 2. Frontend: Agent Card Click and Config Panel

- [x] 2.1 Add click handler on custom agent cards that toggles an inline config panel below the card
- [x] 2.2 Add CSS for the config panel (expandable section with border, padding, background)
- [x] 2.3 Render the task type editor when the panel opens, loading existing task types from the agent data

## 3. Frontend: Task Type Editor Controls

- [x] 3.1 Render each task type as a section with editable name input, description input, and delete button
- [x] 3.2 Render field rows within each task type with inputs for: name, label, type (dropdown), required (checkbox), placeholder
- [x] 3.3 Add "Add Field" button per task type that appends a new empty field row
- [x] 3.4 Add remove button per field row
- [x] 3.5 When field type is "select", show an additional options textarea for value:label pairs
- [x] 3.6 Add "Add Task Type" button at the bottom of the editor

## 4. Frontend: Save and Validation

- [x] 4.1 Add Save button that collects all editor state into a task_types JSON array
- [x] 4.2 Validate that every task type has a name before saving
- [x] 4.3 Send PUT to `/api/agents/{id}/task-types` and show success/error toast
- [x] 4.4 Refresh agent data after successful save

## 5. Cleanup

- [x] 5.1 Remove `task_types` from registration in `examples/custom_agent.py`
- [x] 5.2 Remove the static task types display from the agent card (replaced by config panel)
