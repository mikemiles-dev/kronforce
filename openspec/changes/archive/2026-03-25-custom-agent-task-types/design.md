## Context

The system has 5 built-in task types (`Shell`, `Sql`, `Ftp`, `Http`, `Script`) defined as a Rust enum `TaskType` in `models.rs`. Custom agents (pull-based) receive the same task types as standard agents (push-based). The job creation UI hardcodes these 5 types as radio buttons with static form fields. Agent targets (`Local`, `Agent`, `Any`, `All`, `Tagged`) don't distinguish between standard and custom agents.

Key files:
- `src/models.rs`: `TaskType` enum, `AgentTarget` enum, `Agent` struct
- `src/protocol.rs`: `AgentRegistration` struct (registration payload)
- `src/api.rs`: Agent registration endpoint, job CRUD
- `src/executor.rs`: Dispatch logic (`dispatch_to_any`, `dispatch_to_all`, `dispatch_to_specific_agent`)
- `src/db.rs`: Agent storage, migrations
- `src/dashboard.html`: Job creation modal, task type radio buttons, form fields

## Goals / Non-Goals

**Goals:**
- Custom agents declare task type definitions during registration
- UI dynamically renders custom task forms based on agent definitions
- Job creation has clear execution mode separation (Local / Standard Agent / Custom Agent)
- Any/All/Tagged targets are agent-type-aware
- Custom task data stored as typed JSON (`Custom { agent_task_type, data }`)

