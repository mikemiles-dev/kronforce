# Triggers, Events & Workflows

Kronforce has a powerful event-driven system that lets you chain jobs together, react to output patterns, and build complex automation workflows. This guide covers all the ways jobs can be triggered and how they compose.

## Ways to Trigger a Job

| Method | How | Use Case |
|---|---|---|
| **Cron schedule** | `schedule: { type: "cron", value: "0 */5 * * * *" }` | Periodic tasks — health checks, reports, backups |
| **One-shot** | `schedule: { type: "one_shot", value: "2026-04-01T00:00:00Z" }` | Run once at a specific time — migrations, deployments |
| **Manual (API)** | `POST /api/jobs/{id}/trigger` | On-demand from CI/CD, scripts, or the dashboard UI |
| **Manual (UI)** | Click the play button on a job | Quick ad-hoc execution |
| **Event trigger** | `schedule: { type: "event", value: { kind_pattern: "..." } }` | React to system events — failures, agent changes, output matches |
| **Dependency chain** | `depends_on: [{ job_id: "...", within_secs: 3600 }]` | ETL pipelines — only run if parent succeeded recently |
| **Output pattern match** | Output triggers on a parent job emit `output.matched` events | React to specific content in job output |
| **Webhook** | `POST /api/webhooks/{token}` (no auth) | External integrations — GitHub, CI/CD, Stripe, PagerDuty |

## Parameterized Runs

Jobs can define parameter schemas. When triggered (via API, UI, or webhook), callers pass runtime values that are substituted into the task using `{{params.NAME}}`:

```bash
# Job created with parameters
curl -X POST http://localhost:8080/api/jobs \
  -d '{"name": "deploy", "task": {"type": "shell", "command": "deploy.sh {{params.version}} {{params.env}}"}, "schedule": {"type": "on_demand"}, "parameters": [{"name": "version", "param_type": "text", "required": true}, {"name": "env", "param_type": "select", "options": ["staging","production"], "default": "staging"}]}'

# Triggered with values
curl -X POST http://localhost:8080/api/jobs/{id}/trigger \
  -d '{"params": {"version": "1.2.3", "env": "production"}}'
```

Parameters work alongside global variables (`{{VAR_NAME}}`). Params are resolved first, then global variables. The execution record stores which params were used.

In the UI, triggering a parameterized job opens a form pre-filled with defaults.

## Webhook Triggers

Generate a unique token URL for any job. External systems can trigger it without an API key:

```bash
# Enable webhook
curl -X POST http://localhost:8080/api/jobs/{id}/webhook -H "Authorization: Bearer kf_key"
# Returns: {"token": "abc...", "webhook_url": "/api/webhooks/abc..."}

# External system triggers it (no auth)
curl -X POST http://localhost:8080/api/webhooks/abc...

# With parameters
curl -X POST http://localhost:8080/api/webhooks/abc... \
  -d '{"params": {"version": "1.2.3"}}'
```

Webhook URLs are shown in the job detail view with copy/disable buttons. Disable with `DELETE /api/jobs/{id}/webhook`.

## Event-Driven Jobs

Event-triggered jobs fire automatically when a matching system event occurs. Configure them with `schedule.type: "event"`.

### Event Kinds

| Kind | When It Fires |
|---|---|
| `job.created` | A job is created |
| `job.updated` | A job is edited |
| `job.deleted` | A job is deleted |
| `job.triggered` | A job is manually triggered |
| `execution.completed` | An execution finishes (success or failure) |
| `output.matched` | An output trigger pattern matches stdout/stderr |
| `agent.registered` | An agent registers with the controller |
| `agent.offline` | An agent heartbeat times out |
| `agent.unpaired` | An agent is removed |
| `key.created` | An API key is created |
| `key.revoked` | An API key is revoked |

### Event Trigger Configuration

```json
{
    "schedule": {
        "type": "event",
        "value": {
            "kind_pattern": "execution.completed",
            "severity": "error",
            "job_name_filter": "etl-pipeline"
        }
    }
}
```

| Field | Required | Description |
|---|---|---|
| `kind_pattern` | Yes | Event kind to match. Supports exact (`output.matched`), prefix wildcard (`job.*`), or all (`*`) |
| `severity` | No | Only trigger on events with this severity: `success`, `error`, `warning`, `info` |
| `job_name_filter` | No | **Source job filter.** Only trigger when the event came from a job with this name (substring match, case-insensitive). In the UI this is labeled "From source job". |

