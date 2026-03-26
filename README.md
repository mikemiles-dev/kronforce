# Kronforce

A workload automation and job scheduling engine built in Rust. Features a controller/agent architecture for distributed job execution.

![Kronforce Dashboard](screenshot.png)

## Quick Start

### Controller

```bash
cargo run --bin kronforce
```

Opens on `http://localhost:8080` with web dashboard, REST API, scheduler, and SQLite database.

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

### Agent

| Variable | Default | Description |
|---|---|---|
| `KRONFORCE_CONTROLLER_URL` | `http://localhost:8080` | Controller to register with |
| `KRONFORCE_AGENT_NAME` | hostname | Agent display name |
| `KRONFORCE_AGENT_TAGS` | (none) | Comma-separated tags |
| `KRONFORCE_AGENT_ADDRESS` | hostname | Address the controller uses to reach this agent |
| `KRONFORCE_AGENT_BIND` | `0.0.0.0:8081` | Agent listen address |

## Features

- **Task types** — Shell, HTTP, SQL, FTP/SFTP, Rhai Script, and Custom agent types
- **Custom agents** — pull-based agents in any language with UI-managed task type definitions
- **Execution modes** — Standard or Custom Agent mode in job creation
- **Output intelligence** — extract values from output (regex/jsonpath), trigger events on patterns, diff output across runs
- **Cron scheduling** — 6-field second-precision cron with visual builder
- **Event triggers** — fire jobs reactively on system events or output pattern matches
- **Dependency DAG** — job dependencies with time windows and visual map
- **Rhai scripting** — embedded scripting with HTTP, shell, TCP/UDP, and more
- **Dark/Light mode**, auto-refresh, pagination, audit trail, API key auth

## Dashboard Pages

| Page | Description |
|---|---|
| Dashboard | Stats, execution timeline, recent activity |
| Jobs | Job list with search, filters, bulk actions, sortable columns |
| Executions | All executions with status filters and output viewer |
| Map | Visual dependency graph |
| Agents | Agent cards with custom agent task type editor |
| Scripts | Rhai script editor with syntax highlighting |
| Events | Activity feed |
| Docs | In-app documentation for all features |
| Settings | Theme, API keys, data retention |

## Documentation

- [Architecture](docs/ARCHITECTURE.md) — system design, components, execution flow, database schema
- [API Reference](docs/API.md) — all endpoints with examples, schedule types, event triggers, output rules
- [Triggers & Workflows](docs/TRIGGERS_AND_WORKFLOWS.md) — event-driven automation, output intelligence, dependency chains, workflow patterns
- [Custom Agents](docs/CUSTOM_AGENTS.md) — protocol, task types, queue behavior, Python example

The dashboard also includes a **Docs** page with the same content accessible from the sidebar.

## Authentication

API keys with roles: `admin`, `operator`, `viewer`. Bootstrap key printed on first startup. Agent endpoints require no key.

## Development

```bash
cargo build                                          # Build
cargo test                                           # Test
RUST_LOG=kronforce=debug cargo run --bin kronforce    # Debug logging
```
