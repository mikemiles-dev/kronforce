# Code Architecture

Kronforce is a single Rust crate (`kronforce`) that produces two binaries: `kronforce` (controller) and `kronforce-agent` (agent). The crate is ~11,500 lines of Rust across 57 source files.

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                     src/bin/controller.rs                           │
│                     (entry point — startup, wiring)                 │
└──────────┬──────────┬──────────┬──────────┬──────────┬─────────────┘
           │          │          │          │          │
     ┌─────▼────┐ ┌───▼────┐ ┌──▼───┐ ┌───▼────┐ ┌───▼──────────┐
     │  api/    │ │scheduler│ │ db/  │ │executor│ │ mcp_server   │
     │ (Axum   │ │ (cron   │ │(r2d2 │ │(run    │ │ (MCP tools   │
     │  REST)  │ │  loop)  │ │ pool)│ │ tasks) │ │  for AI)     │
     └────┬────┘ └───┬────┘ └──┬───┘ └───┬────┘ └──────────────┘
          │          │          │          │
          └──────────┴──────────┴──────────┘
                     shared via AppState
```

## Source Tree

```
src/
├── lib.rs                    # Crate root — re-exports all public modules
├── error.rs                  # AppError enum → HTTP status codes (60 lines)
├── config.rs                 # ControllerConfig + AgentConfig from env vars (130 lines)
├── dag.rs                    # DAG cycle detection + dependency resolution (102 lines)
│
├── bin/
│   ├── controller.rs         # Controller entry point — wires everything together (240 lines)
│   └── agent.rs              # Agent entry point — register, heartbeat, serve (132 lines)
│
├── api/                      # REST API layer (2,669 lines)
│   ├── mod.rs                # Router builder, AppState, middleware stack
│   ├── auth.rs               # API key auth middleware, role guards, key CRUD
│   ├── jobs.rs               # Job CRUD, trigger, groups, bulk operations
│   ├── executions.rs         # Execution list/detail/cancel
│   ├── agents.rs             # Agent register, heartbeat, queue, task types
│   ├── callbacks.rs          # Execution result callback from agents
│   ├── events.rs             # Event list, timeline buckets
│   ├── scripts.rs            # Rhai script CRUD
│   ├── settings.rs           # Settings CRUD, notification config
│   ├── variables.rs          # Global variable CRUD
│   ├── stats.rs              # Chart stats aggregation
│   ├── audit.rs              # Audit log query endpoint
│   ├── mcp.rs                # MCP tool discovery endpoint (client-side)
│   └── rate_limit.rs         # Per-IP/per-key sliding window rate limiter
│
├── db/                       # Database layer (2,263 lines + 878 lines models)
│   ├── mod.rs                # Db struct (r2d2 pool), migrations, db_call helper
│   ├── helpers.rs            # QueryFilters builder for dynamic WHERE clauses
│   ├── jobs.rs               # Job queries — CRUD, filters, groups, chart stats
│   ├── executions.rs         # Execution queries — CRUD, timeline, outcome counts
│   ├── agents.rs             # Agent queries — upsert, heartbeat, expire
│   ├── events.rs             # Event queries — insert, list, count
│   ├── keys.rs               # API key queries — lookup by hash, CRUD
│   ├── settings.rs           # Settings KV store, retention purge
│   ├── variables.rs          # Global variable queries
│   ├── queue.rs              # Custom agent job queue — enqueue, dequeue, stale cleanup
│   ├── audit.rs              # Audit log — record, list, purge
│   └── models/               # Data structures (878 lines)
│       ├── mod.rs            # Re-exports all model types
│       ├── job.rs            # Job, ScheduleKind, Dependency, OutputRules, AgentTarget
│       ├── task.rs           # TaskType enum (12 variants), McpTransport, SqlDriver, etc.
│       ├── execution.rs      # ExecutionRecord, ExecutionStatus, TriggerSource
│       ├── agent.rs          # Agent, AgentStatus, AgentType, TaskTypeDefinition
│       ├── auth.rs           # ApiKey, ApiKeyRole
│       ├── event.rs          # Event, EventSeverity
│       └── variable.rs       # Variable
│
├── executor/                 # Task execution engine (1,735 + 1,403 lines)
│   ├── mod.rs                # Executor struct, variable substitution, dispatch routing
│   ├── local.rs              # Local execution — spawn process, capture output, retry logic
│   ├── dispatch.rs           # Agent dispatch — pick agent, send HTTP, enqueue for custom
│   ├── notifications.rs      # Email (SMTP) + SMS (webhook) notification dispatch
│   ├── output_rules.rs       # Post-execution: regex/jsonpath extraction, assertions, triggers
│   ├── scripts.rs            # Rhai script file store — CRUD, file watcher
│   └── tasks/                # Per-task-type executors (1,403 lines)
│       ├── mod.rs            # Re-exports all task runners
│       ├── shell.rs          # sh -c / cmd /C execution
│       ├── http.rs           # reqwest HTTP client with SSRF protection
│       ├── sql.rs            # Shell out to psql/mysql/sqlite3
│       ├── ftp.rs            # curl with netrc for FTP/SFTP
│       ├── script.rs         # Rhai engine with builtins (http, shell, tcp, udp)
│       ├── file_push.rs      # Base64 decode + write to filesystem
│       ├── messaging.rs      # Kafka, RabbitMQ, MQTT, Redis via CLI tools
│       └── mcp.rs            # MCP client — JSON-RPC over HTTP, tool discovery
│
├── scheduler/                # Job scheduling (825 lines)
│   ├── mod.rs                # Tick loop, command handler, event triggers, cron/oneshot firing
│   └── cron_parser.rs        # 6-field cron expression parser + next-fire calculator
│
├── agent/                    # Agent communication (326 lines)
│   ├── mod.rs                # Re-exports
│   ├── client.rs             # AgentClient — HTTP dispatch + cancel to remote agents
│   ├── protocol.rs           # JobDispatchRequest/Response, ExecutionResultReport
│   └── server.rs             # Agent HTTP server — /execute, /cancel, /health, /shutdown
│
└── mcp_server.rs             # MCP server endpoint (671 lines)
                              # JSON-RPC handler, 10 tools, role-based filtering, SSE responses
