## ADDED Requirements

### Requirement: MCP task type
The system SHALL support a new `Mcp` task type that connects to an MCP server, calls a specified tool with arguments, and captures the tool result as execution output. The task definition SHALL include: `server` (command or URL), `transport` ("stdio" or "http"), `tool` (tool name), and `arguments` (optional JSON object).

#### Scenario: Create job with MCP stdio task
- **WHEN** a user creates a job with task `{"type": "mcp", "server": "python3 my_server.py", "transport": "stdio", "tool": "analyze_logs", "arguments": {"path": "/var/log/app.log"}}`
- **THEN** the job is created and the task is stored in `task_json`

#### Scenario: Create job with MCP HTTP task
- **WHEN** a user creates a job with task `{"type": "mcp", "server": "http://localhost:3000/mcp", "transport": "http", "tool": "query_db", "arguments": {"sql": "SELECT count(*) FROM users"}}`
- **THEN** the job is created and the task is stored in `task_json`

#### Scenario: MCP task with no arguments
- **WHEN** a user creates an MCP task with `arguments` omitted or null
- **THEN** the tool is called with an empty arguments object

### Requirement: MCP stdio transport execution
The system SHALL execute stdio MCP tasks by spawning the server command as a subprocess, performing the MCP initialization handshake over stdin/stdout, calling the specified tool, capturing the result, and terminating the process.

#### Scenario: Successful stdio tool call
- **WHEN** an MCP stdio task executes and the tool returns a successful result with text content
- **THEN** the execution status is `Succeeded`, stdout contains the joined text content from all content blocks, and exit code is 0

#### Scenario: Stdio tool returns error
- **WHEN** an MCP stdio task executes and the tool returns `isError: true`
- **THEN** the execution status is `Failed`, stderr contains the error text, and exit code is 1

#### Scenario: Stdio server fails to start
- **WHEN** the server command cannot be spawned (e.g., command not found)
- **THEN** the execution status is `Failed`, stderr contains the spawn error, and exit code is -1

#### Scenario: Stdio server crashes during handshake
- **WHEN** the server process exits before completing the MCP initialization
- **THEN** the execution status is `Failed` with a protocol error in stderr

#### Scenario: Stdio uses cross-platform shell
- **WHEN** an MCP stdio task runs on Unix
- **THEN** the server command is spawned via `sh -c`
- **WHEN** an MCP stdio task runs on Windows
- **THEN** the server command is spawned via `cmd /C`

### Requirement: MCP HTTP transport execution
The system SHALL execute HTTP MCP tasks by sending JSON-RPC messages to the server URL via HTTP POST, performing the initialization handshake, calling the specified tool, and capturing the result.

#### Scenario: Successful HTTP tool call
- **WHEN** an MCP HTTP task executes and the tool returns a successful result
- **THEN** the execution status is `Succeeded`, stdout contains the tool result text, and exit code is 0

#### Scenario: HTTP server unreachable
- **WHEN** the MCP HTTP server URL is unreachable
- **THEN** the execution status is `Failed`, stderr contains the connection error, and exit code is -1

#### Scenario: HTTP server returns error response
- **WHEN** the HTTP server returns a non-200 status code
- **THEN** the execution status is `Failed` with the HTTP error in stderr

### Requirement: MCP initialization handshake
Before calling any tool, the MCP client SHALL perform the required initialization handshake: send an `initialize` request with client info and capabilities, receive the server's capabilities response, then send a `notifications/initialized` notification.

#### Scenario: Successful handshake
- **WHEN** the MCP client connects to a server
- **THEN** it sends `initialize` with `clientInfo: {name: "kronforce", version: "0.1.0"}`, receives the server response, and sends `notifications/initialized` before proceeding

#### Scenario: Protocol version mismatch
- **WHEN** the server responds with an unsupported protocol version
- **THEN** the execution fails with a protocol error in stderr

