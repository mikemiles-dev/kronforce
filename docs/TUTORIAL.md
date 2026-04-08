# Tutorial: End-to-End Setup

This walks through a complete Kronforce setup from scratch — controller on one machine, agent on another, jobs that chain together, notifications when things break. By the end you'll have a working pipeline running across two machines.

## What You'll Build

- A **controller** running on a VM with the dashboard and scheduler
- A **remote agent** on a second machine executing jobs
- A **3-job pipeline**: extract data → transform it → load it, with dependencies so each step waits for the previous one
- **Notifications** to Slack when something fails
- A **health check** job that monitors an endpoint every minute

## Step 1: Start the Controller

SSH into your controller machine (a VM, cloud instance, or even your laptop).

**Option A — Download the binary:**

```bash
VERSION=$(curl -s https://api.github.com/repos/mikemiles-dev/kronforce/releases/latest | grep tag_name | cut -d'"' -f4)
curl -L -o kronforce https://github.com/mikemiles-dev/kronforce/releases/download/$VERSION/kronforce-linux-amd64
chmod +x kronforce
./kronforce
```

**Option B — Docker:**

```bash
docker run -d \
  --name kronforce \
  -p 8080:8080 \
  -v kronforce-data:/data \
  -e KRONFORCE_DB=/data/kronforce.db \
  ghcr.io/mikemiles-dev/kronforce:latest
```

On first startup you'll see output like:

```
=== Bootstrap API Keys ===
Admin key:  kf_abc123...
Agent key:  kf_xyz789...
===========================
```

**Save both keys.** The admin key is your login to the dashboard. The agent key is what remote agents use to connect.

Open `http://<controller-ip>:8080` in your browser and log in with the admin key.

## Step 2: Create a Health Check Job

Let's start simple — a job that pings a URL every minute.

**In the dashboard:**

1. Go to **Jobs** → click **+ Create**
2. Fill in:
   - **Name**: `health-check`
   - **Task type**: HTTP
   - **URL**: `https://httpbin.org/status/200` (or your own endpoint)
   - **Method**: GET
   - **Schedule**: Cron → `0 * * * * *` (every minute)
3. Click **Save**

The job appears in the list. Within a minute it fires automatically. Click it to see the execution output — you should see a 200 response.

**Or via the API:**

```bash
ADMIN_KEY="kf_abc123..."  # your admin key from step 1

curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "health-check",
    "group": "Monitoring",
    "task": {
      "type": "http",
      "url": "https://httpbin.org/status/200",
      "method": "GET"
    },
    "schedule": {"type": "cron", "value": "0 * * * * *"}
  }'
```

## Step 3: Deploy a Remote Agent

SSH into your second machine (a different VM, server, or container).

```bash
VERSION=$(curl -s https://api.github.com/repos/mikemiles-dev/kronforce/releases/latest | grep tag_name | cut -d'"' -f4)
curl -L -o kronforce-agent https://github.com/mikemiles-dev/kronforce/releases/download/$VERSION/kronforce-agent-linux-amd64
chmod +x kronforce-agent

KRONFORCE_AGENT_KEY=kf_xyz789... \
KRONFORCE_CONTROLLER_URL=http://<controller-ip>:8080 \
KRONFORCE_AGENT_NAME=worker-1 \
KRONFORCE_AGENT_TAGS=linux,etl \
  ./kronforce-agent
```

You should see:

```
registered with controller as worker-1 (agent_id: ...)
heartbeat: ok
```

Back in the dashboard, go to the **Agents** page — you'll see `worker-1` show up as online. The agent is now polling for work.

**Docker option for the agent:**

```bash
docker run -d \
  --name kronforce-agent \
  -e KRONFORCE_AGENT_KEY=kf_xyz789... \
  -e KRONFORCE_CONTROLLER_URL=http://<controller-ip>:8080 \
  -e KRONFORCE_AGENT_NAME=worker-1 \
  -e KRONFORCE_AGENT_TAGS=linux,etl \
  ghcr.io/mikemiles-dev/kronforce:latest \
  kronforce-agent
```

## Step 4: Build a 3-Job Pipeline

Now let's create an ETL pipeline where each job depends on the previous one finishing successfully. All three jobs will run on the remote agent.

### Job 1: Extract

```bash
curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "etl-extract",
    "group": "ETL",
    "task": {
      "type": "shell",
      "command": "echo \"extracted 1523 records from source\" && echo \"RECORD_COUNT=1523\""
    },
    "schedule": {"type": "cron", "value": "0 0 2 * * *"},
    "target": {"type": "tagged", "tag": "etl"},
    "output_rules": {
      "extractions": [
        {"name": "record_count", "pattern": "RECORD_COUNT=(\\d+)", "target": "variable"}
      ]
    }
  }'
```

Save the returned `id` — you'll need it for job 2.

### Job 2: Transform (depends on Extract)

```bash
EXTRACT_ID="<id from job 1>"

curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "etl-transform",
    "group": "ETL",
    "task": {
      "type": "shell",
      "command": "echo \"transforming {{record_count}} records\" && echo \"TRANSFORM_STATUS=complete\""
    },
    "schedule": {"type": "cron", "value": "0 0 3 * * *"},
    "target": {"type": "tagged", "tag": "etl"},
    "depends_on": [{"job_id": "'$EXTRACT_ID'", "within_secs": 7200}],
    "output_rules": {
      "extractions": [
        {"name": "transform_status", "pattern": "TRANSFORM_STATUS=(\\w+)", "target": "variable"}
      ]
    }
  }'
```

