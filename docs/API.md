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
| Cron | `{"type": "cron", "value": "0 * * * * *"}` | Recurring cron schedule (6-field: sec min hour dom month dow). POSIX OR semantics for dom/dow. |
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

# Kafka — consume messages from a topic
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "kafka-consume",
    "task": {"type": "kafka_consume", "broker": "localhost:9092", "topic": "events", "max_messages": 10, "offset": "latest", "group_id": "kronforce-consumer"},
    "schedule": {"type": "cron", "value": "0 */5 * * * *"}
  }'

# MQTT — subscribe and receive messages
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "mqtt-subscribe",
    "task": {"type": "mqtt_subscribe", "broker": "localhost", "topic": "sensors/#", "max_messages": 5, "qos": 1},
    "schedule": {"type": "cron", "value": "0 * * * * *"},
    "timeout_secs": 30
  }'

# RabbitMQ — consume from a queue
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "rabbitmq-consume",
    "task": {"type": "rabbitmq_consume", "url": "amqp://guest:guest@localhost:5672", "queue": "tasks", "max_messages": 5},
    "schedule": {"type": "cron", "value": "0 */5 * * * *"}
  }'

# Redis — read from a list
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "redis-drain",
    "task": {"type": "redis_read", "url": "redis://localhost:6379", "key": "work-queue", "mode": "lpop", "count": 10},
    "schedule": {"type": "cron", "value": "0 * * * * *"}
  }'
```

Consume tasks output messages to stdout. Combine with output extraction rules to parse messages, write values to variables, or trigger downstream jobs.

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
curl -X POST http://localhost:8080/api/jobs/{id}/trigger              # Trigger now
curl -X POST http://localhost:8080/api/jobs/{id}/trigger \
  -d '{"params": {"version": "1.2.3"}}'                              # Trigger with params
curl -X POST "http://localhost:8080/api/jobs/{id}/trigger?skip_deps=true"  # Skip dependency checks
curl "http://localhost:8080/api/jobs/{id}/executions?page=1"          # History
curl http://localhost:8080/api/executions/{id}                        # Details
curl http://localhost:8080/api/executions/{id}/stream                 # Live output (SSE)
curl "http://localhost:8080/api/executions?group=ETL"                 # Filter by group
curl "http://localhost:8080/api/executions?status=failed&since=2026-04-01T00:00:00Z"  # Filter by status and time
curl -X POST http://localhost:8080/api/executions/{id}/cancel         # Cancel
curl -X POST http://localhost:8080/api/executions/{id}/approve        # Approve (for approval-gated jobs)
curl http://localhost:8080/api/jobs/{id}/versions                     # Job version history
curl -X POST http://localhost:8080/api/jobs/{id}/webhook              # Generate webhook token
curl -X DELETE http://localhost:8080/api/jobs/{id}/webhook             # Remove webhook token
curl -X POST http://localhost:8080/api/webhooks/{token}               # Trigger via webhook (no auth)
```

### Parameterized Runs

Define parameters on a job and pass values at trigger time:

```bash
# Create a job with parameters
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "deploy",
    "task": {"type": "shell", "command": "deploy.sh {{params.version}} {{params.env}}"},
    "schedule": {"type": "on_demand"},
    "parameters": [
      {"name": "version", "param_type": "text", "required": true},
      {"name": "env", "param_type": "select", "options": ["staging", "production"], "default": "staging"}
    ]
  }'

# Trigger with parameters
curl -X POST http://localhost:8080/api/jobs/{id}/trigger \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"params": {"version": "1.2.3", "env": "production"}}'
```

Parameters are substituted into task fields via `{{params.NAME}}` (alongside `{{VARIABLE}}` for global variables). The execution record stores which params were used.

### Webhook Triggers

Generate a unique URL that triggers a job without API key authentication:

```bash
# Enable webhook for a job
curl -X POST http://localhost:8080/api/jobs/{id}/webhook \
  -H "Authorization: Bearer kf_admin_key"
# Returns: {"token": "abc123...", "webhook_url": "/api/webhooks/abc123..."}

# Trigger via webhook (no auth required)
curl -X POST http://localhost:8080/api/webhooks/abc123...
# With parameters:
curl -X POST http://localhost:8080/api/webhooks/abc123... \
  -H "Content-Type: application/json" \
  -d '{"params": {"version": "1.2.3"}}'

# Disable webhook
curl -X DELETE http://localhost:8080/api/jobs/{id}/webhook \
  -H "Authorization: Bearer kf_admin_key"
```

### Concurrency Controls

Limit how many instances of a job can run simultaneously:

```bash
curl -X PUT http://localhost:8080/api/jobs/{id} \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"max_concurrent": 1}'
```

