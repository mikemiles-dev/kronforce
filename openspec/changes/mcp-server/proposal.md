## Why

Kronforce has a full REST API, but AI tools and assistants increasingly communicate via the Model Context Protocol (MCP). Exposing Kronforce as an MCP server lets any MCP client — Claude, Cursor, custom AI agents, or other automation tools — discover and interact with jobs, executions, and agents through a standard protocol. An AI assistant could list running jobs, trigger a deployment, check if it succeeded, and create follow-up jobs, all through MCP tool calls without needing custom API integration.

## What Changes

- Add an MCP server endpoint at `/mcp` that implements the MCP Streamable HTTP transport
- Expose Kronforce operations as MCP tools: list/get/create/trigger jobs, list/get executions, list agents, list groups, list events, get system stats
- Authenticate MCP clients using existing API keys via the `Authorization: Bearer` header — same auth as the REST API
- Role-based tool visibility: viewer sees read-only tools, operator sees mutation tools, admin sees all
- Implement the MCP server protocol: handle `initialize`, `tools/list`, and `tools/call` requests, respond with proper JSON-RPC over SSE
- Add MCP server configuration: enable/disable via env var, configurable tool set

## Capabilities

### New Capabilities
- `mcp-server`: MCP Streamable HTTP server endpoint exposing Kronforce operations as discoverable tools with role-based access control

### Modified Capabilities

## Impact

- **Backend**: New `src/mcp_server.rs` module implementing the MCP server protocol. New route `/mcp` on the Axum router. Handlers map MCP tool calls to existing `Db` methods.
- **Authentication**: Reuses existing API key auth — no new auth mechanism. MCP clients send `Authorization: Bearer kf_...` in HTTP headers.
- **Configuration**: New env var `KRONFORCE_MCP_ENABLED` (default true) to enable/disable the MCP server endpoint.
- **No database changes**: MCP server calls existing DB/API methods.
- **No breaking changes**: New endpoint, additive only. Existing REST API unaffected.
