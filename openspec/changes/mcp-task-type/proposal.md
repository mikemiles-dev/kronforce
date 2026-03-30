## Why

Kronforce automates workloads by executing tasks (shell commands, HTTP requests, scripts, etc.), but it has no way to interact with AI tool ecosystems. The Model Context Protocol (MCP) is the emerging standard for connecting applications to AI tools — databases, file systems, APIs, and custom integrations all expose capabilities via MCP servers. Adding an MCP task type lets Kronforce orchestrate AI tool chains on a schedule, react to events, and feed results into downstream jobs — turning it into a workflow engine for both traditional ops and AI-powered automation.

## What Changes

- Add a new `Mcp` task type that connects to an MCP server, calls a specified tool with arguments, and captures the tool result as execution output
- Support two MCP transport modes:
  - **stdio** — spawn an MCP server as a subprocess (e.g., `python3 my_server.py`), communicate via stdin/stdout JSON-RPC
  - **HTTP** — connect to a remote MCP server URL via Streamable HTTP transport
- Implement the MCP client handshake: `initialize` → server capabilities → `notifications/initialized` → ready
- Implement `tools/list` for discovering available tools from a connected server
- Implement `tools/call` to invoke a tool and capture the response content as stdout
- Add a `GET /api/mcp/tools` endpoint that connects to a server and returns its available tools (for the UI tool picker)
- Add MCP task configuration in the job create/edit modal: server command/URL, tool name, arguments (dynamic form based on tool's `inputSchema`)
- Variable substitution works in MCP tool arguments (`{{VAR_NAME}}`)
- Output extraction rules work on MCP tool result text (regex/jsonpath)
- Use the `rmcp` crate (official Rust MCP SDK) for protocol implementation

## Capabilities

### New Capabilities
- `mcp-task-type`: MCP client integration including stdio and HTTP transports, tool discovery, tool invocation, result capture, and UI configuration

### Modified Capabilities

## Impact

- **Dependencies**: New crate `rmcp` (official Rust MCP SDK) added to `Cargo.toml`
- **Database**: No schema changes — MCP is a new `TaskType` variant serialized as JSON in the existing `task_json` column
- **Backend**: New `src/executor/tasks/mcp.rs` for MCP client logic. New `TaskType::Mcp` variant in models. New `GET /api/mcp/tools` endpoint for tool discovery.
- **Frontend**: MCP task type option in the job modal with server config, tool picker dropdown (populated via discovery), and dynamic argument form based on tool's input schema
- **No breaking changes**: New task type, additive only. Existing jobs and APIs unaffected.