When `max_concurrent` is set and the job already has that many running/pending executions, the scheduler skips the fire. Default is 0 (unlimited).

### Live Output Streaming

Connect to the SSE endpoint to watch execution output in real-time:

```bash
curl -N http://localhost:8080/api/executions/{id}/stream \
  -H "Authorization: Bearer kf_admin_key"
```

Each line of stdout is sent as an SSE `message` event. Stderr lines are prefixed with `[stderr]`. A `done` event is sent when the execution completes. Only available for locally-executed jobs while they are running.

### Skip Dependencies

Jobs with `depends_on` normally skip execution when dependencies aren't satisfied. To force a single run regardless of dependency status, pass `?skip_deps=true`:

```bash
curl -X POST "http://localhost:8080/api/jobs/{id}/trigger?skip_deps=true" \
  -H "Authorization: Bearer kf_admin_key"
```

This is a one-time override — the job's dependency configuration is unchanged and future scheduled runs still check dependencies normally. The event log records that dependencies were skipped.

In the UI, click the "waiting" badge on a blocked job to see dependency status, then click **Run Anyway** to trigger with dependencies skipped.

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

### Calendar Schedule

Schedule jobs using business-day expressions instead of cron:

```bash
# Run 2 days before end of month at 9am
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "month-end-report",
    "task": {"type": "shell", "command": "./generate-report.sh"},
    "schedule": {"type": "calendar", "value": {"anchor": "last_day", "offset_days": -2, "hour": 9, "minute": 0}}
  }'

# Run on the 2nd Tuesday of every quarter (Jan, Apr, Jul, Oct)
curl -X POST http://localhost:8080/api/jobs \
  -d '{
    "name": "quarterly-review",
    "task": {"type": "shell", "command": "./quarterly.sh"},
    "schedule": {"type": "calendar", "value": {"anchor": "nth_weekday", "nth": 2, "weekday": "tuesday", "hour": 10, "minute": 0, "months": [1, 4, 7, 10]}}
  }'
```

**Anchor options:** `last_day`, `day_N` (e.g. `day_15`), `first_monday`...`first_friday`, `last_monday`...`last_friday`, `nth_weekday` (with `nth` and `weekday` fields).

**Offset:** `offset_days` shifts from the anchor (negative = before, positive = after). "Last day - 2" = `{"anchor": "last_day", "offset_days": -2}`.

**Months:** Empty array = every month. `[1, 7]` = January and July only.

**Business days:** `"skip_weekends": true` skips Saturday/Sunday. `"holidays": ["2026-12-25"]` skips specific dates.

### Interval Schedule

Run a job at a fixed delay after the last execution finishes:

```bash
# Run every 30 minutes after the previous run completes
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "polling-job", "task": {"type": "shell", "command": "./poll.sh"}, "schedule": {"type": "interval", "value": {"interval_secs": 1800}}}'
```

The scheduler checks if enough time has elapsed since the last execution's `finished_at`. Won't fire if the previous execution is still running.

### Timezone

Set `timezone` on a job (IANA format) for timezone-aware scheduling:

```bash
curl -X PUT http://localhost:8080/api/jobs/{id} \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"timezone": "America/New_York"}'
```

### Docker Build

Build Docker images from stored Dockerfile scripts:

```bash
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "build-app",
    "task": {"type": "docker_build", "script_name": "my-dockerfile", "image_tag": "my-app:latest", "run_after_build": true},
    "schedule": {"type": "on_demand"}
  }'
```

Create Dockerfile scripts in Toolbox → Scripts (select type "Dockerfile"). The build writes the Dockerfile to a temp directory and runs `docker build`. Set `run_after_build: true` to also run the container.

### Schedule Window

Constrain when a job's schedule is active with `starts_at` and `expires_at` (ISO 8601 datetimes). These work with any schedule type (cron, one-shot, on-demand, event, calendar):

```bash
# Run every hour, but only for the next 3 weeks
curl -X PUT http://localhost:8080/api/jobs/{id} \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"starts_at": "2026-04-07T00:00:00Z", "expires_at": "2026-04-28T00:00:00Z"}'
```

- **`starts_at`** — the scheduler won't fire the job before this time
- **`expires_at`** — the scheduler stops firing after this time and marks the job as unscheduled

Both fields are optional and independent. Set `starts_at` to delay activation ("start next Monday"), or `expires_at` for temporary jobs ("run for 3 weeks then stop"), or both for a fixed window. Manual triggers via `?skip_deps=true` are not affected by the schedule window.

### Job Templates

Save reusable job definitions and create new jobs from them:

```bash
# List all templates
curl http://localhost:8080/api/templates

# Save a template
curl -X POST http://localhost:8080/api/templates \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "health-check-template", "description": "HTTP health check pattern", "snapshot": {"task": {"type": "http", "method": "GET", "url": "https://example.com/health"}, "notifications": {"on_failure": true}}}'

# Delete a template
curl -X DELETE http://localhost:8080/api/templates/health-check-template
```

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