**Non-Goals:**
- Client-side field validation beyond required/optional (server doesn't validate custom field schemas)
- Versioning of task type definitions (agent re-registration overwrites)
- Sharing task type definitions across agents (each agent owns its own)
- Custom task types for standard agents (standard agents use built-in types only)

## Decisions

### 1. Task type definitions as JSON in the agents table

**Decision**: Add `task_types_json TEXT` column to the `agents` table via migration. The column stores a JSON array of task type definitions. Each definition has:

```json
{
  "name": "python",
  "description": "Run a Python script",
  "fields": [
    { "name": "script", "label": "Script", "type": "textarea", "required": true, "placeholder": "print('hello')" },
    { "name": "args", "label": "Arguments", "type": "text", "required": false }
  ]
}
```

Field types supported: `text`, `textarea`, `number`, `select`, `password`. Select fields include an `options` array of `{ value, label }`.

**Rationale**: Storing on the agent is natural — the agent owns its capabilities. Re-registration updates the definitions. No separate table needed. The UI fetches definitions via the existing `/api/agents/{id}` endpoint (already returns the full agent struct).

**Alternative considered**: Separate `task_type_definitions` table — rejected because definitions are 1:1 with agents and don't need their own lifecycle.

### 2. New `Custom` variant in `TaskType` enum

**Decision**: Add a `Custom` variant to the existing `TaskType` enum:

```rust
pub enum TaskType {
    Shell { command: String },
    // ... existing variants ...
    Custom {
        agent_task_type: String,
        data: serde_json::Value,
    },
}
```

`agent_task_type` is the name from the definition (e.g., `"python"`). `data` is the arbitrary JSON object with field values. The custom agent receives this as-is in the queue and handles it however it wants.

**Rationale**: Keeps `TaskType` as the single source of truth for all task shapes. The `Custom` variant is open-ended (arbitrary JSON) while built-in types remain strongly typed. Serde handles serialization via `#[serde(tag = "type")]`.

**Alternative considered**: Separate `custom_task_json` field on `Job` — rejected because it splits task data across two fields and complicates the executor.

### 3. Execution mode selector replaces target section in UI

**Decision**: Replace the current "Target" radio group (Local / Specific Agent / Any Agent / All Agents) with a two-level selection:

**Level 1 — Execution Mode** (radio buttons):
- **Local** — runs on controller (current default)
- **Standard Agent** — runs on standard agents
- **Custom Agent** — runs on custom agents

**Level 2 — shown based on mode:**
- **Local**: No target options needed
- **Standard Agent**: Target sub-options (Specific Agent / Any / All), agent dropdown filtered to standard agents only
- **Custom Agent**: Agent dropdown filtered to custom agents only. Selecting an agent loads its task type definitions and replaces the task type radio buttons with the agent's custom types. Form fields render dynamically using `formField()`.

The task type section changes based on mode:
- Local/Standard Agent: Show the 5 built-in types (Shell/HTTP/SQL/FTP/Script) as today
- Custom Agent: Show task types from the selected agent's definitions

**Rationale**: This makes the mode distinction explicit and prevents invalid combinations (e.g., custom task on standard agent, or shell on custom-only agent). The two-level approach keeps the UI clean.

### 4. Agent-type filtering in dispatch

**Decision**: Modify `dispatch_to_any()`, `dispatch_to_all()`, and `dispatch_to_tagged()` to filter agents by type based on the task:

- If `task` is `Custom { .. }` → only dispatch to custom agents
- Otherwise → only dispatch to standard agents

Add `get_online_agents_by_type(agent_type)` to `db.rs`. Update the three dispatch methods to call it.

**Rationale**: This is the minimal backend change needed. The UI enforces correct combinations at creation time, but the backend should also enforce it at dispatch time as a safety net. No changes to `AgentTarget` enum needed — `Any` means "any agent of the appropriate type".

### 5. Registration protocol extension

**Decision**: Add optional `task_types` field to `AgentRegistration`:

```rust
pub struct AgentRegistration {
    pub name: String,
    pub tags: Vec<String>,
    pub hostname: String,
    pub address: String,
    pub port: u16,
    pub agent_type: Option<String>,
    pub task_types: Option<Vec<TaskTypeDefinition>>,
}
```

Where `TaskTypeDefinition` is:
```rust
pub struct TaskTypeDefinition {
    pub name: String,
    pub description: Option<String>,
    pub fields: Vec<TaskFieldDefinition>,
}

pub struct TaskFieldDefinition {
    pub name: String,
    pub label: String,
    pub field_type: String, // "text", "textarea", "number", "select", "password"
    pub required: Option<bool>,
    pub placeholder: Option<String>,
    pub options: Option<Vec<FieldOption>>, // for select type
}

pub struct FieldOption {
    pub value: String,
    pub label: String,
}
```

The `task_types` field is optional and ignored for standard agents. On registration, the controller serializes it to JSON and stores in `task_types_json`.

**Rationale**: Extending the existing registration is cleaner than a separate endpoint. Standard agents don't send this field and are unaffected.

### 6. No new API endpoint needed

**Decision**: The existing `GET /api/agents/{id}` already returns the full `Agent` struct. Adding `task_types_json` to the agent and including it in the serialized response is sufficient — no new endpoint needed. The UI fetches agent details to get task type definitions.

Add a `task_types` field to the `Agent` struct:
```rust
pub struct Agent {
    // ... existing fields ...
    pub task_types: Vec<TaskTypeDefinition>,
}
```

The `list_agents` / `get_agent` queries already return all columns. Adding `task_types_json` to the query and deserializing is minimal change.

**Rationale**: Avoids API proliferation. The agent detail already has everything the UI needs.

## Risks / Trade-offs

- **No server-side validation of custom task data** → The controller stores whatever JSON the UI sends as `Custom { data }`. The custom agent is responsible for validating. Acceptable because custom agents are by definition custom — the controller can't know their schemas.
- **Task type definitions only available while agent is registered** → If a custom agent is removed, its task types disappear from the dropdown. Jobs already created with those types still have the data in `task_json` and can be viewed, but not edited/cloned. Acceptable for now.
- **UI complexity increase** → The execution mode selector adds a layer to job creation. Mitigated by defaulting to "Local" (most common) and only showing the mode selector — the rest follows naturally.
- **`Custom` variant in `TaskType` makes the enum open-ended** → `run_task()` (local execution) will need a branch that returns an error for `Custom` tasks since they can't run locally. Simple to handle.
