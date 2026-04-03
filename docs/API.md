# API Reference

All `/api/*` endpoints (except health, agent registration, polling, and callbacks) require an API key via `Authorization: Bearer kf_...` header.

## Jobs

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

## Schedule Types

| Type | JSON | Description |
|---|---|---|
| One-shot | `{"type": "one_shot", "value": "2026-04-01T00:00:00Z"}` | Fires once at the specified time |
| Cron | `{"type": "cron", "value": "0 * * * * *"}` | Recurring cron schedule (6-field, second precision) |
| On-demand | `{"type": "on_demand"}` | Triggered via API/UI only |
| Event | `{"type": "event", "value": {...}}` | Fires when a matching system event occurs |

## Event-Triggered Jobs

```bash
# Run cleanup when any execution fails
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "failure-cleanup",
    "task": {"type": "shell", "command": "/opt/scripts/cleanup.sh"},
    "schedule": {"type": "event", "value": {"kind_pattern": "execution.completed", "severity": "error"}}
  }'
```

**Event kinds:** `job.created`, `job.updated`, `job.deleted`, `job.triggered`, `execution.completed`, `output.matched`, `agent.registered`, `agent.offline`, `agent.unpaired`, `key.created`, `key.revoked`

### Output Rules (on job creation/update)

```json
{
    "output_rules": {
        "extractions": [
            {"name": "duration", "pattern": "took (\\d+)ms", "type": "regex"},
            {"name": "count", "pattern": "$.results.count", "type": "jsonpath", "write_to_variable": "LAST_COUNT"}
        ],
        "triggers": [
            {"pattern": "ERROR|FATAL", "severity": "error"},
            {"pattern": "WARNING", "severity": "warning"}
        ]
    }
}
```

Extractions run against stdout after each execution. Extracted values appear in `GET /api/executions/{id}` as the `extracted` field. Add `write_to_variable` to an extraction rule to upsert the value into a global variable. Triggers emit `output.matched` events. See [Triggers & Workflows](TRIGGERS_AND_WORKFLOWS.md).

### Variables

Global key-value variables that can be referenced in task fields using `{{VAR_NAME}}` syntax. Variables are substituted before execution.

| Method | Endpoint | Description |
|---|---|---|
| `GET` | `/api/variables` | List all variables |
| `GET` | `/api/variables/{name}` | Get a variable |
| `POST` | `/api/variables` | Create a variable (`{"name": "API_HOST", "value": "https://api.example.com"}`) |
| `PUT` | `/api/variables/{name}` | Update a variable (`{"value": "new_value"}`) |
| `DELETE` | `/api/variables/{name}` | Delete a variable |

Variable names must match `[A-Za-z0-9_]+`.

### Output Assertions

Fail the execution if expected patterns are NOT found in stdout (only checked on successful runs):

```json
{
    "output_rules": {
        "assertions": [
            {"pattern": "OK", "message": "Health check did not return OK"},
            {"pattern": "records processed", "message": "ETL did not process any records"}
        ]
    }
}
```

### Job Notifications

Send email/SMS alerts based on execution results:

```json
{
    "notifications": {
        "on_failure": true,
        "on_success": false,
        "on_assertion_failure": true,
        "recipients": {
            "emails": ["ops@example.com"],
            "phones": ["+1234567890"]
        }
    }
}
```

If `recipients` is omitted, falls back to the global notification recipients configured in Settings.

### Additional Task Type Examples

```bash
# File push — deploy a config file to an agent
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "deploy-config",
    "task": {"type": "file_push", "filename": "app.conf", "destination": "/opt/app/app.conf", "content_base64": "dGVzdA==", "overwrite": true},
    "schedule": {"type": "on_demand"},
    "target": {"type": "agent", "agent_id": "uuid"}
  }'

# Kafka — publish a message
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "kafka-publish",
    "task": {"type": "kafka", "broker": "localhost:9092", "topic": "events", "message": "{\"event\":\"user.created\"}"},
    "schedule": {"type": "on_demand"}
  }'

# MQTT — publish a sensor reading
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "mqtt-temp",
    "task": {"type": "mqtt", "broker": "localhost", "port": 1883, "topic": "sensors/temp", "message": "22.5", "qos": 1},
    "schedule": {"type": "cron", "value": "0 * * * * *"}
  }'

# RabbitMQ — publish to an exchange
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "rabbitmq-event",
    "task": {"type": "rabbitmq", "url": "amqp://guest:guest@localhost:5672", "exchange": "events", "routing_key": "user.created", "message": "{\"user\":\"alice\"}", "content_type": "application/json"},
    "schedule": {"type": "on_demand"}
  }'

# Redis — publish to a channel
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "redis-notify",
    "task": {"type": "redis", "url": "redis://localhost:6379", "channel": "notifications", "message": "{\"type\":\"alert\"}"},
    "schedule": {"type": "on_demand"}
  }'
```

