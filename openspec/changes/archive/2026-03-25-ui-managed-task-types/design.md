## Context

We just added `task_types` to the agent registration protocol and store them in `agents.task_types_json`. The job creation UI reads these to render dynamic forms. Now we need to move ownership of task type definitions from the agent registration to the UI, so admins manage workflow definitions through the dashboard and agent code only handles execution.

## Goals / Non-Goals

**Goals:**
- Task type definitions managed entirely through the UI
- Agent card opens a config panel for custom agents
- Interactive editor with proper form controls (not raw JSON editing)
- Preview of what the job creation form will look like
- API endpoint to persist changes

**Non-Goals:**
- Versioning or history of task type changes
- Role-based access control for who can edit task types
- Sharing task type definitions across agents

## Decisions

### 1. Agent card click opens inline config panel

**Decision**: Clicking a custom agent card toggles an expandable detail section below the card (not a modal). The section contains the task type editor. Standard agent cards remain click-inert.

**Rationale**: Inline expansion feels lighter than a modal and lets you see the agent card context while editing. Modals are already used for job creation/execution details — using inline expansion differentiates the agent config experience.

### 2. Task type editor as structured form controls

**Decision**: The editor renders each task type as a collapsible section with:
- **Header**: Task type name (editable text input) + description (editable text input) + delete button
- **Fields list**: Each field shown as a row with inputs for name, label, type (dropdown), required (checkbox), placeholder. Add/remove field buttons.
- **Add Task Type button** at the bottom
- **Save button** that PUTs the entire `task_types` array to the API

Field type dropdown options: text, textarea, number, select, password. When "select" is chosen, an additional "Options" textarea appears for entering value:label pairs (one per line).

**Rationale**: Structured controls are more user-friendly than raw JSON editing. Each field property maps to a clear input. The save is explicit so you can make multiple changes before persisting.

### 3. New API endpoint `PUT /api/agents/{id}/task-types`

**Decision**: Add a PUT endpoint that accepts `{ task_types: [...] }` and updates the agent's `task_types_json` column. Returns the updated agent.

**Rationale**: Separate endpoint rather than using the general agent update because task types are a distinct concern and this keeps the API surface clean.

### 4. Registration ignores task_types

**Decision**: The `register_agent()` handler ignores the `task_types` field in the registration payload. The field remains in `AgentRegistration` as `Option` for backward compatibility (old agents sending it won't break), but the value is not stored. The agent's existing `task_types_json` is preserved across re-registrations.

**Rationale**: Clean separation — registration is about identity and connectivity, not workflow configuration.

### 5. Add `update_agent_task_types()` to db.rs

**Decision**: Add a focused DB method that only updates the `task_types_json` column for a given agent ID, rather than re-using `upsert_agent()`.

**Rationale**: Updating task types shouldn't touch heartbeat, status, or other agent fields.

## Risks / Trade-offs

- **New agents start with no task types** → Admin must configure them after registration. Acceptable — this is the explicit UX we want.
- **Agent re-registration preserves task types** → If an agent is removed and re-added with the same name, it gets a fresh registration with no task types. The admin would need to reconfigure. Acceptable.
- **No validation that agent code handles configured task types** → The UI can define any task types but the agent might not implement handlers for them. Jobs would fail at the agent level with an "unsupported task type" error. This is acceptable — it's the admin's responsibility to configure types the agent supports.
