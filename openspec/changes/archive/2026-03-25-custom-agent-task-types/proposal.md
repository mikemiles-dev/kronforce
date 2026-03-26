## Why

Custom agents can handle arbitrary workloads (Python scripts, ML pipelines, custom APIs), but the job creation UI only offers the 5 built-in task types (Shell, HTTP, SQL, FTP, Script). When a user targets a custom agent, they're forced to shoehorn their work into "Shell" even if the agent implements something completely different. Additionally, `Any Agent` and `All Agents` targets are agent-type-blind — they can dispatch a shell job to a custom agent that doesn't support it, or vice versa. The system needs a clear separation between standard agent jobs and custom agent workflows, with custom agents able to define their own task types and form fields.

## What Changes

- **Custom agents register task type definitions**: During registration, a custom agent declares the task types it supports. Each task type includes a name, description, and a list of form fields (name, type, label, placeholder, required). These definitions are stored with the agent.
- **New `Custom` task type variant**: Add a `Custom { agent_task_type, data }` variant to `TaskType` that stores the custom type name and arbitrary JSON data. This is what gets stored in the job and sent to the agent.
- **Job creation UI splits by execution mode**: The create job modal gets a top-level "Execution Mode" selector: **Local**, **Standard Agent**, or **Custom Agent**. This replaces the current combined Target section.
  - **Local**: Current behavior (Shell/HTTP/SQL/FTP/Script task types, runs on controller)
  - **Standard Agent**: Current behavior (Shell/HTTP/SQL/FTP/Script, target = specific/any/all standard agents)
  - **Custom Agent**: Shows only custom agents in the agent selector. Task type options are dynamically loaded from the selected agent's registered definitions. Form fields render dynamically based on the definition.
- **Agent-type-aware targeting**: `Any Agent` and `All Agents` targets filter by agent type — standard mode only picks standard agents, custom mode only picks custom agents.
- **API endpoint for task type definitions**: Add `GET /api/agents/{id}/task-types` to fetch an agent's registered task type definitions (used by the UI to render forms dynamically).

## Capabilities

### New Capabilities
- `agent-task-definitions`: Custom agents register task type definitions with field schemas; definitions stored and served via API
- `execution-mode`: Job creation separates Local / Standard Agent / Custom Agent modes; agent-type-aware dispatch

### Modified Capabilities

## Impact

- **Models**: New `Custom` variant in `TaskType` enum, new `TaskTypeDefinition` struct
- **Protocol**: `AgentRegistration` gains optional `task_types` field
- **Database**: Migration adds `task_types_json` column to `agents` table
- **API**: New endpoint for task type definitions; registration stores definitions
- **Executor**: `dispatch_to_any` and `dispatch_to_all` filter agents by type based on task
- **Frontend**: Job creation modal restructured with execution mode selector; dynamic form rendering for custom task types
- **Python example**: Updated to register task type definitions on startup
