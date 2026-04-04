# Migration Guide

Moving your existing scheduled tasks to Kronforce from cron, Rundeck, or Airflow.

## From Cron

### Automatic Import

Kronforce can import your crontab directly:

```bash
# Import from the current user's crontab
crontab -l | kronforce-import-crontab kf_your_admin_key

# Import from a file
kronforce-import-crontab kf_your_admin_key < /etc/cron.d/my-jobs

# Import to a specific group
kronforce-import-crontab kf_your_admin_key --group Monitoring

# Import to a remote Kronforce instance
KRONFORCE_URL=http://kronforce:8080 kronforce-import-crontab kf_your_admin_key < mycrontab
```

The importer creates a Kronforce job for each cron entry with the same schedule and command. Jobs are created in the "Default" group (or the group you specify) with status "Scheduled".

### Manual Migration

Each cron line maps directly to a Kronforce shell job:

**Cron:**
```
*/5 * * * * /usr/local/bin/check-disk.sh
0 3 * * * /opt/backup/run-backup.sh >> /var/log/backup.log 2>&1
30 */2 * * 1-5 curl -sf https://api.example.com/health
```

**Kronforce equivalent:**

| Cron Expression | Kronforce Cron | Notes |
|---|---|---|
| `*/5 * * * *` | `0 */5 * * * *` | Add leading `0` for seconds field |
| `0 3 * * *` | `0 0 3 * * *` | Kronforce has 6 fields (seconds first) |
| `30 */2 * * 1-5` | `0 30 */2 * * 1-5` | Day-of-week: 0=Sun in both |

**Key differences:**
- Kronforce uses **6-field cron** (seconds, minutes, hours, day-of-month, month, day-of-week)
- Standard cron uses 5 fields (no seconds). Prepend `0` to keep the same schedule.
- Output is captured automatically — no need for `>> logfile 2>&1`
- Environment variables use `{{VAR_NAME}}` syntax instead of shell `$VAR`

### What You Gain

| Cron | Kronforce |
|---|---|
| No dashboard | Visual dashboard with execution history |
| No alerts on failure | Slack, email, PagerDuty notifications |
| No output capture | Full stdout/stderr with search and diff |
| No retry | Automatic retry with exponential backoff |
| No dependencies | Job dependency DAG with time windows |
| Scattered across machines | Centralized with distributed agents |
| No audit trail | Full audit log of all changes |

## From Rundeck

Rundeck jobs are typically exported as XML or YAML. There's no automated import tool (Rundeck's format varies significantly by version and plugin set), but the mapping is straightforward.

### Job Mapping

| Rundeck | Kronforce | Notes |
|---|---|---|
| Job name | `name` | Direct mapping |
| Job group | `group` | Kronforce has flat groups (no nesting) |
| Description | `description` | Direct mapping |
| Schedule (cron) | `schedule.type: "cron"` | Add seconds field (prepend `0`) |
| Node filter | `target` | Use tags: `{"type": "tagged", "tag": "linux"}` |
| Script step | `task.type: "shell"` | Paste the command |
| HTTP step | `task.type: "http"` | URL, method, headers, body |
| Script file step | `task.type: "shell"` | Or use Rhai scripting |
| Notification (email) | `notifications.on_failure: true` | Configure SMTP in Settings |
| Notification (webhook) | Webhook channel in Settings | Slack, Teams, PagerDuty |
| Key Storage | Secret variables | `{{DB_PASSWORD}}` with `secret: true` |
| ACL Policies | API key roles + group scoping | 4 roles, per-group access |

### Step-by-Step

1. **Export your Rundeck jobs** — `rd jobs list -p myproject -f yaml > jobs.yaml`
2. **For each job:**
   - Create in Kronforce UI or API
   - Copy the command/script
   - Convert the cron schedule (add seconds field)
   - Set the target (agents replace Rundeck nodes)
   - Configure notifications
3. **Migrate secrets** — recreate as secret variables in Kronforce
4. **Set up agents** — deploy Kronforce agents on the same machines Rundeck was targeting
5. **Test** — trigger each job manually before enabling schedules

### What You Gain

| Rundeck | Kronforce |
|---|---|
| Java + database + web server | Single binary, zero dependencies |
| Plugin ecosystem (complex) | Custom agents in any language (simple) |
| Enterprise license for SSO | OIDC/SSO included free |
| XML/YAML job definitions | JSON API + visual editor |
| Node discovery required | Tag-based agent targeting |

