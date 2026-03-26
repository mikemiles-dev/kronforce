# Kronforce

A workload automation and job scheduling engine built in Rust. Features a controller/agent architecture for distributed job execution.

![Kronforce Dashboard](screenshot.png)

## Quick Start

### Controller

```bash
cargo run --bin kronforce
```

The controller starts on `0.0.0.0:8080` with a web dashboard, REST API, scheduler, and SQLite database. Open `http://localhost:8080` in your browser.

### Agent

In another terminal:

```bash
KRONFORCE_CONTROLLER_URL=http://localhost:8080 \
KRONFORCE_AGENT_NAME=agent-1 \
KRONFORCE_AGENT_TAGS=linux,dev \
KRONFORCE_AGENT_ADDRESS=127.0.0.1 \
cargo run --bin kronforce-agent
```

The agent registers with the controller, sends heartbeats, and executes jobs dispatched to it. Jobs with no target still run locally on the controller.

### Configuration

#### Controller

| Variable | Default | Description |
|---|---|---|
| `KRONFORCE_DB` | `kronforce.db` | SQLite database path |
| `KRONFORCE_BIND` | `0.0.0.0:8080` | Listen address |
| `KRONFORCE_TICK_SECS` | `1` | Scheduler tick interval |
| `KRONFORCE_CALLBACK_URL` | `http://{BIND}` | URL agents use to report results back |
| `KRONFORCE_HEARTBEAT_TIMEOUT_SECS` | `30` | Seconds before marking an agent offline |
| `KRONFORCE_SCRIPTS_DIR` | `./scripts` | Directory for Rhai script files |

#### Agent

| Variable | Default | Description |
|---|---|---|
| `KRONFORCE_CONTROLLER_URL` | `http://localhost:8080` | Controller to register with |
| `KRONFORCE_AGENT_NAME` | hostname | Agent display name |
| `KRONFORCE_AGENT_TAGS` | (none) | Comma-separated tags for job targeting |
| `KRONFORCE_AGENT_ADDRESS` | hostname | Address the controller uses to reach this agent |
| `KRONFORCE_AGENT_BIND` | `0.0.0.0:8081` | Agent listen address |
| `KRONFORCE_HEARTBEAT_SECS` | `10` | Heartbeat interval |

## Architecture

```
┌──────────────────────────────────────────────────────────────────┐
│                        CONTROLLER (:8080)                        │
│                                                                  │
│  ┌──────────┐    mpsc     ┌───────────┐            ┌──────────┐ │
│  │  REST    │───────────▶│ Scheduler │───────────▶│ Executor │ │
│  │  API     │            │  (1s tick) │            │          │ │
│  │  + Web   │            └───────────┘            └────┬─────┘ │
│  └──────────┘                                          │       │
│       │                                          ┌─────┴─────┐ │
│       │              ┌─────────┐                 │  Local OR  │ │
│       └─────────────▶│ SQLite  │                 │  Dispatch  │ │
│                      │  (WAL)  │                 └─────┬─────┘ │
│                      └─────────┘                       │       │
└────────────────────────────────────────────────────────┼───────┘
                                                         │
                              HTTP POST /execute         │
                    ┌────────────────────────────────────┘
                    │
                    ▼
┌──────────────────────────────────────────┐
│            AGENT (:8081)                 │
│                                          │
│  ┌──────────┐    ┌───────────────────┐   │
│  │ /execute │───▶│ sh -c "command"   │   │
│  │ /cancel  │    │ stdout/stderr cap │   │
│  │ /health  │    └───────┬───────────┘   │
│  └──────────┘            │               │
│                          │ POST result   │
│                          └──────────────▶│──▶ Controller callback
└──────────────────────────────────────────┘
```

**Flow:**
1. Controller scheduler detects a due job
2. If the job has a target (agent or tag), the executor dispatches it via HTTP to the agent
3. If no target, the executor runs it locally (backward compatible)
4. Agent executes the command, captures stdout/stderr (256KB cap per stream)
5. Agent POSTs the result back to the controller's callback endpoint
6. Controller updates the execution record in SQLite

## Web Dashboard

The dashboard is embedded in the controller binary (no separate build step). Navigate to `http://localhost:8080`.

### Pages

| Page | URL | Description |
|---|---|---|
| Jobs | `/#/jobs` | Job list with search, filters, bulk actions, sortable columns |
| Map | `/#/map` | Visual dependency graph showing all jobs and their relationships |
| Agents | `/#/agents` | Registered agents with status, tags, heartbeat info |
| Scripts | `/#/scripts` | Manage Rhai scripts with syntax-highlighted editor |
| Events | `/#/events` | Activity feed — job triggers, completions, agent status changes |
| Docs | `/#/docs` | Custom agents, scripting, task types, API reference, cron docs |
| Settings | `/#/settings` | Theme toggle, API key management, sign out |
| Job Detail | `/#/jobs/{id}` | Job info, execution history, output viewer, mini dependency map |

All URLs are shareable — opening a link goes directly to that view.

### Features

