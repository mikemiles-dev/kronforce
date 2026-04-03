# Kronforce

[![CI](https://github.com/mikemiles-dev/kronforce/actions/workflows/ci.yml/badge.svg)](https://github.com/mikemiles-dev/kronforce/actions/workflows/ci.yml)
[![Release](https://github.com/mikemiles-dev/kronforce/actions/workflows/release.yml/badge.svg)](https://github.com/mikemiles-dev/kronforce/releases)
[![License: MIT/Apache-2.0](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE-MIT)

A workload automation and job scheduling engine built in Rust. Single binary, embedded dashboard, zero external dependencies.

![Kronforce Dashboard](screenshot.png)

- **One binary, batteries included** — controller, scheduler, REST API, and web dashboard in a single Rust binary. No Node, no Redis, no Postgres. Just SQLite.
- **12 task types** — Shell, HTTP, SQL, FTP/SFTP, Rhai scripting, MCP (AI tool protocol), file push, Kafka, RabbitMQ, MQTT, Redis, and custom agent-defined types
- **MCP server** — expose Kronforce as an MCP server so AI assistants can discover and manage jobs, trigger executions, and query results
- **Distributed agents** — push-based standard agents (Rust) or pull-based custom agents in any language (Python, Go, Node, etc.)
- **Event-driven workflows** — chain jobs based on completions, failures, output patterns, agent status changes, and more
- **Output intelligence** — extract values from stdout (regex/jsonpath), fail jobs when expected output is missing, compare output across runs
- **Built-in notifications** — email (SMTP), SMS (webhook), Slack, Microsoft Teams, and PagerDuty alerts on job failures, successes, and agent outages
- **Per-job controls** — cron scheduling (second precision), dependencies with time windows, timeouts, run-as user, notification toggles
- **Dark/light UI** — compact icon sidebar with flyout menus, tabbed dashboard, dependency map, execution timeline, cron builder, script editor, and in-app docs
- **Secure by default** — API key authentication with 4 roles (admin, operator, viewer, agent), optional OIDC/OAuth2 SSO (Okta, Azure AD, Google, Keycloak), rate limiting on all endpoints, and audit logging for sensitive operations. Bootstrap keys auto-generated on first startup.
- **Docker ready** — pre-built images on [GitHub Container Registry](https://ghcr.io/mikemiles-dev/kronforce), separate compose files for controller and agent

## Quick Start

### Controller

```bash
cargo run --bin kronforce
```

Opens on `http://localhost:8080` with web dashboard, REST API, scheduler, and SQLite database. On first startup, bootstrap API keys are printed to the console.

### Standard Agent

```bash
KRONFORCE_CONTROLLER_URL=http://localhost:8080 \
KRONFORCE_AGENT_NAME=agent-1 \
KRONFORCE_AGENT_TAGS=linux,dev \
KRONFORCE_AGENT_ADDRESS=127.0.0.1 \
cargo run --bin kronforce-agent
```

### Custom Agent

```bash
pip install requests
python3 examples/custom_agent.py
```

Then configure task types in the dashboard (Agents page → click the agent card).

See [Custom Agents documentation](docs/CUSTOM_AGENTS.md) for the full protocol.

### gRPC Agent

```bash
brew install grpcurl
KRONFORCE_AGENT_KEY=kf_your_agent_key python3 examples/grpc_agent.py
```

Calls gRPC services via [grpcurl](https://github.com/fullstorydev/grpcurl) with reflection or proto file support. Configure task types in the dashboard.

## Configuration

### Controller

| Variable | Default | Description |
|---|---|---|
| `KRONFORCE_DB` | `kronforce.db` | SQLite database path |
| `KRONFORCE_BIND` | `0.0.0.0:8080` | Listen address |
| `KRONFORCE_TICK_SECS` | `1` | Scheduler tick interval |
| `KRONFORCE_CALLBACK_URL` | `http://{BIND}` | URL agents use to report results back |
| `KRONFORCE_HEARTBEAT_TIMEOUT_SECS` | `30` | Seconds before marking an agent offline |
| `KRONFORCE_SCRIPTS_DIR` | `./scripts` | Directory for Rhai script files |
| `KRONFORCE_RATE_LIMIT_ENABLED` | `true` | Enable/disable API rate limiting |
| `KRONFORCE_RATE_LIMIT_PUBLIC` | `30` | Max requests/min for public endpoints (per IP) |
| `KRONFORCE_RATE_LIMIT_AUTHENTICATED` | `120` | Max requests/min for authenticated endpoints (per API key) |
| `KRONFORCE_RATE_LIMIT_AGENT` | `600` | Max requests/min for agent endpoints (per API key) |
| `KRONFORCE_MCP_ENABLED` | `true` | Enable/disable the MCP server endpoint |
| `KRONFORCE_OIDC_ISSUER` | (none) | OIDC issuer URL (enables SSO when set with CLIENT_ID) |
| `KRONFORCE_OIDC_CLIENT_ID` | (none) | OAuth2 client ID |
| `KRONFORCE_OIDC_CLIENT_SECRET` | (none) | OAuth2 client secret |
| `KRONFORCE_OIDC_REDIRECT_URI` | `{CALLBACK_URL}/api/auth/oidc/callback` | OAuth2 callback URL |
| `KRONFORCE_OIDC_SCOPES` | `openid email profile` | OIDC scopes to request |
| `KRONFORCE_OIDC_ROLE_CLAIM` | `groups` | Claim path for role mapping (dot-notation) |
| `KRONFORCE_OIDC_ADMIN_VALUES` | (none) | Comma-separated claim values that map to admin role |
| `KRONFORCE_OIDC_OPERATOR_VALUES` | (none) | Comma-separated claim values that map to operator role |
| `KRONFORCE_OIDC_DEFAULT_ROLE` | `viewer` | Fallback role when no claim matches |
| `KRONFORCE_OIDC_SESSION_TTL_SECS` | `86400` | SSO session lifetime in seconds (default 24h) |

### Agent

| Variable | Default | Description |
|---|---|---|
| `KRONFORCE_CONTROLLER_URL` | `http://localhost:8080` | Controller to register with |
| `KRONFORCE_AGENT_NAME` | hostname | Agent display name |
| `KRONFORCE_AGENT_TAGS` | (none) | Comma-separated tags |
| `KRONFORCE_AGENT_ADDRESS` | hostname | Address the controller uses to reach this agent |
| `KRONFORCE_AGENT_BIND` | `0.0.0.0:8081` | Agent listen address |
| `KRONFORCE_AGENT_KEY` | (none) | API key with `agent` role for authenticating with the controller |

## Features

- **Task types** — Shell, HTTP, SQL, FTP/SFTP, Rhai Script, MCP (AI tools), and Custom agent types
- **Custom agents** — pull-based agents in any language with UI-managed task type definitions
- **Execution modes** — Standard or Custom Agent mode in job creation
- **Execution retry** — automatic retry on failure/timeout with configurable backoff
- **Job groups** — organize jobs into named groups with dedicated Groups page
- **Output intelligence** — extract values from output (regex/jsonpath), trigger events on patterns, diff output across runs, write extracted values to global variables
- **Secret variables** — variables marked as secret are masked in the API and UI but substituted into tasks at runtime
- **Global variables** — shared key-value store with `{{VAR_NAME}}` substitution in all task fields, updatable via UI, API, or output extraction write-back
- **Cron scheduling** — 6-field second-precision cron with visual builder
- **Priority scheduling** — higher priority jobs fire first when multiple are due simultaneously
- **Approval workflows** — jobs can require approval before execution; pending executions must be approved via API
- **SLA deadlines** — set a completion deadline (HH:MM UTC) per job with early warning; fires `sla.warning` and `sla.breach` events with notifications
- **Job version history** — every create/update snapshots the full job definition for audit trail and rollback
- **Event triggers** — fire jobs reactively on system events or output pattern matches
- **Dependency DAG** — job dependencies with time windows and visual map
- **Rhai scripting** — embedded scripting with HTTP, shell, TCP/UDP, and more
- **Notifications** — email (SMTP), SMS (webhook), Slack, Microsoft Teams, PagerDuty on job failures, successes, and agent outages
- **Prometheus metrics** — `/metrics` endpoint for Grafana/Prometheus scraping (execution counts, DB health, job/agent totals)
- **HA/disaster recovery** — Litestream replication to S3 with automatic restore on failover, graceful shutdown with WAL checkpoint
- **Connection pooling** — r2d2 SQLite connection pool for concurrent database access
- **Team isolation** — API keys can be scoped to specific job groups for team-level access control
- **Dark/Light mode**, auto-refresh, pagination, audit log, API key + OIDC/SSO auth, rate limiting

## Dashboard Pages

| Page | Sidebar | Description |
|---|---|---|
| Dashboard | Dashboard | Stats, timeline, charts (tabbed: Overview, Activity, Infrastructure) |
| Jobs | Jobs | Job list with search, filters, bulk actions, sortable columns |
| Groups | Groups | Job group cards with rename, delete, and job counts |
| Executions | Executions | All executions with status filters, UTC timestamps, output viewer |
| Events | Events | Activity feed with severity filtering |
| Map | Map | Visual dependency graph with group badges and filter |
| Scripts | Tools ▸ Scripts | Rhai script editor with syntax highlighting |
| Variables | Tools ▸ Variables | Global key-value variable management (with secret variable support) |
| Docs | Docs | In-app documentation for all features |
| Agents | Manage ▸ Agents | Agent cards with custom agent task type editor |
| Settings | Manage ▸ Settings | Theme, API keys (with group scoping), data retention, notifications (email/SMS/webhook) |

## Documentation

- [Deployment](docs/DEPLOYMENT.md) — Docker Compose setup, configuration, authentication, scaling, troubleshooting
- [Architecture](docs/ARCHITECTURE.md) — system design, components, execution flow, database schema
- [Code Architecture](docs/CODE_ARCHITECTURE.md) — source tree, data flows, design patterns, module guide
- [API Reference](docs/API.md) — all endpoints with examples, schedule types, event triggers, output rules
- [Triggers & Workflows](docs/TRIGGERS_AND_WORKFLOWS.md) — event-driven automation, output intelligence, dependency chains, workflow patterns
- [Custom Agents](docs/CUSTOM_AGENTS.md) — protocol, task types, queue behavior, Python example
- [Testing](docs/TESTING.md) — test suite, seed data, MCP testing, agent testing, Docker testing

The dashboard also includes a **Docs** page with the same content accessible from the sidebar.

## Authentication

**API keys** with roles: `admin`, `operator`, `viewer`, `agent`. Bootstrap admin and agent keys printed on first startup. Agents authenticate with keys that have the `agent` role. Keys can be scoped to specific job groups for team-level isolation.

**OIDC/SSO** (optional): Set `KRONFORCE_OIDC_ISSUER` and `KRONFORCE_OIDC_CLIENT_ID` to enable "Sign in with SSO" on the login screen. Users authenticate via your IdP (Okta, Azure AD, Google, Keycloak) and are mapped to Kronforce roles based on claim values. API keys continue to work alongside SSO for agents and automation.

## MCP Server

Kronforce acts as an MCP (Model Context Protocol) server, letting AI assistants discover and manage jobs. The MCP endpoint is enabled by default at `POST /mcp`.

**Connect an MCP client:**
```bash
# Discover available tools
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"my-client","version":"1.0"}}}'
```

**Available tools:** `list_jobs`, `get_job`, `create_job`, `trigger_job`, `list_executions`, `get_execution`, `list_agents`, `list_groups`, `list_events`, `get_system_stats`

Tools are filtered by API key role — viewers get read-only tools, operators can create and trigger jobs. Disable with `KRONFORCE_MCP_ENABLED=false`.

## Development

```bash
cargo build                                          # Build
cargo test                                           # Test
RUST_LOG=kronforce=debug cargo run --bin kronforce    # Debug logging
```