### Requirement: MCP result mapping
Tool results SHALL be mapped to the existing `CommandResult` structure. All text content blocks SHALL be joined with newlines as stdout. Image and other non-text content blocks SHALL be represented as metadata descriptions in stdout.

#### Scenario: Single text content block
- **WHEN** a tool returns `{"content": [{"type": "text", "text": "result data"}], "isError": false}`
- **THEN** stdout is `"result data"`

#### Scenario: Multiple text content blocks
- **WHEN** a tool returns two text content blocks
- **THEN** stdout contains both texts joined with a newline

#### Scenario: Non-text content block
- **WHEN** a tool returns an image content block
- **THEN** stdout contains a description like `[image: image/png, 1234 bytes]`

### Requirement: MCP tool discovery API
The system SHALL provide a `GET /api/mcp/tools` endpoint that connects to an MCP server, performs the handshake, calls `tools/list`, and returns the available tools with their input schemas. The endpoint SHALL require authentication.

#### Scenario: Discover tools via stdio
- **WHEN** an authenticated user requests `GET /api/mcp/tools?server=python3+server.py&transport=stdio`
- **THEN** the system spawns the server, performs the handshake, calls `tools/list`, returns the tools array, and terminates the server

#### Scenario: Discover tools via HTTP
- **WHEN** an authenticated user requests `GET /api/mcp/tools?server=http://localhost:3000/mcp&transport=http`
- **THEN** the system connects, performs the handshake, calls `tools/list`, and returns the tools array

#### Scenario: Server has no tools
- **WHEN** the MCP server returns an empty tools list
- **THEN** the endpoint returns `{"tools": []}`

#### Scenario: Discovery failure
- **WHEN** the server cannot be reached or the handshake fails
- **THEN** the endpoint returns an appropriate error (400 or 502)

### Requirement: Variable substitution in MCP arguments
The system SHALL apply global variable substitution (`{{VAR_NAME}}`) to MCP tool arguments before execution, consistent with how variables are substituted in other task types.

#### Scenario: Variable in argument value
- **WHEN** an MCP task has arguments `{"path": "{{LOG_DIR}}/app.log"}` and variable `LOG_DIR=/var/log`
- **THEN** the tool is called with arguments `{"path": "/var/log/app.log"}`

### Requirement: MCP task timeout
MCP task execution SHALL respect the job's `timeout_secs` setting. If the timeout is reached before the tool call completes, the stdio server process SHALL be killed or the HTTP connection dropped, and the execution SHALL be marked as `TimedOut`.

#### Scenario: Task times out
- **WHEN** an MCP task takes longer than the configured timeout
- **THEN** the execution status is `TimedOut` and the server process is terminated

#### Scenario: No timeout configured
- **WHEN** an MCP task has no timeout set
- **THEN** the task runs until the tool completes (no time limit)

### Requirement: MCP task cancellation
MCP task execution SHALL support cancellation. When cancelled, the stdio server process SHALL be killed or the HTTP connection dropped.

#### Scenario: Task cancelled by user
- **WHEN** a user cancels a running MCP task execution
- **THEN** the execution status is `Cancelled` and the server process is terminated

### Requirement: MCP task UI configuration
The job create/edit modal SHALL include an MCP task type option with: transport selector (Stdio/HTTP), server input field, "Discover Tools" button, tool dropdown populated from discovery, and a dynamic arguments form generated from the selected tool's `inputSchema`.

#### Scenario: Select MCP task type in modal
- **WHEN** the user selects "MCP" as the task type
- **THEN** the form shows transport selector, server input, discover button, tool dropdown, and arguments area

#### Scenario: Discover and select a tool
- **WHEN** the user enters a server and clicks "Discover Tools"
- **THEN** the tool dropdown is populated with available tools, and selecting a tool generates an argument form based on its `inputSchema`

#### Scenario: Edit existing MCP job
- **WHEN** the user opens the edit modal for an MCP job
- **THEN** the transport, server, tool, and arguments are pre-populated from the job data
