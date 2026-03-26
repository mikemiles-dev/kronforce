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
            {"name": "count", "pattern": "$.results.count", "type": "jsonpath"}
        ],
        "triggers": [
            {"pattern": "ERROR|FATAL", "severity": "error"},
            {"pattern": "WARNING", "severity": "warning"}
        ]
    }
}
```

Extractions run against stdout after each execution. Extracted values appear in `GET /api/executions/{id}` as the `extracted` field. Triggers emit `output.matched` events. See [Triggers & Workflows](TRIGGERS_AND_WORKFLOWS.md).

**Trigger fields:** `kind_pattern` (supports wildcards: `job.*`, `*`), `severity` (optional), `job_name_filter` (optional)

## Job Targeting

| Target | JSON | Description |
|---|---|---|
| Local | `null` or `{"type": "local"}` | Runs on the controller (default) |
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

## Settings

```bash
curl http://localhost:8080/api/settings                          # Get all
curl -X PUT http://localhost:8080/api/settings \                 # Update
  -d '{"retention_days": "14"}'
```

## API Keys

```bash
curl http://localhost:8080/api/keys                              # List
curl -X POST http://localhost:8080/api/keys \                    # Create
  -d '{"name": "CI pipeline", "role": "operator"}'
curl -X DELETE http://localhost:8080/api/keys/{id}               # Revoke
```

Roles: `admin` (full access), `operator` (jobs + agents), `viewer` (read-only).

## Authentication

On first startup, a bootstrap admin key is printed to the console. If no keys exist, auth is disabled.

Agent endpoints (register, poll, heartbeat, callback, task-type discovery) require **no API key**.
