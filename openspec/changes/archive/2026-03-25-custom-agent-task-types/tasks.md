## 1. Backend: Models and Protocol

- [x] 1.1 Add `TaskTypeDefinition`, `TaskFieldDefinition`, and `FieldOption` structs to `src/models.rs` (or `src/protocol.rs`)
- [x] 1.2 Add `Custom { agent_task_type: String, data: serde_json::Value }` variant to the `TaskType` enum in `src/models.rs`
- [x] 1.3 Add `task_types: Option<Vec<TaskTypeDefinition>>` field to `AgentRegistration` in `src/protocol.rs`
- [x] 1.4 Add `task_types: Vec<TaskTypeDefinition>` field to the `Agent` struct in `src/models.rs`

## 2. Backend: Database

- [x] 2.1 Add migration to add `task_types_json TEXT` column to the `agents` table in `src/db.rs`
- [x] 2.2 Update `upsert_agent()` to store `task_types_json` from the agent's `task_types` field
- [x] 2.3 Update agent row-reading helper to deserialize `task_types_json` into `Vec<TaskTypeDefinition>` (default to empty vec if null)
- [x] 2.4 Add `get_online_agents_by_type(agent_type: AgentType)` query to `src/db.rs`

## 3. Backend: Registration and API

- [x] 3.1 Update `register_agent()` in `src/api.rs` to read `task_types` from registration payload and store on the `Agent`
- [x] 3.2 Verify `GET /api/agents` and `GET /api/agents/{id}` return `task_types` in the response (via Agent struct serialization)

## 4. Backend: Executor Dispatch

- [x] 4.1 Handle `Custom` variant in `run_task()` — return a failed result with error "custom tasks require a custom agent"
- [x] 4.2 Update `dispatch_to_any()` to filter agents by type: custom agents for `Custom` tasks, standard agents for built-in tasks
- [x] 4.3 Update `dispatch_to_all()` with the same agent-type filtering
- [x] 4.4 Update `dispatch_to_tagged()` with the same agent-type filtering

## 5. Frontend: Execution Mode Selector

- [x] 5.1 Add "Execution Mode" radio group (Local / Standard Agent / Custom Agent) to the create job modal HTML, above the current task type section
- [x] 5.2 Add `updateExecutionMode()` handler that shows/hides the appropriate sections based on selected mode
- [x] 5.3 In Local mode: show built-in task types, hide all target options
- [x] 5.4 In Standard Agent mode: show built-in task types, show target sub-options (Specific/Any/All) with agent dropdown filtered to standard agents
- [x] 5.5 In Custom Agent mode: show custom agent dropdown, hide built-in task types, show dynamic task type section

## 6. Frontend: Dynamic Custom Task Forms

- [x] 6.1 Add `onCustomAgentSelected()` handler — fetch agent details, extract `task_types`, render custom task type radio buttons
- [x] 6.2 Add `onCustomTaskTypeSelected(taskType)` handler — render form fields dynamically using `formField()` based on the task type's field definitions
- [x] 6.3 Update `submitJobForm()` to build a `Custom { agent_task_type, data }` task when in Custom Agent mode, collecting field values into a JSON object

## 7. Frontend: Edit Job Support

- [x] 7.1 Update `openEditModal()` to detect `Custom` task type and set execution mode to "Custom Agent"
- [x] 7.2 Restore custom agent selection, task type, and field values when editing a custom agent job
- [x] 7.3 Default to "Local" mode when opening create modal (backward compatible)

## 8. Frontend: Agent Dropdown Filtering

- [x] 8.1 Update `populateAgentSelect()` to accept an `agentType` filter parameter and only show agents of that type
- [x] 8.2 Show "No online standard agents" or "No online custom agents" when filtered list is empty

## 9. Python Example

- [x] 9.1 Update `examples/custom_agent.py` to include `task_types` in the registration payload with a sample "python" task type definition
- [x] 9.2 Update the `execute_task()` function to handle `custom` task type by reading `agent_task_type` and `data` fields
