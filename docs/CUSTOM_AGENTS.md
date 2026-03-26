# Custom Agents

Custom agents use a **pull-based** model — they poll the controller for work, execute it however they want, and post the result back. Build agents in any language with just an HTTP client.

## How It Works

Custom agents separate **workflow definition** from **implementation**:

- **The UI defines the workflow** — admins configure task types (name, description, form fields) on the agent card in the dashboard
- **The agent code defines the implementation** — the agent handles whatever task data it receives

This means you can reconfigure task types without restarting the agent, and non-developers can set up workflows through the dashboard.

## Quick Start

1. Start the controller: `cargo run --bin kronforce`
2. Start the example agent: `python3 examples/custom_agent.py`
3. In the dashboard, go to **Agents** and click the custom agent card
4. Add task types (e.g., "python" with a "script" textarea field) and click **Save**
5. Create a job → select **Custom Agent** mode → pick your agent → fill in the form → trigger

## Protocol

All agent endpoints require an API key with the `agent` role (or `admin`). On first startup, a bootstrap agent key is created and printed to the console. Set it as `KRONFORCE_AGENT_KEY` on your agents. Create additional agent keys in Settings.

### 1. Register

```bash
curl -X POST http://localhost:8080/api/agents/register \
  -H "Authorization: Bearer kf_your_agent_key" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "my-python-agent",
    "tags": ["python", "ml"],
    "hostname": "ml-box",
    "address": "0.0.0.0",
    "port": 0,
    "agent_type": "custom"
  }'
# Returns: {"agent_id": "uuid", "heartbeat_interval_secs": 10}
```

### 2. Discover Task Types (optional)

```bash
curl http://localhost:8080/api/agents/{agent_id}/task-types
# Returns: [{"name": "python", "fields": [...]}, ...]
```

Useful for logging on startup or validating that your agent handles the configured types.

### 3. Poll for Work

```bash
curl http://localhost:8080/api/agent-queue/{agent_id}/next
# Returns 204 if no work, or 200 with:
```

```json
{
    "queue_id": "uuid",
    "execution_id": "uuid",
    "job_id": "uuid",
    "agent_id": "uuid",
    "task": {
        "type": "custom",
        "agent_task_type": "python",
        "data": {"script": "print('hello')", "args": "--verbose"}
    },
    "callback_url": "http://controller:8080/api/callbacks/execution-result"
}
```

Your agent switches on `task.agent_task_type` and reads field values from `task.data`.

### 4. Report Result

```bash
curl -X POST {callback_url} \
  -H "Content-Type: application/json" \
  -d '{
    "execution_id": "...",
    "job_id": "...",
    "agent_id": "...",
    "status": "succeeded",
    "exit_code": 0,
    "stdout": "output here",
    "stderr": "",
    "stdout_truncated": false,
    "stderr_truncated": false,
    "started_at": "2026-03-25T10:00:00Z",
    "finished_at": "2026-03-25T10:00:05Z"
  }'
```

## Task Type Definitions

Configured per-agent in the UI (Agents page → click custom agent card). Each definition:

```json
{
    "name": "python",
    "description": "Run a Python script",
    "fields": [
        {"name": "script", "label": "Script", "field_type": "textarea", "required": true, "placeholder": "print('hello')"},
        {"name": "args", "label": "Arguments", "field_type": "text", "required": false}
    ]
}
```

**Field types:** `text`, `textarea`, `number`, `select`, `password`

Select fields include `options`: `[{"value": "dev", "label": "Development"}, ...]`

Task types can also be managed via API: `GET/PUT /api/agents/{id}/task-types`

## Queue Behavior

- Jobs are **queued** until the agent polls
- Unclaimed jobs are failed after **5 minutes**
- Claimed but unreported jobs are failed after **10 minutes**
- Polling acts as a heartbeat — no separate call needed
- The UI shows a "queued" badge for pending custom agent jobs

## Building Your Own Agent

Any language with an HTTP client works. The protocol is:

1. `POST /api/agents/register` with `"agent_type": "custom"`
2. `GET /api/agents/{id}/task-types` (optional, for discovery)
3. `GET /api/agent-queue/{id}/next` in a loop (also heartbeats)
4. `POST {callback_url}` with the execution result

```python
task = job["task"]
if task["agent_task_type"] == "python":
    script = task["data"]["script"]
    # Run the script...
elif task["agent_task_type"] == "train-model":
    dataset = task["data"]["dataset_url"]
    # Train the model...
```

## Python Example

A complete working example is at `examples/custom_agent.py`:

```bash
pip install requests

KRONFORCE_URL=http://controller:8080 \
AGENT_NAME=my-agent \
AGENT_TAGS=python,ml \
python3 examples/custom_agent.py
```

The example handles `python` (runs via `python3 -c`), `shell` (via `sh -c`), and `http` task types.

## Output Rules with Custom Agents

Custom agent jobs support the same output rules as standard jobs:

- **Extractions**: Pull values from agent stdout using regex or JSON path
- **Triggers**: Emit `output.matched` events when output matches patterns
- **Diff**: Compare output across custom agent runs in the execution detail

Configure output rules in the job's Advanced section when creating a Custom Agent job. The agent doesn't need to know about the rules — they run on the controller after the result is reported back.

See [Triggers & Workflows](TRIGGERS_AND_WORKFLOWS.md) for patterns combining custom agents with event-driven automation.
