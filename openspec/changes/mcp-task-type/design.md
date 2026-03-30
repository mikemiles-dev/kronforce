## Context

Kronforce executes tasks via the `TaskType` enum in `src/db/models/task.rs`, dispatched through `run_task()` in `src/executor/local.rs`. Each task type has a dedicated module in `src/executor/tasks/` (shell.rs, http.rs, script.rs, etc.). Tasks receive parameters, execute, and return a `CommandResult` with status, exit code, stdout, and stderr.

The Model Context Protocol (MCP) uses JSON-RPC 2.0 over either stdio (subprocess) or Streamable HTTP. The lifecycle is: `initialize` handshake → `tools/list` for discovery → `tools/call` for invocation. Tool results contain typed content blocks (text, image, resource links). The official Rust SDK is `rmcp` on crates.io.

## Goals / Non-Goals

**Goals:**
- New `Mcp` task type that calls a tool on an MCP server and captures the result
- Support stdio transport (spawn subprocess) and HTTP transport (remote URL)
- Tool discovery API so the UI can show a tool picker with argument schemas
- MCP server connection lifecycle managed per-execution (connect, call, disconnect)
- Tool result text content captured as stdout, errors as stderr
- Works with existing variable substitution, output rules, retry, and notifications

**Non-Goals:**
- Kronforce acting as an MCP server (only client)
- Persistent/long-lived MCP connections (connection pool for MCP servers)
- MCP resources or prompts — only tools
- Streaming tool results (wait for complete response)
- MCP sampling/elicitation capabilities (these are for AI model interaction)
- Agent-side MCP execution (MCP tasks run on the controller only, for now)

## Decisions

### 1. Use `rmcp` crate for protocol implementation

The official Rust MCP SDK (`rmcp`) handles JSON-RPC framing, the initialization handshake, and typed request/response structures. This avoids hand-rolling JSON-RPC.

```toml
rmcp = { version = "0.1", features = ["client", "transport-child-process", "transport-sse-client"] }
```

**Alternatives considered:**
- Hand-rolled JSON-RPC: Error-prone, protocol is complex (handshake, capabilities, content types)
- `rust-mcp-sdk`: Less mature than `rmcp`
- Shelling out to an MCP CLI: Adds process overhead, harder to capture structured results

### 2. Connection-per-execution model

Each MCP task execution:
1. Spawns/connects to the MCP server
2. Performs the `initialize` handshake
3. Calls `tools/call` with the specified tool and arguments
4. Captures the result
5. Disconnects/kills the server process

No persistent connections. This is simpler, avoids connection state management, and matches how shell tasks work (spawn, execute, done). The overhead of the handshake (~10ms for stdio, ~50ms for HTTP) is negligible for scheduled jobs.

**Alternatives considered:**
- Connection pool for MCP servers: Complex lifecycle management, server crashes, reconnection. Overkill for job scheduling where executions are seconds/minutes apart.
- Long-lived server process: Requires health monitoring, restart logic, state management. Future enhancement if needed.

### 3. `TaskType::Mcp` variant structure

```rust
TaskType::Mcp {
    /// stdio command (e.g., "python3 my_server.py") or HTTP URL
    server: String,
    /// "stdio" or "http"
    transport: McpTransport,
    /// Tool name to invoke
    tool: String,
    /// Tool arguments as JSON object
    arguments: Option<serde_json::Value>,
}

enum McpTransport {
    Stdio,
    Http,
}
```

The `server` field is either a shell command (for stdio) or a URL (for HTTP). This keeps the task definition simple and JSON-serializable.

### 4. Result mapping to CommandResult

MCP tool results contain content blocks. Map them to the existing `CommandResult`:

| MCP Result | CommandResult Field |
|---|---|
| `content[].text` (all text blocks joined) | `stdout` |
| `isError: true` message | `stderr` |
| `isError: false` | `status: Succeeded` |
| `isError: true` | `status: Failed` |
| Connection/protocol error | `status: Failed`, error in `stderr` |

Exit code: `0` for success, `1` for tool error, `-1` for connection/protocol error.

### 5. Tool discovery via `GET /api/mcp/tools`

A new API endpoint that takes a `server` and `transport` query parameter, connects to the MCP server, calls `tools/list`, and returns the tool definitions with their input schemas.

```
GET /api/mcp/tools?server=python3+my_server.py&transport=stdio
```

Returns:
```json
{
  "tools": [
    {
      "name": "analyze_logs",
      "description": "Analyze log files for patterns",
      "inputSchema": {
        "type": "object",
        "properties": {
          "path": {"type": "string", "description": "Log file path"},
          "query": {"type": "string", "description": "Search query"}
        },
        "required": ["path"]
      }
    }
  ]
}
```

This is called by the UI when configuring an MCP task — the user enters the server, clicks "Discover Tools", and gets a dropdown of available tools with auto-generated argument forms.

### 6. Stdio transport uses the cross-platform shell

For stdio, the server command is executed via `sh -c` (Unix) or `cmd /C` (Windows), matching the existing `run_command` pattern. The spawned process's stdin/stdout carry JSON-RPC messages, stderr is captured separately for server logs/errors.

### 7. HTTP transport uses reqwest

For HTTP, the client POSTs JSON-RPC messages to the server URL. SSE streaming is not needed since we wait for the complete tool response. A simple POST/response cycle per message suffices.

### 8. UI: MCP task form in job modal

The job modal's Task tab gets an "MCP" option alongside Shell, HTTP, SQL, etc. When selected:
1. **Server field** — text input for command (stdio) or URL (http)
2. **Transport toggle** — radio buttons: Stdio / HTTP
3. **"Discover Tools" button** — calls `GET /api/mcp/tools`, populates the tool dropdown
4. **Tool dropdown** — select from discovered tools
5. **Arguments form** — dynamically generated from the selected tool's `inputSchema` (text inputs for string fields, number inputs for number fields, etc.)

### 9. Timeout handling

MCP tasks respect the job's `timeout_secs` setting. The MCP client connection and tool call are wrapped in a tokio timeout. If the timeout fires, the stdio process is killed or the HTTP connection is dropped.

## Risks / Trade-offs

- **`rmcp` crate maturity** → The crate is relatively new. If it has bugs, we may need to fork or patch. Mitigation: the protocol is simple enough that we can fall back to raw JSON-RPC over reqwest/stdin if needed.
- **Stdio server lifecycle** → If the server process crashes during the handshake, the task fails. This is fine — retries handle transient failures.
- **Large tool results** → MCP tool results can be large (images, files). We apply the same 10MB output truncation as other tasks.
- **Security** → MCP servers can execute arbitrary code. Same risk as shell tasks — the operator chooses what servers to run. Document that MCP servers should be trusted.
- **No agent support** → MCP tasks only run on the controller. Adding agent support later requires shipping the MCP client to agents, which is a separate change.