**How `job_name_filter` works:** When an event is emitted, its message includes the source job name (e.g., `"Output pattern matched: 'ERROR' in job 'health-check'"`). The `job_name_filter` does a case-insensitive substring match against this message. So setting `job_name_filter: "health-check"` means "only fire when the event came from a job named health-check".

## Output Intelligence

Jobs can define **output rules** that process stdout after each execution. Two types:

### Output Extractions

Pull structured values from unstructured output using regex or JSON path:

```json
{
    "output_rules": {
        "extractions": [
            { "name": "duration_ms", "pattern": "completed in (\\d+)ms", "type": "regex" },
            { "name": "record_count", "pattern": "$.results.count", "type": "jsonpath", "write_to_variable": "LAST_RECORD_COUNT" }
        ]
    }
}
```

- **Regex**: Captures group 1 (or named groups) as the value
- **JSON path**: Parses stdout as JSON and traverses dot-notation paths (e.g., `$.data.total`)
- Extracted values are stored on the execution and displayed in the execution detail modal
- **Write to variable**: Optionally set `write_to_variable` on an extraction rule to upsert the extracted value into a global variable. This enables job chaining — one job extracts a value, another job uses it via `{{VAR_NAME}}` substitution.
- Maximum 10 extraction rules per job

### Global Variables

Global variables are shared key-value pairs that can be referenced in any task field using `{{VAR_NAME}}` syntax. Variables are substituted controller-side before execution, so they work for both local and remote agent jobs.

- **Manage variables** in the Variables page or via `POST/PUT/DELETE /api/variables`
- **Use in tasks**: Reference variables in shell commands, HTTP URLs, SQL queries, message bodies, etc. — e.g., `curl {{API_HOST}}/status`
- **Auto-update from jobs**: Set `write_to_variable` on an extraction rule to have a job update a variable with its output
- **Pipeline pattern**: Job A extracts a value into `DEPLOY_VERSION`, Job B uses `{{DEPLOY_VERSION}}` in its command
- Unresolved `{{VAR}}` references are left as-is (with a warning logged), so shell variables like `${HOME}` are unaffected

### Output Triggers

Emit events when output matches a pattern:

```json
{
    "output_rules": {
        "triggers": [
            { "pattern": "ERROR|FATAL", "severity": "error" },
            { "pattern": "WARNING", "severity": "warning" },
            { "pattern": "records processed: 0", "severity": "warning" }
        ]
    }
}
```

When a pattern matches stdout or stderr, the system emits an `output.matched` event. Other event-triggered jobs can react to these events.

Patterns are treated as regex first. If the regex is invalid, falls back to substring matching.

### Output Assertions

Fail the execution if expected patterns are NOT found in stdout. Only checked on successful runs — if the command itself fails, assertions are skipped.

```json
{
    "output_rules": {
        "assertions": [
            { "pattern": "OK", "message": "Health check did not return OK" },
            { "pattern": "records processed" }
        ]
    }
}
```

- Patterns are tried as regex first, fall back to substring match
- If the pattern is NOT found, the execution flips from `succeeded` to `failed`
- The failure message is appended to stderr as `[assertion failed] ...`
- Custom `message` is optional — if omitted, a default message is generated
- Combine with `on_assertion_failure` notifications to get alerted when assertions trip

**Use cases:**
- Health check must output "OK" — fail if it doesn't
- ETL must report "records processed" — catch silent failures
- Backup must confirm "completed successfully"
- API response must contain expected JSON fields

### Output Diff

The execution detail modal includes a **Compare** button that lets you select a previous execution and see a side-by-side diff of the output. Useful for:

- Detecting configuration drift in periodic checks
- Spotting regressions in test output
- Monitoring changes in API responses or system state

## Workflow Patterns

### Pattern 1: Failure Alert

A job monitors for failed executions and sends a Slack notification:

```bash
# Step 1: Any job runs and might fail
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "etl-pipeline",
    "task": {"type": "shell", "command": "./etl.sh"},
    "schedule": {"type": "cron", "value": "0 0 */6 * * *"}
}'

# Step 2: Alert job fires when any execution fails
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "failure-alert",
    "task": {"type": "script", "script_name": "slack-alert"},
    "schedule": {"type": "event", "value": {
        "kind_pattern": "execution.completed",
        "severity": "error"
    }}
}'
```

