## ADDED Requirements

### Requirement: MCP server endpoint
The system SHALL provide an MCP Streamable HTTP server endpoint at `POST /mcp` that accepts JSON-RPC 2.0 messages and responds with SSE-formatted JSON-RPC responses. The endpoint SHALL require authentication via API key.

#### Scenario: Initialize handshake
- **WHEN** an MCP client sends an `initialize` request to `POST /mcp`
- **THEN** the server responds with SSE containing server info, protocol version, and capabilities including `tools`

#### Scenario: Unauthenticated request
- **WHEN** a request is sent to `/mcp` without an API key and keys are configured
- **THEN** the server returns 401 Unauthorized

#### Scenario: Notification receives 202
- **WHEN** an MCP client sends a `notifications/initialized` notification
- **THEN** the server returns 202 Accepted with no body

#### Scenario: Invalid Accept header
- **WHEN** a request is sent without `Accept: application/json, text/event-stream`
- **THEN** the server returns 406 Not Acceptable

### Requirement: MCP tool discovery
The system SHALL respond to `tools/list` requests with the list of available tools filtered by the caller's API key role. Each tool SHALL include a name, description, and JSON Schema for its input parameters.

#### Scenario: Viewer lists tools
- **WHEN** a viewer-role API key calls `tools/list`
- **THEN** only read-only tools are returned (list_jobs, get_job, list_executions, get_execution, list_agents, list_groups, list_events, get_system_stats)

#### Scenario: Operator lists tools
- **WHEN** an operator-role API key calls `tools/list`
- **THEN** read-only tools plus mutation tools are returned (create_job, trigger_job)

#### Scenario: Admin lists tools
- **WHEN** an admin-role API key calls `tools/list`
- **THEN** all tools are returned

### Requirement: Read-only MCP tools
The system SHALL expose the following read-only tools available to all authenticated roles.

#### Scenario: list_jobs tool
- **WHEN** an MCP client calls `list_jobs` with optional `group`, `status`, and `search` arguments
- **THEN** the tool returns a JSON array of jobs with name, id, status, group, schedule, and last execution info

#### Scenario: get_job tool
- **WHEN** an MCP client calls `get_job` with a `name` or `id` argument
- **THEN** the tool returns the full job details including task, schedule, dependencies, and execution counts

#### Scenario: list_executions tool
- **WHEN** an MCP client calls `list_executions` with optional `status` and `limit` arguments
- **THEN** the tool returns recent executions with job name, status, output excerpt, and timestamps

#### Scenario: get_execution tool
- **WHEN** an MCP client calls `get_execution` with an `id` argument
- **THEN** the tool returns full execution details including stdout, stderr, exit code, and trigger info

#### Scenario: list_agents tool
- **WHEN** an MCP client calls `list_agents`
- **THEN** the tool returns all registered agents with name, status, type, and tags

#### Scenario: list_groups tool
- **WHEN** an MCP client calls `list_groups`
- **THEN** the tool returns all job group names

#### Scenario: list_events tool
- **WHEN** an MCP client calls `list_events` with optional `limit` argument
- **THEN** the tool returns recent system events with kind, severity, message, and timestamp

#### Scenario: get_system_stats tool
- **WHEN** an MCP client calls `get_system_stats`
- **THEN** the tool returns job counts, execution success/failure rates, agent status counts, and group counts

### Requirement: Mutation MCP tools
The system SHALL expose mutation tools available to operator and admin roles.

#### Scenario: create_job tool
- **WHEN** an MCP client calls `create_job` with `name`, `task`, `schedule`, and optional `group`, `description`, `timeout_secs`
- **THEN** the tool creates the job and returns the new job's ID and name

#### Scenario: trigger_job tool
- **WHEN** an MCP client calls `trigger_job` with a job `name` or `id`
- **THEN** the tool triggers the job and returns the execution ID

#### Scenario: Viewer denied mutation
- **WHEN** a viewer-role API key calls `create_job` or `trigger_job`
- **THEN** the tool returns an error indicating insufficient permissions

### Requirement: Session management
The server SHALL generate a unique `Mcp-Session-Id` on the initialize response and return it in the response header. Subsequent requests SHOULD include this session ID.

#### Scenario: Session ID returned on initialize
- **WHEN** an MCP client sends an `initialize` request
- **THEN** the response includes a `Mcp-Session-Id` header with a UUID

#### Scenario: Session ID validated on subsequent requests
- **WHEN** a client sends a `tools/call` request with a valid `Mcp-Session-Id`
- **THEN** the request is processed normally

### Requirement: MCP server configuration
The MCP server endpoint SHALL be configurable via environment variable.

#### Scenario: MCP enabled by default
- **WHEN** `KRONFORCE_MCP_ENABLED` is not set
- **THEN** the `/mcp` endpoint is available

#### Scenario: MCP disabled
- **WHEN** `KRONFORCE_MCP_ENABLED=false` is set
- **THEN** the `/mcp` endpoint returns 404
