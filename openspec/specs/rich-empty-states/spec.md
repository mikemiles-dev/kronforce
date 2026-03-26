### Requirement: Rich empty state component
The system SHALL provide a `renderRichEmptyState(config)` function that generates an enhanced empty state with icon, title, description, multiple action buttons, and optional hint text.

#### Scenario: Rendering a rich empty state
- **WHEN** `renderRichEmptyState` is called with icon, title, description, and actions
- **THEN** it returns HTML with a centered card containing the icon, title, description text, and action buttons styled as the primary/ghost button variants

### Requirement: Jobs page has rich empty state
When no jobs exist, the jobs page SHALL show a rich empty state with quick-create template buttons.

#### Scenario: No jobs exist
- **WHEN** the jobs table renders with zero jobs
- **THEN** a rich empty state is shown with title "No jobs yet", description of what jobs do, and buttons for "Health Check", "Cron Task", "Event Watcher", and "Create from scratch"

#### Scenario: Clicking a template button
- **WHEN** the user clicks "Health Check" on the empty state
- **THEN** the create job modal opens with pre-filled health check defaults

### Requirement: Agents page has rich empty state
When no agents are registered, the agents page SHALL show a rich empty state with connection instructions.

#### Scenario: No agents registered
- **WHEN** the agents list renders with zero agents
- **THEN** a rich empty state is shown with title "No agents registered", a copy-paste command for starting a standard agent, and a link to Docs for custom agents

### Requirement: Executions page has rich empty state
When no executions exist, the executions page SHALL show a rich empty state explaining what executions are.

#### Scenario: No executions
- **WHEN** the all-executions table renders with zero results
- **THEN** a rich empty state is shown with title "No executions yet" and a prompt to create and trigger a job

### Requirement: Map page has rich empty state
When no jobs with dependencies exist, the map page SHALL show a rich empty state explaining the dependency graph.

#### Scenario: No dependencies to visualize
- **WHEN** the map renders with zero jobs or no dependencies
- **THEN** a rich empty state is shown explaining that the map visualizes job dependencies, with a prompt to add dependencies to jobs

### Requirement: Events page has rich empty state
When no events exist, the events page SHALL show a rich empty state explaining what generates events.

#### Scenario: No events
- **WHEN** the events list renders with zero results
- **THEN** a rich empty state is shown with title "No events yet" and a description of what actions generate events
