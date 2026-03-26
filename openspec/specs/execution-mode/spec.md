
### Requirement: Job creation has execution mode selector
The create job modal SHALL have a top-level "Execution Mode" selector with three options: Local, Standard Agent, and Custom Agent.

#### Scenario: Local mode selected
- **WHEN** the user selects "Local" execution mode
- **THEN** the task type options show the 5 built-in types (Shell, HTTP, SQL, FTP, Script) and no agent target options are shown

#### Scenario: Standard Agent mode selected
- **WHEN** the user selects "Standard Agent" execution mode
- **THEN** the task type options show the 5 built-in types and the target sub-options appear (Specific Agent, Any Agent, All Agents) with agent dropdowns filtered to standard agents only

#### Scenario: Custom Agent mode selected
- **WHEN** the user selects "Custom Agent" execution mode
- **THEN** an agent dropdown appears filtered to custom agents only, and the task type section updates to show the selected agent's registered task types

### Requirement: Agent dropdowns filter by agent type
When in Standard Agent mode, agent dropdowns SHALL only show standard agents. When in Custom Agent mode, agent dropdowns SHALL only show custom agents.

#### Scenario: Standard mode agent dropdown
- **WHEN** the user is in Standard Agent mode and opens the agent dropdown
- **THEN** only agents with `agent_type = "standard"` are listed

#### Scenario: Custom mode agent dropdown
- **WHEN** the user is in Custom Agent mode and opens the agent dropdown
- **THEN** only agents with `agent_type = "custom"` are listed

#### Scenario: No agents of required type
- **WHEN** the user selects a mode but no online agents of that type exist
- **THEN** the dropdown shows "No online [standard/custom] agents"

### Requirement: Dispatch filters agents by task type
The executor SHALL filter agents by type when dispatching to `Any`, `All`, or `Tagged` targets. Jobs with `Custom` tasks SHALL only dispatch to custom agents. Jobs with built-in tasks SHALL only dispatch to standard agents.

#### Scenario: Any agent with built-in task
- **WHEN** a job with a Shell task and target `Any` is executed
- **THEN** the executor selects only from online standard agents

#### Scenario: Any agent with custom task
- **WHEN** a job with a Custom task and target `Any` is executed
- **THEN** the executor selects only from online custom agents

#### Scenario: All agents with built-in task
- **WHEN** a job with an HTTP task and target `All` is executed
- **THEN** the executor dispatches only to online standard agents

#### Scenario: Tagged agents filtered by type
- **WHEN** a job with a Custom task and target `Tagged { tag: "ml" }` is executed
- **THEN** the executor selects only from online custom agents with the tag "ml"

#### Scenario: No matching agents available
- **WHEN** a job dispatches to `Any` but no online agents of the required type exist
- **THEN** the executor returns an `AgentUnavailable` error

### Requirement: Default execution mode is Local
The create job modal SHALL default to "Local" execution mode, preserving backward compatibility with the current behavior.

#### Scenario: Opening create modal
- **WHEN** the user opens the create job modal
- **THEN** "Local" is pre-selected as the execution mode and the built-in task types are shown

#### Scenario: Editing an existing local job
- **WHEN** the user edits a job with no target or target `Local`
- **THEN** "Local" mode is pre-selected

#### Scenario: Editing an existing custom agent job
- **WHEN** the user edits a job with a `Custom` task type targeting a specific agent
- **THEN** "Custom Agent" mode is pre-selected with the agent and custom task type restored
