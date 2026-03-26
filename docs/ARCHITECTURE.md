# Architecture

## System Diagram

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
                    ┌────────────────────────────────────┘
                    │
        ┌───────────┴───────────┐
        │                       │
        ▼                       ▼
┌────────────────┐    ┌────────────────┐
│ STANDARD AGENT │    │ CUSTOM AGENT   │
│   (push)       │    │   (pull)       │
│                │    │                │
│ HTTP /execute  │    │ Polls queue    │
│ Runs sh -c     │    │ Any language   │
│ Reports back   │    │ Reports back   │
└────────────────┘    └────────────────┘
```

## Execution Flow

1. **Scheduler** detects a due job (cron tick, one-shot time, or event match)
2. **Dependency check** — if job has `depends_on`, verify all parents succeeded within their time windows
3. **Executor** determines where to run based on target:
   - **Local**: runs on the controller via `sh -c` (or `sudo -n -u` with `run_as`)
   - **Standard Agent**: dispatches via HTTP POST to the agent's `/execute` endpoint
   - **Custom Agent**: enqueues in `job_queue` table for the agent to poll
4. **Output capture** — stdout/stderr captured (256KB cap per stream with truncation)
5. **Output rules** — if the job has extraction rules or triggers, they run against stdout:
   - Extracted values stored on the execution record
   - Matched trigger patterns emit `output.matched` events
6. **Result** — execution record updated in SQLite with status, output, extracted values
7. **Events** — `execution.completed` event emitted, which can trigger other event-driven jobs

## Components

| Component | File | Description |
|---|---|---|
| REST API | `src/api.rs` | Axum HTTP server — dashboard, job CRUD, agent management, callbacks, settings |
| Scheduler | `src/scheduler.rs` | Tick-based cron evaluator with mpsc channel for reload/trigger/event commands |
| Executor | `src/executor.rs` | Runs tasks locally or dispatches to agents. Handles timeouts, cancellation, output rules |
| Database | `src/db.rs` | SQLite with WAL mode. 11 versioned migrations. Jobs, executions, agents, events, settings, queue |
| Models | `src/models.rs` | All data types — TaskType, ScheduleKind, AgentTarget, OutputRules, etc. |
| Output Rules | `src/output_rules.rs` | Regex/jsonpath extraction engine and trigger pattern matcher |
| Scripts | `src/scripts.rs` | Rhai script file store — CRUD, file discovery, name validation |
| Agent Client | `src/agent_client.rs` | HTTP client for dispatching jobs to standard agents |
| Agent Server | `src/agent_server.rs` | HTTP server that standard agents run (receives /execute, /cancel, /health) |
| Dashboard | `src/dashboard.html` | Single-file HTML embedded via `include_str!` — all pages, CSS, JS |
| Config | `src/config.rs` | Environment variable parsing for controller and agent |

## Agent Types

| Type | Model | Registration | Task Types | Dispatch |
|---|---|---|---|---|
| Standard | Push | Controller pushes via HTTP POST | Shell, HTTP, SQL, FTP, Script | Immediate via `/execute` |
| Custom | Pull | Agent polls `GET /api/agent-queue/{id}/next` | UI-defined per agent | Queued in `job_queue` table |

## Execution Modes

| Mode | Task Types | Targets | Description |
|---|---|---|---|
| Standard | Shell, HTTP, SQL, FTP, Script | Local / Specific / Any / All / Tagged | Built-in task types run on controller or standard agents |
| Custom Agent | Defined per agent in UI | Specific custom agent | Custom task data dispatched to pull-based agents |

`Any`, `All`, and `Tagged` targets are **type-aware** — they only pick agents matching the task type (standard for built-in, custom for custom tasks).

## Task Types

| Type | Execution Method | Key Fields |
|---|---|---|
| `shell` | `sh -c` (or `sudo -n -u` with run_as) | `command` |
| `http` | In-process reqwest HTTP client | `method`, `url`, `headers`, `body`, `expect_status` |
| `sql` | Shells out to `psql`/`mysql`/`sqlite3` | `driver`, `connection_string`, `query` |
| `ftp` | Uses curl for transfers | `protocol`, `host`, `port`, `username`, `password`, `direction`, `remote_path`, `local_path` |
| `script` | Embedded Rhai scripting engine | `script_name` |
| `custom` | Dispatched to custom agent | `agent_task_type`, `data` (arbitrary JSON) |

## Schedule Types

| Type | Description |
|---|---|
| Cron | 6-field second-precision cron (`sec min hour dom month dow`) |
| One-shot | Fire once at a specific UTC datetime |
| On-demand | Manual trigger only (API or UI) |
| Event | Fire when a matching system event occurs |

## Database Schema

**Tables**: `jobs`, `executions`, `agents`, `events`, `api_keys`, `job_queue`, `settings`, `schema_version`

**Key design decisions**:
- JSON columns for flexible nested data (`task_json`, `schedule_json`, `depends_on_json`, `output_rules_json`, `task_types_json`, `extracted_json`)
- WAL mode for concurrent reads during writes
- Foreign keys enforced
- 11 versioned migrations applied automatically on startup

## Event System

Events are the connective tissue of the system. Every significant action emits an event, and event-triggered jobs can react to any event kind. See [Triggers & Workflows](TRIGGERS_AND_WORKFLOWS.md) for the full event-driven architecture.

## Data Retention

Configurable via Settings page or API (`PUT /api/settings`). The health monitor loop (every 10 seconds) purges:
- Completed executions older than N days
- Events older than N days
- Completed queue items older than N days

Default: 7 days. Set to 0 to disable purging.

## Queue System (Custom Agents)

Custom agent jobs are enqueued in the `job_queue` table with statuses: `pending` → `claimed` → `completed`.

**Stale cleanup** (runs every 10 seconds):
- Unclaimed jobs (`pending` > 5 minutes) → failed with timeout message
- Abandoned jobs (`claimed` > 10 minutes) → failed with timeout message

## Authentication

API key middleware on all `/api/*` routes (except health, agent registration, polling, callbacks). Three roles: `admin`, `operator`, `viewer`. Auto-disabled when no keys exist (first-time setup).