## Pipeline Schedules

Set recurring schedules on pipeline groups to automatically trigger root jobs. Dependencies cascade as usual.

```bash
# Get pipeline schedule for a group
curl http://localhost:8080/api/jobs/pipeline-schedule/ETL

# Set a cron schedule (triggers root jobs on schedule)
curl -X PUT http://localhost:8080/api/jobs/pipeline-schedule/ETL \
  -d '{"schedule": {"type": "cron", "value": "0 0 6 * * *"}}'

# Set an interval schedule (every N seconds after last fire)
curl -X PUT http://localhost:8080/api/jobs/pipeline-schedule/ETL \
  -d '{"schedule": {"type": "interval", "value": {"interval_secs": 3600}}}'

# Remove a pipeline schedule
curl -X DELETE http://localhost:8080/api/jobs/pipeline-schedule/ETL
```

The schedule fires root jobs (jobs with no in-group dependencies). Dependent jobs then cascade automatically via the dependency engine.

## Connections

Named credential profiles for databases, APIs, and services. Configs are encrypted at rest.

```bash
# List all connections (sensitive fields masked)
curl http://localhost:8080/api/connections

# Create a PostgreSQL connection
curl -X POST http://localhost:8080/api/connections \
  -d '{"name":"prod-db","conn_type":"postgres","description":"Production DB","config":{"connection_string":"postgresql://user:pass@host:5432/db"}}'

# Create an HTTP connection with bearer auth
curl -X POST http://localhost:8080/api/connections \
  -d '{"name":"vendor-api","conn_type":"http","config":{"base_url":"https://api.vendor.com","auth_type":"bearer","token":"my-token"}}'

# Update (send ******** for sensitive fields to preserve existing values)
curl -X PUT http://localhost:8080/api/connections/prod-db \
  -d '{"description":"Updated description","config":{"connection_string":"********"}}'

# Test connectivity
curl -X POST http://localhost:8080/api/connections/prod-db/test

# Delete
curl -X DELETE http://localhost:8080/api/connections/prod-db

# Use in a job — add "connection" field to any supported task type
curl -X POST http://localhost:8080/api/jobs -d '{
  "name": "daily-report",
  "task": {"type": "sql", "driver": "postgres", "query": "SELECT count(*) FROM orders", "connection": "prod-db"},
  "schedule": {"type": "cron", "value": "0 0 8 * * *"}
}'
```

Supported types: `postgres`, `mysql`, `sqlite`, `ftp`, `sftp`, `http`, `kafka`, `mqtt`, `rabbitmq`, `redis`, `mongodb`, `ssh`, `smtp`, `s3`.

## AI Assistant

Generate job configurations from natural language descriptions. Requires `KRONFORCE_AI_API_KEY`.

```bash
# Generate a job from a description
curl -X POST http://localhost:8080/api/ai/generate-job \
  -d '{"prompt": "back up postgres every night at 3am with 1 hour timeout"}'

# Returns a complete job configuration:
# {"job": {"name": "postgres-backup", "task": {"type": "shell", "command": "pg_dump ..."}, "schedule": {"type": "cron", "value": "0 0 3 * * *"}, ...}}
```

The response is a JSON job definition that can be used directly with `POST /api/jobs` or previewed in the UI. Supports Anthropic (default) and OpenAI providers.

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

## Data Export

```bash
# Export all data (secrets masked)
curl http://localhost:8080/api/data/export

# Export with decrypted secrets, connections, and API key metadata (admin only)
curl "http://localhost:8080/api/data/export?include_secrets=true"
```

The standard export includes jobs, variables (secrets masked), templates, agents, groups, and settings. With `?include_secrets=true`, the export also includes:
- **Variables** with decrypted secret values
- **Connections** with decrypted configs (passwords, tokens, connection strings)
- **API keys** metadata (name, role, permissions, expiry — not raw keys or hashes)

Use this for backup and migration between Kronforce instances. Admin role required.

## Data Import

```bash
# Import from a previous export (admin only)
curl -X POST http://localhost:8080/api/data/import \
  -H "Content-Type: application/json" \
  -d @export.json
```

Restores data from an export JSON. Imports jobs, variables, connections, groups, and settings. Items that already exist (by name or ID) are skipped rather than overwritten. Masked secret variables (`********`) are also skipped.

Returns a summary of imported and skipped counts per resource type:
```json
{
  "status": "ok",
  "imported": { "jobs": 5, "variables": 3, "connections": 2, "groups": 1, "settings": 4 },
  "skipped": { "variables": 1, "connections": 1 }
}
```

Admin role required.

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