### Pattern 2: Output-Driven Escalation

A health check runs periodically. If its output contains "CRITICAL", a remediation job fires:

```bash
# Step 1: Health check with output trigger
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "health-check",
    "task": {"type": "http", "method": "get", "url": "https://api.example.com/health"},
    "schedule": {"type": "cron", "value": "0 * * * * *"},
    "output_rules": {
        "extractions": [
            {"name": "status", "pattern": "$.status", "type": "jsonpath"},
            {"name": "latency", "pattern": "$.latency_ms", "type": "jsonpath"}
        ],
        "triggers": [
            {"pattern": "CRITICAL|DOWN|unavailable", "severity": "error"},
            {"pattern": "degraded|slow", "severity": "warning"}
        ]
    }
}'

# Step 2: Remediation fires on critical output
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "restart-service",
    "task": {"type": "shell", "command": "systemctl restart myapp"},
    "schedule": {"type": "event", "value": {
        "kind_pattern": "output.matched",
        "severity": "error",
        "job_name_filter": "health-check"
    }}
}'
```

**Flow**: health-check runs every minute → output contains "CRITICAL" → `output.matched` event emitted → restart-service triggers automatically.

### Pattern 3: ETL Pipeline with Dependencies

Three jobs form a pipeline where each step depends on the previous:

```bash
# Step 1: Extract
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "extract",
    "task": {"type": "shell", "command": "./extract.sh"},
    "schedule": {"type": "cron", "value": "0 0 2 * * *"}
}'

# Step 2: Transform (only if extract succeeded in last 2 hours)
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "transform",
    "task": {"type": "shell", "command": "./transform.sh"},
    "schedule": {"type": "cron", "value": "0 0 3 * * *"},
    "depends_on": [{"job_id": "<extract-id>", "within_secs": 7200}]
}'

# Step 3: Load (only if transform succeeded)
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "load",
    "task": {"type": "shell", "command": "./load.sh"},
    "schedule": {"type": "cron", "value": "0 0 4 * * *"},
    "depends_on": [{"job_id": "<transform-id>", "within_secs": 7200}]
}'
```

If extract fails at 2am, transform skips at 3am (dependency not satisfied), and load skips at 4am. The dependency map page visualizes this chain.

To force a blocked job to run once without waiting for dependencies, trigger it with `?skip_deps=true`:

```bash
curl -X POST "http://localhost:8080/api/jobs/<transform-id>/trigger?skip_deps=true"
```

Or click the "waiting" badge in the UI and use **Run Anyway**. This is a one-time override — future scheduled runs still check dependencies normally.

### Pattern 4: Fan-Out to Agents

Deploy to all production servers simultaneously:

```bash
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "deploy-prod",
    "task": {"type": "shell", "command": "/opt/deploy/release.sh"},
    "schedule": {"type": "on_demand"},
    "target": {"type": "tagged", "tag": "production"},
    "output_rules": {
        "triggers": [
            {"pattern": "DEPLOY FAILED", "severity": "error"}
        ]
    }
}'
```

When triggered, this runs on every online agent tagged "production". If any agent's output contains "DEPLOY FAILED", an `output.matched` event fires — which another job could use to trigger a rollback.

### Pattern 5: Custom Agent ML Pipeline

Use a custom Python agent for GPU workloads with UI-defined task types:

1. Start a custom agent on your GPU machine
2. In the dashboard, click the agent card and configure a "train-model" task type with fields: `dataset_url`, `epochs`, `learning_rate`
3. Create a job in Custom Agent mode, fill in the training parameters
4. Add output extractions to capture metrics: `{"name": "accuracy", "pattern": "accuracy: ([\\d.]+)", "type": "regex"}`
5. Add an output trigger for poor results: `{"pattern": "accuracy: 0\\.[0-4]", "severity": "warning"}`

The extracted accuracy value appears in the execution detail. If accuracy drops below 0.5, a warning event fires.

### Pattern 6: Security Audit Chain

React to API key changes with an audit job:

```bash
# Audit fires whenever API keys are created or revoked
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "security-audit",
    "task": {"type": "script", "script_name": "audit-keys"},
    "schedule": {"type": "event", "value": {
        "kind_pattern": "key.*"
    }}
}'
```

