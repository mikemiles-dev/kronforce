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
| Settings | `/#/settings` | Theme toggle, API key management, sign out |
| Job Detail | `/#/jobs/{id}` | Job info, execution history, output viewer, mini dependency map |

All URLs are shareable — opening a link goes directly to that view.

### Features

- **Task types** — Shell, HTTP, SQL, and FTP/SFTP job types with type-specific configuration forms
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
| `script` | Rhai scripting engine with built-in APIs | `code` |

All task types capture output and errors. HTTP returns the response body as output and the status code as exit code. SQL returns query results as output. FTP returns the transfer log.

### Custom Scripts (Rhai)

The `script` task type runs custom logic written in [Rhai](https://rhai.rs), a lightweight scripting language embedded in the Rust binary. Scripts are stored as `.rhai` files in the scripts directory (default `./scripts/`).

#### Managing Scripts

Scripts can be managed via the **Scripts** page in the dashboard (with syntax highlighting editor), the API, or by placing `.rhai` files directly in the scripts directory (auto-discovered on startup).

```bash
# Create/update a script via API
curl -X PUT http://localhost:8080/api/scripts/health-check \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{"code": "let resp = http_get(\"https://api.example.com/health\");\nif resp.status != 200 {\n    fail(\"down\");\n}\nprint(\"OK\");"}'

# List all scripts
curl -H "Authorization: Bearer kf_your_key" http://localhost:8080/api/scripts

# Or just drop a file in the scripts directory
echo 'print("hello from file");' > ./scripts/hello.rhai
```

#### Using Scripts in Jobs

Jobs reference scripts by name — select from a dropdown in the UI:

```bash
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "health-monitor",
    "task": {"type": "script", "script_name": "health-check"},
    "schedule": {"type": "cron", "value": "0 */5 * * * *"}
  }'
```

#### Available Functions

| Function | Returns | Description |
|---|---|---|
| `print(msg)` | — | Appends to job output |
| `http_get(url)` | `#{status, body}` | Makes an HTTP GET request |
| `http_post(url, body)` | `#{status, body}` | Makes an HTTP POST request |
| `shell_exec(cmd)` | `#{exit_code, stdout, stderr}` | Runs a shell command |
| `env_var(name)` | `string` | Reads an environment variable |
| `sleep_ms(ms)` | — | Sleeps for N milliseconds |
| `fail(msg)` | — | Marks the execution as failed |

#### Script Examples

**Health check with Slack notification:**
```javascript
let resp = http_get("https://api.example.com/health");
if resp.status != 200 {
    http_post("https://hooks.slack.com/services/T00/B00/xxx",
        `{"text": "API is DOWN! Status: ${resp.status}"}`);
    fail("Health check failed");
}
print("API is healthy");
```

**Run a command and parse the output:**
```javascript
let result = shell_exec("df -h / | tail -1 | awk '{print $5}'");
let usage = result.stdout;
print("Disk usage: " + usage);

if parse_int(usage.replace("%", "")) > 90 {
    fail("Disk usage critical: " + usage);
}
```

**Chain multiple API calls:**
```javascript
let data = http_get("https://api.example.com/orders/today");
let orders = parse_json(data.body);
print("Orders today: " + orders.len());

let report = http_post("https://reports.internal/generate", data.body);
if report.status != 200 {
    fail("Report generation failed");
}
print("Report generated: " + report.body);
```

**Conditional deployment:**
```javascript
let tests = shell_exec("cd /app && cargo test 2>&1");
if tests.exit_code != 0 {
    print("Tests failed:");
    print(tests.stderr);
    fail("Cannot deploy: tests failed");
}

let deploy = shell_exec("/opt/deploy/prod.sh");
print(deploy.stdout);
if deploy.exit_code != 0 {
    fail("Deployment failed");
}
print("Deployment successful");
```

#### Sandboxing

Scripts run with these limits:
- **1,000,000 operations** max (prevents infinite loops)
- **256KB string size** max
- **Timeout** enforced by the job's `timeout_secs` setting (default 60s)
- No direct file system access (use `shell_exec` for controlled file operations)
- No network access except through `http_get`/`http_post`

## Cron Expressions

6-field cron with second-level precision: `sec min hour dom month dow`

| Expression | Description |
|---|---|
| `* * * * * *` | Every second |
| `0 * * * * *` | Every minute |
| `0 0 * * * *` | Every hour |
| `0 0 9 * * *` | Daily at 9:00 AM |
| `0 0 9 * * 1-5` | Weekdays at 9:00 AM |
| `0 */5 * * * *` | Every 5 minutes |
| `*/30 * * * * *` | Every 30 seconds |

Supports: `*`, ranges (`1-5`), lists (`1,3,5`), steps (`*/5`, `1-30/5`).

## Dependencies

Jobs can declare dependencies with optional time windows. A job only runs when all dependencies have a successful execution within the specified window. Circular dependencies are rejected at creation time.

```bash
# Job that depends on "extract" completing successfully within the last 2 hours
curl -X POST http://localhost:8080/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "transform",
    "task": {"type": "shell", "command": "transform.sh"},
    "schedule": {"type": "cron", "value": "0 0 3 * * *"},
    "depends_on": [
      {"job_id": "<extract-job-id>", "within_secs": 7200}
    ]
  }'

# Job with a dependency but no time window (any past success counts)
curl -X POST http://localhost:8080/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "load",
    "task": {"type": "shell", "command": "load.sh"},
    "schedule": {"type": "cron", "value": "0 0 4 * * *"},
    "depends_on": [
      {"job_id": "<transform-job-id>", "within_secs": null}
    ]
  }'
```

## Job States

| State | Description |
|---|---|
| `enabled` | Scheduled and will run automatically |
| `paused` | Won't be scheduled until resumed |
| `disabled` | Permanently disabled |
| `unscheduled` | No future schedule (one-shot that has fired), can still be triggered manually |

## Execution Statuses

| Status | Description |
|---|---|
| `pending` | Dispatched to agent, waiting to start |
| `running` | Currently executing |
| `succeeded` | Completed with exit code 0 |
| `failed` | Completed with non-zero exit code |
| `timed_out` | Killed after exceeding `timeout_secs` |
| `cancelled` | Cancelled via API |
| `skipped` | Skipped due to failed dependency |

## Output Capture

Stdout and stderr are captured and stored in the database. Each stream is capped at **256KB** — output beyond that is truncated from the front (keeps the tail). Truncated output is prefixed with `[...truncated N bytes...]` and flagged in the API response.

## Authentication

Kronforce uses API keys for authentication. All `/api/*` endpoints (except health and agent callbacks) require a valid key.

### Bootstrap Key

On first startup with a fresh database, an admin API key is automatically generated and printed to the console:

```
INFO kronforce: =============================================================
INFO kronforce:   No API keys found. Bootstrap admin key created:
INFO kronforce:   kf_rlpzh75R60xG9w8QtLhxyNAqvA-q_K7NKB3mFT9uH1g
INFO kronforce:   Save this key — it will not be shown again.
INFO kronforce: =============================================================
```

Copy this key immediately — it's only shown once.

### Using API Keys

Pass the key via the `Authorization` header:

```bash
curl -H "Authorization: Bearer kf_your_key_here" http://localhost:8080/api/jobs
```

In the web dashboard, you'll see a login screen where you paste your key. It's stored in your browser's localStorage.

### Roles

| Role | Permissions |
|---|---|
| `admin` | Full access: manage jobs, agents, API keys |
| `operator` | Create/edit/trigger/delete jobs, view agents. Cannot manage keys |
| `viewer` | Read-only: view jobs, executions, agents, events |

### Managing Keys

Admins can create and revoke keys via the API or the Settings page in the dashboard.

```bash
# Create a new key
curl -X POST http://localhost:8080/api/keys \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "CI pipeline", "role": "operator"}'

# List all keys
curl -H "Authorization: Bearer kf_admin_key" http://localhost:8080/api/keys

# Revoke a key
curl -X DELETE -H "Authorization: Bearer kf_admin_key" http://localhost:8080/api/keys/{id}
```

If no API keys exist in the database, authentication is disabled and all endpoints are open. This allows initial setup without needing a key first.

## Run-As User

Jobs can specify a system user to execute as. This uses `sudo -n -u <user>` (non-interactive) on the controller or agent.

```bash
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "db-backup",
    "task": {"type": "shell", "command": "pg_dump mydb > /backups/mydb.sql"},
    "run_as": "postgres",
    "schedule": {"type": "cron", "value": "0 0 2 * * *"}
  }'
```

The controller/agent process must have passwordless sudo access for the target user. Configure via `/etc/sudoers`:

```
kronforce ALL=(postgres) NOPASSWD: ALL
```

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