```

## Data Flow

### Job Execution (Local)

```
Scheduler tick
  → CronSchedule::next_after() matches
  → DagResolver::deps_satisfied() checks dependencies
  → Executor::execute()
    → substitute_variables() replaces {{VAR}} in task fields
    → Executor::execute_local()
      → tokio::spawn background task
        → run_task() dispatches to task-specific handler
        → handle_execution_complete()
          → update execution in DB
          → run output rules (extract, assert, trigger)
          → send notifications
          → check retry logic → schedule retry if needed
          → emit execution.completed event
```

### Job Execution (Agent)

```
Executor::execute()
  → dispatch_to_agent() / dispatch_to_tagged() / dispatch_to_any()
    → Standard agent: HTTP POST to agent /execute endpoint
    → Custom agent: enqueue in job_queue table
  → Agent runs task, POSTs result to /api/callbacks/execution-result
    → callbacks handler updates execution, runs output rules, sends notifications
```

### Request Flow (API)

```
HTTP Request
  → Axum router
  → Rate limit middleware (per-IP or per-key)
  → Auth middleware (validate Bearer token, set ApiKey in extensions)
  → Handler (e.g., jobs::create_job)
    → db_call() → spawn_blocking → pool.get() → SQL query
    → Return JSON response
```

### MCP Server Flow

```
POST /mcp
  → Auth middleware (API key from Authorization header)
  → mcp_handler()
    → Validate Accept header
    → Parse JSON-RPC message
    → Dispatch: initialize / tools/list / tools/call
    → tools/call → execute_tool() → calls Db methods
    → Return SSE response (event: message\ndata: {...})
```

## Key Design Patterns

### Connection Pool (r2d2)
Every DB operation calls `self.pool.get()` to get a pooled connection from `r2d2::Pool<SqliteConnectionManager>`. WAL mode + foreign keys + busy_timeout are set on each connection via `with_init`. Pool size defaults to 8, forced to 1 for `:memory:` test databases.

### Async Wrapper (`db_call`)
All DB operations are synchronous (rusqlite). The `db_call` helper wraps them in `tokio::task::spawn_blocking` so they don't block the async runtime:
```rust
pub async fn db_call<F, T>(db: &Db, f: F) -> Result<T, AppError>
```

### Error Handling
`AppError` enum maps to HTTP status codes. All DB operations return `Result<T, AppError>`. The `IntoResponse` impl on `AppError` returns JSON error bodies.

### Task Execution
Each task type has a dedicated async function in `src/executor/tasks/` that returns `CommandResult { status, exit_code, stdout, stderr }`. The `run_task` function in `local.rs` matches on `TaskType` and dispatches.

### Variable Substitution
`substitute_variables()` in `executor/mod.rs` serializes any `TaskType` to JSON, replaces `{{VAR_NAME}}` patterns using a regex, then deserializes back. This works generically across all task types.

### Scheduler Commands
The scheduler runs in its own tokio task and receives commands via an `mpsc` channel:
- `Reload` — invalidate job cache
- `TriggerNow(job_id, skip_deps)` — fire a job immediately; when `skip_deps` is true, dependency checks are bypassed for this single run
- `CancelExecution(exec_id)` — cancel a running execution
- `EventOccurred(event)` — check event-triggered jobs
- `RetryExecution { job_id, original_id, attempt }` — retry a failed job

### Build System
`build.rs` bundles the web frontend at compile time:
- Reads `web/index.html` and processes `<!-- INCLUDE:path -->` markers
- Concatenates all `web/js/*.js` files (app.js first)
- Inlines `web/css/style.css`
- Embeds migrations from `migrations/*.sql`
- Result: single `dashboard.html` string via `include_str!`