The `key.*` pattern matches both `key.created` and `key.revoked` events.

### Pattern 7: Assertion-Based Monitoring

Use assertions to detect silent failures where the command succeeds (exit code 0) but the output is wrong:

```bash
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "check-api",
    "task": {"type": "http", "method": "get", "url": "https://api.example.com/status"},
    "schedule": {"type": "cron", "value": "0 * * * * *"},
    "output_rules": {
        "assertions": [
            {"pattern": "\"healthy\":true", "message": "API not reporting healthy"}
        ]
    },
    "notifications": {
        "on_assertion_failure": true
    }
}'
```

The HTTP request returns 200 (success) but if the response body doesn't contain `"healthy":true`, the execution is failed and a notification is sent.

### Pattern 8: Self-Healing Infrastructure

Combine agent monitoring with automatic remediation:

```bash
# When an agent goes offline, run a diagnostic
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "agent-diagnostic",
    "task": {"type": "shell", "command": "/opt/scripts/check-agent.sh"},
    "schedule": {"type": "event", "value": {
        "kind_pattern": "agent.offline"
    }}
}'

# When a new agent registers, provision it
curl -X POST http://localhost:8080/api/jobs -d '{
    "name": "provision-agent",
    "task": {"type": "shell", "command": "/opt/scripts/provision.sh"},
    "schedule": {"type": "event", "value": {
        "kind_pattern": "agent.registered"
    }},
    "target": {"type": "any"}
}'
```

## How It All Connects

```
┌─────────────┐     cron/oneshot      ┌────────────┐
│  Scheduler  │──────────────────────▶│  Execute   │
└─────────────┘                       │   Job      │
                                      └─────┬──────┘
┌─────────────┐     POST /trigger           │
│  API / UI   │──────────────────────▶      │
└─────────────┘                             │
                                            ▼
                                   ┌────────────────┐
                                   │  Output Rules  │
                                   │  (extraction   │
                                   │   + triggers)  │
                                   └───────┬────────┘
                                           │
                              ┌────────────┼────────────┐
                              │            │            │
                              ▼            ▼            ▼
                     ┌──────────┐  ┌────────────┐  ┌──────────┐
                     │ Extracted │  │  output.   │  │execution.│
                     │  Values  │  │  matched   │  │completed │
                     │ (stored) │  │  (event)   │  │ (event)  │
                     └──────────┘  └─────┬──────┘  └────┬─────┘
                                         │              │
                                         ▼              ▼
                                   ┌────────────────────────┐
                                   │  Event-Triggered Jobs  │
                                   │  (match kind_pattern,  │
                                   │   severity, job_name)  │
                                   └───────────┬────────────┘
                                               │
                                               ▼
                                        Execute again...
                                        (cycle continues)
```

Every execution generates events. Events can trigger more jobs. Those jobs produce output that can trigger more events. This creates a powerful reactive system where you define the rules and Kronforce handles the orchestration.

## Approval Gates

Jobs with `approval_required: true` create a `pending_approval` execution when triggered instead of running immediately. This enables change-control workflows:

1. **Developer triggers deploy-prod** → Execution created with status `pending_approval`
2. **Notification sent** → Team lead sees pending execution in the Events feed
3. **Team lead approves** → `POST /api/executions/{id}/approve` triggers the actual execution
4. **Job runs** → Normal execution lifecycle from here

Approval works with all trigger sources: manual, cron, event-driven, and dependency-based. Unapproved executions remain in `pending_approval` status indefinitely until approved or cancelled.

## Tips

- **Use `job_name_filter`** on event triggers to scope reactions to specific jobs, otherwise `execution.completed` fires for *every* job
- **Combine dependencies + events**: A job can have both `depends_on` (must succeed first) and an event schedule (only fires on specific events)
- **Output extractions are cumulative**: Each run stores its own extracted values. Use the Compare button to diff output between runs
- **Event triggers show in the Map**: Jobs connected by event triggers (with `job_name_filter`) appear as dashed lines in the dependency map
- **`output.matched` includes the job name** in its message, so downstream event triggers can use `job_name_filter` to react to specific sources
- **Custom agents + output rules**: Custom agent task output is processed the same way — extractions and triggers work regardless of where the job executes
