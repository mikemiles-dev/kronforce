# Kronforce

A workload automation and job scheduling engine built in Rust.

![Kronforce Dashboard](screenshot.png)

## Quick Start

```bash
cargo run
```

The server starts on `0.0.0.0:8080` with a SQLite database (`kronforce.db`) in the current directory.

### Configuration

| Environment Variable | Default | Description |
|---|---|---|
| `KRONFORCE_DB` | `kronforce.db` | Path to SQLite database file |
| `KRONFORCE_BIND` | `0.0.0.0:8080` | Address and port to listen on |
| `KRONFORCE_TICK_SECS` | `1` | Scheduler tick interval in seconds |

```bash
KRONFORCE_DB=mydata.db KRONFORCE_BIND=127.0.0.1:3000 cargo run
```

## API

### Jobs

```bash
# Create a cron job (runs every minute)
curl -X POST http://localhost:8080/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "cleanup",
    "command": "echo running cleanup",
    "schedule": {"type": "cron", "value": "0 * * * * *"}
  }'

# Create a manual job (triggered via API only)
curl -X POST http://localhost:8080/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "deploy",
    "command": "/opt/scripts/deploy.sh",
    "schedule": {"type": "manual"},
    "timeout_secs": 300
  }'

# Create a one-shot job (runs once at a specific time)
curl -X POST http://localhost:8080/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "migration",
    "command": "./migrate.sh",
    "schedule": {"type": "one_shot", "value": "2026-04-01T00:00:00Z"}
  }'

# List all jobs
curl http://localhost:8080/api/jobs

# List jobs filtered by status
curl http://localhost:8080/api/jobs?status=active

# Get a specific job
curl http://localhost:8080/api/jobs/{id}

# Update a job
curl -X PUT http://localhost:8080/api/jobs/{id} \
  -H 'Content-Type: application/json' \
  -d '{"command": "echo updated command"}'

# Delete a job
curl -X DELETE http://localhost:8080/api/jobs/{id}
```

### Execution

```bash
# Manually trigger a job
curl -X POST http://localhost:8080/api/jobs/{id}/trigger

# View execution history for a job
curl http://localhost:8080/api/jobs/{id}/executions
curl http://localhost:8080/api/jobs/{id}/executions?limit=50

# Get execution details (includes stdout/stderr)
curl http://localhost:8080/api/executions/{id}

# Cancel a running execution
curl -X POST http://localhost:8080/api/executions/{id}/cancel
```

### Health

```bash
curl http://localhost:8080/api/health
# {"status":"ok"}
```

## Cron Expressions

Kronforce uses 6-field cron expressions with second-level precision:

```
sec min hour day_of_month month day_of_week
```

| Expression | Description |
|---|---|
| `* * * * * *` | Every second |
| `0 * * * * *` | Every minute |
| `0 0 * * * *` | Every hour |
| `0 0 9 * * *` | Daily at 9:00 AM |
| `0 0 9 * * 1-5` | Weekdays at 9:00 AM |
| `0 */5 * * * *` | Every 5 minutes |
| `0 0 0 1 * *` | First of every month at midnight |
| `*/30 * * * * *` | Every 30 seconds |

Supports: `*`, ranges (`1-5`), lists (`1,3,5`), steps (`*/5`, `1-30/5`).

## Dependencies

Jobs can declare dependencies on other jobs. A job only runs when all its dependencies have a recent successful execution.

```bash
# Create parent job
curl -X POST http://localhost:8080/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{"name": "extract", "command": "extract.sh", "schedule": {"type": "cron", "value": "0 0 2 * * *"}}'

# Create child job that depends on the parent
curl -X POST http://localhost:8080/api/jobs \
  -H 'Content-Type: application/json' \
  -d '{
    "name": "transform",
    "command": "transform.sh",
    "schedule": {"type": "cron", "value": "0 0 3 * * *"},
    "depends_on": ["<extract-job-id>"]
  }'
```

Circular dependencies are rejected at creation time.

## Architecture

Single-process architecture with three main components communicating via channels:

```
┌──────────┐    mpsc     ┌───────────┐    spawn    ┌──────────┐
│  REST    │───────────▶│ Scheduler │────────────▶│ Executor │
│  API     │            │  (1s tick) │             │ (tokio)  │
└──────────┘            └───────────┘             └──────────┘
     │                       │                         │
     └───────────┬───────────┘─────────────────────────┘
                 ▼
            ┌─────────┐
            │ SQLite  │
            │  (WAL)  │
            └─────────┘
```

- **Scheduler** — Tokio task that ticks every second, checks for due jobs, resolves dependencies, and dispatches execution
- **Executor** — Spawns shell processes with stdout/stderr capture, timeout enforcement, and cancellation support
- **API** — Axum REST server for job management, triggering, and log viewing
- **Storage** — SQLite with WAL mode, single `Mutex<Connection>`

## Job Statuses

| Status | Description |
|---|---|
| `active` | Scheduled and will run on its cron/one-shot schedule |
| `paused` | Exists but won't be scheduled until reactivated |
| `disabled` | Permanently disabled (one-shot jobs move here after firing) |

## Execution Statuses

| Status | Description |
|---|---|
| `running` | Currently executing |
| `succeeded` | Completed with exit code 0 |
| `failed` | Completed with non-zero exit code |
| `timed_out` | Killed after exceeding `timeout_secs` |
| `cancelled` | Cancelled via API |
| `skipped` | Skipped due to failed dependency |

## Development

```bash
# Build
cargo build

# Run tests
cargo test

# Run with debug logging
RUST_LOG=kronforce=debug cargo run
```