**Trigger fields:** `kind_pattern` (supports wildcards: `job.*`, `*`), `severity` (optional), `job_name_filter` (optional)

## Job Targeting

| Target | JSON | Description |
|---|---|---|
| Controller | `null` or `{"type": "local"}` | Runs on the controller (default) |
| Specific agent | `{"type": "agent", "agent_id": "uuid"}` | Runs on a specific agent |
| Any agent | `{"type": "any"}` | Random online agent (type-aware) |
| All agents | `{"type": "all"}` | Every online agent (type-aware) |
| Tagged | `{"type": "tagged", "tag": "linux"}` | Random agent with the tag (type-aware) |

## Executions

```bash
curl -X POST http://localhost:8080/api/jobs/{id}/trigger        # Trigger now
curl "http://localhost:8080/api/jobs/{id}/executions?page=1"     # History
curl http://localhost:8080/api/executions/{id}                   # Details
curl -X POST http://localhost:8080/api/executions/{id}/cancel    # Cancel
curl -X POST http://localhost:8080/api/executions/{id}/approve   # Approve (for approval-gated jobs)
curl http://localhost:8080/api/jobs/{id}/versions                # Job version history
```

### Approval Workflows

Jobs with `"approval_required": true` create a `pending_approval` execution when triggered. An admin or operator must approve it before it runs:

```bash
# Create a job that requires approval
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "deploy-prod", "task": {"type": "shell", "command": "deploy.sh"}, "schedule": "on_demand", "approval_required": true}'

# Trigger it (creates pending_approval execution)
curl -X POST http://localhost:8080/api/jobs/{id}/trigger

# Approve the execution
curl -X POST http://localhost:8080/api/executions/{exec_id}/approve
```

### Priority Scheduling

Set `"priority"` on a job (default 0, higher = runs first). When multiple jobs are due at the same time, higher priority jobs fire first:

```bash
curl -X PUT http://localhost:8080/api/jobs/{id} \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"priority": 10}'
```

### SLA Deadlines

Set a completion deadline per job. The background monitor fires events when running jobs approach or miss their deadline:

```bash
curl -X PUT http://localhost:8080/api/jobs/{id} \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"sla_deadline": "06:00", "sla_warning_mins": 15}'
```

When the job is still running at 05:45 UTC, a `sla.warning` event fires. At 06:00 UTC, a `sla.breach` event fires. Both trigger configured notifications (Slack, email, PagerDuty).

### Secret Variables

Variables with `"secret": true` have their values masked in API responses:

```bash
curl -X POST http://localhost:8080/api/variables \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "DB_PASSWORD", "value": "s3cret", "secret": true}'

# GET returns masked value: "••••••••"
# But {{DB_PASSWORD}} in task fields resolves to "s3cret" at runtime
```

## Agents

```bash
curl http://localhost:8080/api/agents                            # List
curl http://localhost:8080/api/agents/{id}                       # Details
curl -X DELETE http://localhost:8080/api/agents/{id}             # Deregister
curl http://localhost:8080/api/agents/{id}/task-types            # Get task types (no auth)
curl -X PUT http://localhost:8080/api/agents/{id}/task-types \   # Update task types
  -H "Authorization: Bearer kf_your_key" \
  -d '{"task_types": [...]}'
```

## Events

```bash
curl "http://localhost:8080/api/events?page=1&per_page=50"
```

## Scripts

```bash
curl http://localhost:8080/api/scripts                           # List
curl http://localhost:8080/api/scripts/{name}                    # Get
curl -X PUT http://localhost:8080/api/scripts/{name} \           # Create/update
  -d '{"code": "print(\"hello\");"}'
curl -X DELETE http://localhost:8080/api/scripts/{name}          # Delete
```

## Timeline

```bash
curl http://localhost:8080/api/timeline                          # Global execution timeline (minute buckets)
curl http://localhost:8080/api/timeline/{job_id}                 # Job-specific timeline
curl http://localhost:8080/api/timeline-detail/{bucket}          # Executions in a specific time bucket
```

## MCP Tool Discovery

```bash
curl "http://localhost:8080/api/mcp/tools?server_url=http://localhost:8000/mcp"
```