## From Airflow

Airflow DAGs are Python code, so there's no automated converter. However, most Airflow usage falls into patterns that map cleanly to Kronforce.

### Concept Mapping

| Airflow | Kronforce | Notes |
|---|---|---|
| DAG | Job group + dependencies | Kronforce uses flat jobs with dependency chains |
| Task | Job | One Kronforce job per Airflow task |
| BashOperator | Shell task | `{"type": "shell", "command": "..."}` |
| PythonOperator | Shell task or Rhai script | `python3 -c "..."` or custom agent |
| HttpOperator | HTTP task | `{"type": "http", "method": "GET", "url": "..."}` |
| PostgresOperator | SQL task | `{"type": "sql", "driver": "postgres", ...}` |
| EmailOperator | Job notification | `notifications.on_success: true` |
| Sensor | Event-triggered job | Schedule type "event" with pattern matching |
| XCom (cross-task data) | Variables + output extraction | Extract from stdout, pass via `{{VAR}}` |
| Schedule interval | Cron schedule | 6-field cron with visual builder |
| `depends_on_past` | Job dependencies | `depends_on` with time window |
| Pool | Job priority | Higher priority jobs run first |
| Connection | Secret variables | Masked credentials |
| RBAC | API key roles + OIDC | 4 roles with group scoping |

### Migration Pattern

**Airflow DAG:**
```python
with DAG('etl_pipeline', schedule_interval='0 6 * * *'):
    extract = BashOperator(task_id='extract', bash_command='python3 extract.py')
    transform = BashOperator(task_id='transform', bash_command='python3 transform.py')
    load = BashOperator(task_id='load', bash_command='python3 load.py')
    extract >> transform >> load
```

**Kronforce equivalent:**

1. Create group "ETL"
2. Create job `etl-extract`:
   - Task: Shell, command: `python3 extract.py`
   - Schedule: Cron `0 0 6 * * *`
   - Group: ETL
3. Create job `etl-transform`:
   - Task: Shell, command: `python3 transform.py`
   - Schedule: On Demand
   - Dependencies: `etl-extract` (succeeded within 1 hour)
   - Group: ETL
4. Create job `etl-load`:
   - Task: Shell, command: `python3 load.py`
   - Schedule: On Demand
   - Dependencies: `etl-transform` (succeeded within 1 hour)
   - Group: ETL

### Passing Data Between Jobs (XCom Replacement)

**Airflow:**
```python
def extract(**context):
    count = run_query()
    context['ti'].xcom_push(key='count', value=count)

def transform(**context):
    count = context['ti'].xcom_pull(key='count')
```

**Kronforce:**
1. `etl-extract` outputs: `Extracted 42 records`
2. Add extraction rule: pattern `Extracted (\d+)`, write to variable `RECORD_COUNT`
3. `etl-transform` command: `python3 transform.py --count {{RECORD_COUNT}}`

### What You Gain

| Airflow | Kronforce |
|---|---|
| Python + Postgres + Redis + webserver + scheduler | Single binary |
| DAGs defined in Python code | Visual UI + REST API |
| Requires Python expertise | Any language via custom agents |
| Complex deployment (Kubernetes/Celery) | Download and run |
| Heavyweight for simple jobs | Lightweight for any scale |

### What You Lose

| Airflow Advantage | Kronforce Alternative |
|---|---|
| Rich Python operator ecosystem | 12 built-in task types + custom agents |
| Built-in data lineage | Output extraction + event triggers |
| Kubernetes executor | Agent-based distributed execution |
| Complex DAG branching | Dependencies + event triggers |
| Connection management UI | Secret variables via API/UI |

## Tips for Any Migration

1. **Start small** — migrate a few non-critical jobs first
2. **Run in parallel** — keep old scheduler running while you validate Kronforce
3. **Use the seed script** — `./data/test/seed.sh` loads example jobs to learn the patterns
4. **Check the Getting Started page** — in-app guide walks through every feature
5. **Set up notifications first** — so you know immediately if a migrated job fails
6. **Use groups** — organize migrated jobs by source system (e.g., "Migrated-Cron", "Migrated-Rundeck")
7. **Enable audit log** — track all changes during migration for rollback reference