- **Task types** — Shell, HTTP, SQL, FTP/SFTP, Script, and Custom agent task types with type-specific configuration forms
- **Custom agents** — pull-based agents in any language with UI-managed task type definitions and dynamic form rendering
- **Execution modes** — Local, Standard Agent, or Custom Agent mode selector in job creation
- **Search and filter** — search jobs by name/task, filter by state; search agents by name/hostname/tag, filter by status
- **Bulk actions** — select multiple jobs to schedule or delete at once
- **Sortable columns** — click any column header to sort ascending/descending
- **Auto-refresh** — configurable polling interval (2s–60s) with countdown, toggle on/off
- **Dark/Light mode** — persisted in localStorage, or follow system preference
- **Pagination** — jobs, executions, and events are paginated
- **Output viewer** — execution output with Text/JSON/HTML view tabs (HTML rendered in sandboxed iframe)
- **Mini dependency map** — job detail page shows a focused DAG of related jobs
- **Cron builder** — visual schedule builder with interval/unit picker, day-of-week buttons, and live preview
- **Event-triggered jobs** — fire jobs reactively when system events occur (failures, agent changes, etc.)
- **Audit trail** — all user actions tracked with API key identity in the events feed, job edits show before/after diffs
- **Dependency status** — "waiting" indicator shows which dependencies are blocking a job, click to see details
- **Execution timeline** — Kibana-style bar charts showing execution counts over time (dashboard: 15 min, job detail: 1 hour)
- **Task snapshots** — each execution captures the exact task config at the time it ran

## API

### Jobs

```bash
# Create a shell job (one-shot)
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "migration",
    "task": {"type": "shell", "command": "./migrate.sh"},
    "schedule": {"type": "one_shot", "value": "2026-04-01T00:00:00Z"}
  }'

# Create an HTTP job (cron)
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "health-ping",
    "task": {"type": "http", "method": "get", "url": "https://api.example.com/health", "expect_status": 200},
    "schedule": {"type": "cron", "value": "0 * * * * *"}
  }'

# Create a SQL job
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "report-query",
    "task": {"type": "sql", "driver": "postgres", "connection_string": "postgresql://user:pass@host/db", "query": "SELECT count(*) FROM orders WHERE date = CURRENT_DATE"},
    "schedule": {"type": "cron", "value": "0 30 8 * * 1-5"}
  }'

# Create an FTP job
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "upload-report",
    "task": {"type": "ftp", "protocol": "sftp", "host": "ftp.example.com", "username": "user", "password": "pass", "direction": "upload", "local_path": "/data/report.csv", "remote_path": "/uploads/report.csv"},
    "schedule": {"type": "on_demand"}
  }'

# Create a shell job targeted at agents
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "deploy",
    "task": {"type": "shell", "command": "/opt/scripts/deploy.sh"},
    "schedule": {"type": "on_demand"},
    "timeout_secs": 300,
    "target": {"type": "tagged", "tag": "linux"}
  }'

# List all jobs (paginated, searchable, filterable)
curl http://localhost:8080/api/jobs
curl "http://localhost:8080/api/jobs?status=enabled&search=deploy&page=1&per_page=20"

# Get / Update / Delete
curl http://localhost:8080/api/jobs/{id}
curl -X PUT http://localhost:8080/api/jobs/{id} -H "Authorization: Bearer kf_your_key" -H 'Content-Type: application/json' -d '{"task": {"type": "shell", "command": "echo updated"}}'
curl -X DELETE http://localhost:8080/api/jobs/{id}
```

### Schedule Types

| Type | JSON | Description |
|---|---|---|
| One-shot | `{"type": "one_shot", "value": "2026-04-01T00:00:00Z"}` | Fires once at the specified time, then becomes unscheduled |
| Cron | `{"type": "cron", "value": "0 * * * * *"}` | Fires on a recurring cron schedule |
| On-demand | `{"type": "on_demand"}` | Never fires automatically, triggered via API/UI only |
| Event | `{"type": "event", "value": {...}}` | Fires when a matching system event occurs |

### Event-Triggered Jobs

Jobs can be triggered reactively when system events occur, such as executions completing, agents registering, or other jobs being created/deleted.

```bash
# Run cleanup when any execution fails
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "failure-cleanup",
    "task": {"type": "shell", "command": "/opt/scripts/cleanup.sh"},
    "schedule": {"type": "event", "value": {
      "kind_pattern": "execution.completed",
      "severity": "error"
    }}
  }'

# Provision a new agent when it registers
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "provision-agent",
    "task": {"type": "shell", "command": "/opt/scripts/provision.sh"},
    "schedule": {"type": "event", "value": {
      "kind_pattern": "agent.registered"
    }},
    "target": {"type": "any"}
  }'

# Run security audit when API keys change
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "security-audit",
    "task": {"type": "shell", "command": "/opt/security/audit.sh"},
    "schedule": {"type": "event", "value": {
      "kind_pattern": "key.*"
    }}
  }'
```

**Event trigger config:**