Connects to an MCP server via HTTP, performs the protocol handshake, and returns available tools with their input schemas.

### MCP Task Example

```bash
# Create a job that calls an MCP tool
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "mcp-greet",
    "task": {
      "type": "mcp",
      "server_url": "http://localhost:8000/mcp",
      "tool": "greet",
      "arguments": {"name": "World"}
    },
    "schedule": {"type": "on_demand"}
  }'
```

### Test Server Setup

A test MCP server is included at `examples/mcp_test_server.py`:

```bash
pip install mcp
python3 examples/mcp_test_server.py  # verify it runs
```

Tools: `greet(name)`, `add(a, b)`, `system_info()`, `word_count(text)`, `reverse(text)`.

## MCP Server

Kronforce exposes an MCP (Model Context Protocol) server at `POST /mcp` using the Streamable HTTP transport. Connect any MCP client to discover and manage jobs.

**Endpoint:** `POST /mcp`
**Auth:** API key via `Authorization: Bearer kf_...` header
**Headers required:** `Accept: application/json, text/event-stream` and `Content-Type: application/json`

**Available tools (by role):**

| Tool | Description | Min Role |
|------|-------------|----------|
| `list_jobs` | List jobs with optional group/status/search filter | Viewer |
| `get_job` | Get job details by name or ID | Viewer |
| `create_job` | Create a new job | Operator |
| `trigger_job` | Trigger a job execution | Operator |
| `list_executions` | List recent executions | Viewer |
| `get_execution` | Get execution output/status by ID | Viewer |
| `list_agents` | List registered agents | Viewer |
| `list_groups` | List job groups | Viewer |
| `list_events` | List recent system events | Viewer |
| `get_system_stats` | Dashboard stats overview | Viewer |

Configure with `KRONFORCE_MCP_ENABLED=false` to disable.

## Settings

```bash
curl http://localhost:8080/api/settings                          # Get all
curl -X PUT http://localhost:8080/api/settings \                 # Update
  -d '{"retention_days": "14"}'
```

## Chart Stats

```bash
curl http://localhost:8080/api/stats/charts                     # Execution outcomes, task types, schedule types
```

Returns aggregated data for dashboard charts: `execution_outcomes` (counts by status), `task_types` (counts by task type), `schedule_types` (counts by schedule kind).

## Audit Log

```bash
curl http://localhost:8080/api/audit-log                        # List (admin only)
curl "http://localhost:8080/api/audit-log?operation=job.created" # Filter by operation
curl "http://localhost:8080/api/audit-log?actor=deploy-bot"     # Filter by actor name
curl "http://localhost:8080/api/audit-log?since=2026-03-01T00:00:00Z" # Filter by time
```

Append-only audit trail of sensitive operations. Admin role required. Returns paginated results with: id, timestamp, actor (API key name + ID), operation, resource_type, resource_id, and details.

**Audited operations:** `key.created`, `key.revoked`, `job.created`, `job.updated`, `job.deleted`, `job.triggered`, `script.saved`, `script.deleted`, `settings.updated`, `variable.created`, `variable.updated`, `variable.deleted`, `agent.deregistered`

## API Keys

```bash
curl http://localhost:8080/api/keys                              # List
curl -X POST http://localhost:8080/api/keys \                    # Create
  -d '{"name": "CI pipeline", "role": "operator"}'
curl -X DELETE http://localhost:8080/api/keys/{id}               # Revoke
```

Roles: `admin` (full access + key management), `operator` (jobs + agents), `viewer` (read-only), `agent` (agent endpoints only — register, poll, heartbeat, callback).

## Authentication

On first startup, a bootstrap admin key is printed to the console. If no keys exist, auth is disabled.

Agent endpoints (register, poll, heartbeat, callback, task-type discovery) require an API key with the `agent` role when API keys are configured.

## Rate Limiting

All endpoints are rate limited. Exceeding the limit returns `429 Too Many Requests` with headers:

| Header | Description |
|---|---|
| `Retry-After` | Seconds until the rate limit window resets |
| `X-RateLimit-Limit` | Maximum requests allowed per minute for this tier |
| `X-RateLimit-Remaining` | Requests remaining in the current window |

**Default limits:**

| Tier | Limit | Scope |
|---|---|---|
| Public | 30/min | Per source IP |
| Authenticated | 120/min | Per API key |
| Agent | 600/min | Per API key |

Configure via `KRONFORCE_RATE_LIMIT_*` environment variables. Set `KRONFORCE_RATE_LIMIT_ENABLED=false` to disable.
