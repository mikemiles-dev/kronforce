# Extending Kronforce

Kronforce doesn't use a plugin system. Instead, it provides composable primitives — task types, custom agents, scripting, output rules, connections, and a full REST API — that cover most automation use cases without installing anything.

## Jenkins Plugin Equivalents

| Jenkins Plugin | Kronforce Equivalent | How |
|---|---|---|
| Git / SCM | Shell task | `git clone`, `git pull` in a shell command |
| Slack Notification | Built-in Slack | Settings → Notifications → Slack webhook URL |
| Email Extension | Built-in SMTP | Settings → Notifications → SMTP config |
| PagerDuty | Built-in PagerDuty | Settings → Notifications → routing key |
| Microsoft Teams | Built-in Teams | Settings → Notifications → Teams webhook |
| Docker Pipeline | Shell + Dockerfile scripts | Shell task with `docker build/run`, or Dockerfile script type |
| HTTP Request | HTTP task | GET/POST/PUT/DELETE with headers, body, auth, assertions |
| Database (Postgres, MySQL) | SQL task + Connections | Named connection + SQL query |
| SSH / SSH Agent | SSH connection + Shell | Named SSH connection, or shell task on remote agent |
| Credentials Binding | Secret variables + Connections | `{{MY_SECRET}}` substituted at runtime, AES-256-GCM encrypted at rest |
| Pipeline DSL | Dependencies + Events | Job B depends on Job A; event triggers react to completions |
| Parameterized Build | Job parameters | Define in job config, pass at trigger time via API or UI |
| Cron Trigger | Cron schedule | 6-field cron expression on any job |
| Webhook Trigger | Built-in webhooks | Enable on any job, trigger via unique URL |
| Approval Gate | Built-in approval | `approval_required: true` on any job |
| Retry / Naginator | Built-in retry | `retry_max` + `retry_delay_secs` per job |
| Build Timeout | Built-in timeout | `timeout_secs` per job |
| Prometheus Metrics | Built-in `/metrics` | Scrape endpoint with job/execution/agent counters |
| LDAP / SAML / OIDC | Built-in OIDC | Okta, Azure AD, Google via OIDC config |
| Custom Build Step | Custom agent | Python/Go/Node agent for any task type |
| gRPC / Protobuf | Custom agent | gRPC agent example with `grpcurl` |
| ML / GPU Workloads | Custom agent | Agent on GPU machine with `train-model` task type |
| Kubernetes | Shell task or custom agent | `kubectl apply` in shell, or K8s-native custom agent |
| Terraform / Ansible | Shell task | `terraform apply`, `ansible-playbook` as shell commands |
| S3 / Artifact Upload | S3 connection + Shell | Named S3/MinIO connection + `aws s3 cp` |
| FTP / SFTP | File push task + Connection | Named FTP/SFTP connection, or `file_push` task type |
| MQTT / Kafka / RabbitMQ | Connection + Custom agent | Named connection for config, custom agent for complex consumers |
| Redis | Connection + Shell/Custom | Named Redis connection + CLI or custom agent |
| MongoDB | Connection + Custom agent | Named MongoDB connection + custom agent |

## Extension Mechanisms

### 1. Custom Agents (Any Language)

The primary extension point. Write a custom agent in Python, Go, Node, Rust, or anything with an HTTP client:

1. Register with controller (`POST /api/agents/register`)
2. Long-poll for work (`GET /api/agent-queue/{id}/next?wait=30`)
3. Execute the task however you want
4. Post results back (`POST {callback_url}`)

Custom agents define **task types** with UI-configurable fields. Example: a Python ML agent defines `train-model` with fields for dataset URL, model architecture, and hyperparameters. These appear as form fields in the Builder.

See [Custom Agents](CUSTOM_AGENTS.md) for the full protocol and examples.

### 2. Rhai Scripts (Inline Logic)

For lightweight automation that doesn't need an external agent:

- Sandboxed scripting with HTTP, JSON, math, string functions
- Runs on the controller — no agent setup
- Use cases: health checks, API polling, data validation, conditional workflows

See the in-app Docs → Scripting section for the API reference.

### 3. Output Rules (Reactive Workflows)

Jobs can process their own output to drive automation:

- **Extraction** — capture regex matches or JSON paths from stdout into variables
- **Assertions** — fail a job if output doesn't match expected patterns
- **Triggers** — fire other jobs based on output content
- **Forward** — POST output to an external URL (webhook relay)

Combined with event triggers and dependencies, these compose into complex workflows without plugins. See [Triggers & Workflows](TRIGGERS_AND_WORKFLOWS.md).

### 4. REST API (Build Anything)

Every feature is accessible via the [REST API](API.md):

- Full CRUD on jobs, executions, variables, connections, scripts, agents
- Pagination, filtering, search across execution output
- Real-time streaming (SSE) for live execution output
- Data export/import for backup and migration
- Prometheus metrics for monitoring

Build custom dashboards, CLI tools, Slack bots, or integrations with any system that speaks HTTP.

### 5. MCP Server (AI Integration)

Built-in [Model Context Protocol](https://modelcontextprotocol.io/) server exposes jobs, executions, and agents as tools for AI assistants (Claude, GPT, etc.). Enable with `KRONFORCE_MCP=true`.

### 6. Webhooks (Inbound + Outbound)

- **Inbound**: Enable a webhook URL on any job to trigger it from external systems (GitHub, GitLab, CI/CD, etc.)
- **Outbound**: Use output rule "forward" to POST execution results to external URLs

## When to Use What

| Need | Best Approach |
|---|---|
| Run a CLI tool | Shell task |
| Call an API | HTTP task |
| Query a database | SQL task + named connection |
| Custom protocol/logic | Custom agent |
| Multi-step HTTP workflow | Rhai script |
| React to job output | Output extraction + event trigger |
| CI/CD webhook trigger | Job webhook URL |
| AI-driven automation | MCP server |
| Custom dashboard | REST API |
| Monitoring integration | Prometheus `/metrics` |