| Field | Description |
|---|---|
| `kind_pattern` | Event kind to match. Supports exact (`agent.registered`), wildcard (`job.*`), or all (`*`) |
| `severity` | Optional. Only trigger on events with this severity: `success`, `error`, `warning`, `info` |
| `job_name_filter` | Optional. Only trigger on events whose message contains this text |

**Available event kinds:**

| Kind | When it fires |
|---|---|
| `job.created` | A job is created |
| `job.updated` | A job is edited |
| `job.deleted` | A job is deleted |
| `job.triggered` | A job is manually triggered |
| `execution.completed` | A job execution finishes (success or failure) |
| `agent.registered` | An agent registers with the controller |
| `agent.offline` | An agent's heartbeat times out |
| `agent.unpaired` | An agent is removed |
| `key.created` | An API key is created |
| `key.revoked` | An API key is revoked |

### Job Targeting

| Target | JSON | Description |
|---|---|---|
| Local | `null` or `{"type": "local"}` | Runs on the controller (default) |
| Specific agent | `{"type": "agent", "agent_id": "uuid"}` | Runs on a specific agent |
| Any agent | `{"type": "any"}` | Runs on a random online agent |
| All agents | `{"type": "all"}` | Runs on every online agent simultaneously |
| Tagged | `{"type": "tagged", "tag": "linux"}` | Runs on a random online agent with the tag |

### Execution

```bash
# Trigger a job now
curl -X POST http://localhost:8080/api/jobs/{id}/trigger

# View execution history (paginated)
curl "http://localhost:8080/api/jobs/{id}/executions?page=1&per_page=20"

# Get execution details (includes stdout/stderr)
curl http://localhost:8080/api/executions/{id}

# Cancel a running execution
curl -X POST http://localhost:8080/api/executions/{id}/cancel
```

### Agents

```bash
# List registered agents
curl http://localhost:8080/api/agents

# Get agent details
curl http://localhost:8080/api/agents/{id}

# Deregister an agent
curl -X DELETE http://localhost:8080/api/agents/{id}

# Health check
curl http://localhost:8080/api/health
```

### Events

```bash
# List recent events (paginated)
curl "http://localhost:8080/api/events?page=1&per_page=50"
```

Events are logged for: job created/deleted/triggered, execution completed (success/failure), agent registered/offline.

## Task Types

| Type | Execution | Config Fields |
|---|---|---|
| `shell` | Runs `sh -c` (or `sudo -n -u` with `run_as`) | `command` |
| `http` | In-process HTTP request via reqwest | `method`, `url`, `headers`, `body`, `expect_status` |
| `sql` | Shells out to `psql`/`mysql`/`sqlite3` | `driver`, `connection_string`, `query` |
| `ftp` | Uses `curl` for FTP/FTPS/SFTP transfers | `protocol`, `host`, `port`, `username`, `password`, `direction`, `remote_path`, `local_path` |
| `script` | Rhai scripting engine with built-in APIs | `script_name` |
| `custom` | Dispatched to a custom agent | `agent_task_type`, `data` (fields defined per agent in UI) |

See the **Docs** page in the dashboard for detailed documentation on each task type, scripting, and the custom agent protocol.

## Custom Agents

Custom agents use a pull-based model — build agents in any language that poll for work, execute tasks, and report results back. Task type definitions are managed in the dashboard UI; agent code handles the implementation.

Quick start:
1. Run `python3 examples/custom_agent.py` to start a sample agent
2. In the dashboard, go to **Agents** and click the custom agent card to configure task types
3. Create a job using **Custom Agent** execution mode

See the **Docs** page in the dashboard for the full custom agent protocol, task type schema, and Python example.

## Scripting (Rhai)

The `script` task type runs custom logic in [Rhai](https://rhai.rs), embedded in the binary. Manage scripts via the **Scripts** page or drop `.rhai` files in the scripts directory.

See the **Docs** page in the dashboard for available functions, examples, and sandboxing details.

## More Documentation

The dashboard includes a **Docs** page (accessible from the sidebar) with comprehensive documentation:
- **Custom Agents** — setup, protocol, task type definitions, queue behavior
- **Scripting** — Rhai functions, examples, sandboxing
- **Task Types** — all built-in and custom types with JSON examples
- **API Reference** — complete endpoint listing with auth requirements
- **Cron Expressions** — format, examples, event triggers

## Cron Expressions

6-field cron with second-level precision: `sec min hour dom month dow`

| Expression | Description |
|---|---|
| `0 * * * * *` | Every minute |
| `0 0 9 * * *` | Daily at 9:00 AM |
| `0 0 9 * * 1-5` | Weekdays at 9:00 AM |
| `0 */5 * * * *` | Every 5 minutes |

## Authentication

API keys required for dashboard endpoints. On first startup, a bootstrap admin key is printed to the console. Agent endpoints (register, poll, callback) require no key.

Roles: `admin` (full access), `operator` (jobs + agents), `viewer` (read-only).

## Development

```bash
# Build both binaries
cargo build

# Run tests
cargo test

# Run controller with debug logging
RUST_LOG=kronforce=debug cargo run --bin kronforce

# Run agent with debug logging
RUST_LOG=kronforce_agent=debug cargo run --bin kronforce-agent
```