The `depends_on` means: only run if `etl-extract` succeeded within the last 2 hours. The `{{record_count}}` substitutes the value extracted by the previous job.

Save this job's `id` too.

### Job 3: Load (depends on Transform)

```bash
TRANSFORM_ID="<id from job 2>"

curl -X POST http://localhost:8080/api/jobs \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "name": "etl-load",
    "group": "ETL",
    "task": {
      "type": "shell",
      "command": "echo \"loading data (transform={{transform_status}})\" && echo \"done\""
    },
    "schedule": {"type": "cron", "value": "0 0 4 * * *"},
    "target": {"type": "tagged", "tag": "etl"},
    "depends_on": [{"job_id": "'$TRANSFORM_ID'", "within_secs": 7200}]
  }'
```

### See It in Action

Go to **Jobs** → **Map** tab. You'll see the three jobs connected as a chain: extract → transform → load.

Switch to the **Groups** tab and select the "ETL" group to see them organized together.

**Test the pipeline now** — don't wait for the cron schedule:

1. Go to the **Jobs** tab, find `etl-extract`, click the play button to trigger it
2. Watch it execute on `worker-1` (check the execution detail — the Agent field shows your remote agent)
3. Once extract succeeds, trigger `etl-transform` — it runs because its dependency is satisfied
4. Trigger `etl-load` — same thing, it chains through

**What if a dependency isn't met?** If you try to trigger `etl-load` without transform having succeeded recently, it shows as "waiting" in the UI. Click the waiting badge to see which dependencies are blocking it, then click **Run Anyway** to force it through for this one run.

## Step 5: Set Up Slack Notifications

1. In the dashboard, go to **Settings** → **Notifications**
2. Set the **Slack Webhook URL** (create one at [api.slack.com/messaging/webhooks](https://api.slack.com/messaging/webhooks))
3. Click **Test** to verify it works
4. Click **Save**

Now edit `etl-extract` and under the **Alerts** tab:
- Check **Notify on failure**
- Optionally set a Slack channel override

When the job fails, you'll get a Slack message with the job name, exit code, and a link to the execution.

## Step 6: Add an Approval Gate

For the load step — since it writes to production — let's require approval:

```bash
curl -X PUT http://localhost:8080/api/jobs/$LOAD_ID \
  -H "Authorization: Bearer $ADMIN_KEY" \
  -H "Content-Type: application/json" \
  -d '{"approval_required": true}'
```

Or in the UI: edit `etl-load` → **Advanced** tab → toggle **Require Approval**.

Now when `etl-load` triggers (manually or by schedule), it creates a `pending_approval` execution instead of running immediately. An admin or operator must click **Approve** in the execution detail modal before it runs.

## Step 7: Monitor with the Dashboard

Here's what to look at:

- **Dashboard → Overview**: Donut charts show execution outcomes, task types, and schedule distribution
- **Dashboard → Activity**: Timeline of all events — triggers, completions, failures, agent heartbeats
- **Jobs → Map**: Visual dependency graph of your entire system
- **Jobs → Stages**: Pipeline view grouped by job group — see the ETL pipeline as a stage progression

### Prometheus Integration

If you run Prometheus, scrape the `/metrics` endpoint:

```yaml
# prometheus.yml
scrape_configs:
  - job_name: kronforce
    static_configs:
      - targets: ['<controller-ip>:8080']
```

Available metrics: `kronforce_executions_total`, `kronforce_jobs_total`, `kronforce_agents_online`, `kronforce_db_size_bytes`, and more.

## What You've Got

At this point you have:

| Component | Where | What It Does |
|---|---|---|
| Controller | VM #1 | Scheduler, API, dashboard, SQLite database |
| Agent `worker-1` | VM #2 | Executes shell tasks, tagged `etl` |
| `health-check` | Runs on controller | Pings a URL every minute |
| `etl-extract` | Runs on `worker-1` | Extracts data, writes record count to a variable |
| `etl-transform` | Runs on `worker-1` | Waits for extract, reads the variable, transforms |
| `etl-load` | Runs on `worker-1` | Waits for transform, requires approval before running |
| Slack alerts | — | Notifies on failure |

## Next Steps

- **Add more agents** — scale horizontally by deploying agents on more machines. Use tags to route jobs.
- **Custom agents** — write a Python/Go/Node agent for specialized task types. See [Custom Agents](CUSTOM_AGENTS.md).
- **OIDC/SSO** — replace API key login with your identity provider. See the [README](../README.md#configuration).
- **Event triggers** — react to failures, output patterns, or agent state changes. See [Triggers & Workflows](TRIGGERS_AND_WORKFLOWS.md).
- **Rhai scripting** — write inline scripts for complex logic without shell. See the in-app Docs page.
- **HA setup** — replicate SQLite to S3 with Litestream for failover. See [Deployment](DEPLOYMENT.md#high-availability-with-litestream).
