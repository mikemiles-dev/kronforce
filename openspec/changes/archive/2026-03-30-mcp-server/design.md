## Context

Kronforce already has a complete REST API behind Axum with API key authentication and role-based access. We also just built an MCP HTTP client for the `Mcp` task type, so we understand the protocol well: JSON-RPC 2.0 over HTTP with SSE responses, `initialize` handshake, `tools/list` for discovery, `tools/call` for invocation, and `Mcp-Session-Id` for session tracking.

The MCP Streamable HTTP server spec requires:
- Accept POST requests with JSON-RPC messages
- Respond with `text/event-stream` (SSE) for requests, `202 Accepted` for notifications
- Track sessions via `Mcp-Session-Id` header
- Validate `Accept` and `Content-Type` headers
- Support `initialize` → `tools/list` → `tools/call` lifecycle

## Goals / Non-Goals

**Goals:**
- MCP server endpoint at `/mcp` that MCP clients can connect to
- 10 tools covering core Kronforce operations (jobs, executions, agents, groups, events, stats)
- Role-based tool visibility matching existing API permissions
- Reuse existing API key auth via HTTP headers
- Stateless — no persistent sessions needed (each tool call is independent)
- SSE response format for compatibility with MCP clients

**Non-Goals:**
- MCP resources or prompts — only tools
- MCP sampling/elicitation (AI model features)
- Streaming tool results (return complete response)
- WebSocket transport
- Server-initiated notifications to clients (notifications/tools_changed, etc.)

## Decisions

### 1. Hand-rolled MCP server (no rmcp crate)

Same approach as the MCP client — implement the JSON-RPC protocol directly with serde + Axum. The server side is even simpler than the client: receive JSON-RPC requests, dispatch to handlers, return SSE-formatted responses. ~300 lines.

**Alternatives considered:**
- `rmcp` crate server features: Complex handler trait system, would fight with our Axum integration
- Separate HTTP server: Unnecessary — mount on existing Axum router

### 2. Single Axum POST handler at `/mcp`

One handler receives all JSON-RPC messages, deserializes, and dispatches:

```
POST /mcp
  → initialize request → return server capabilities
  → tools/list request → return tool definitions (filtered by role)
  → tools/call request → dispatch to tool handler, return result
  → notification → return 202 Accepted
```

The handler sits behind the existing `auth_middleware` so the API key is already validated and available in request extensions.

### 3. Tool definitions with input schemas

Each tool is defined with a name, description, and JSON Schema for its input:

```rust
struct McpTool {
    name: &'static str,
    description: &'static str,
    input_schema: Value,  // JSON Schema
    min_role: ApiKeyRole, // Minimum role required
}
```

Tools are filtered by the caller's role before being returned by `tools/list`.

### 4. Tool handlers map to existing Db methods

Each tool call maps directly to an existing `Db` method or API helper:

| Tool | Maps To |
|------|---------|
| `list_jobs` | `Db::list_jobs()` |
| `get_job` | `Db::get_job()` |
| `create_job` | Job creation logic from `api/jobs.rs` |
| `trigger_job` | `SchedulerCommand::TriggerNow` |
| `list_executions` | `Db::list_all_executions()` |
| `get_execution` | `Db::get_execution()` |
| `list_agents` | `Db::list_agents()` |
| `list_groups` | `Db::get_distinct_groups()` |
| `list_events` | `Db::list_events()` |
| `get_system_stats` | Chart stats aggregation |

### 5. SSE response format

All responses use `text/event-stream` with the JSON-RPC response in a `data:` line:

```
HTTP/1.1 200 OK
Content-Type: text/event-stream
Mcp-Session-Id: <uuid>

event: message
data: {"jsonrpc":"2.0","id":1,"result":{...}}

```

Notifications receive `202 Accepted` with no body.

### 6. Session management

Generate a `Mcp-Session-Id` on the initialize request and return it. Validate it on subsequent requests. Sessions are stateless — we don't store state between requests. The session ID just prevents cross-session message mixing.

### 7. Auth integration

The `/mcp` endpoint is added to the `authed` router group, so `auth_middleware` runs first. The handler reads the `ApiKey` from request extensions to determine the role and filter available tools.

When no API keys are configured (auth disabled), all tools are available (same behavior as the REST API).

## Risks / Trade-offs

- **Stateless sessions** → We don't track which tools were listed before a call, so a client could call a tool without listing first. This is fine — the MCP spec doesn't require it.
- **No streaming** → Large tool results (e.g., listing 1000 jobs) are returned as a single SSE event. Fine for typical use cases.
- **SSE response parsing** → Clients must handle SSE format. All modern MCP clients do.
- **No server push** → We can't notify clients when tools change (e.g., new job created). Clients must re-list tools if needed. Acceptable for a job scheduler.
