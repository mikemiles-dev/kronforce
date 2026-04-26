# Changelog

All notable changes to Kronforce will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha] - 2026-04-26

### Added
- **UI redesign: 7-tab navigation** — consolidated from 10+ tabs to 7 focused pages: Dashboard, Monitor (Jobs/Runs/Events sub-tabs), Pipelines (Stages/Map), Builder (full-page job editor with sidebar steps), Toolbox (Scripts/Variables/Connections), Settings, and Docs. Color-coded sub-tabs and step navigation. Legacy URL routes redirect automatically.
- **Full-page Builder** — replaces the cramped 620px modal with a full-page job editor. Left sidebar with color-coded steps (Task, Schedule, Target, Rules, Alerts, Advanced), AI assistant panel on the right, sticky Save/Cancel. After save, navigates directly to the job detail page.
- **Variable expiration** — optional TTL on variables (30/90/180/365 days). Expired variables show a red badge in the UI. Useful for rotating secrets and temporary config.
- **API key expiration UI** — dropdown to set key expiry (30/90/180/365 days or never). Expired keys shown in red in the key list.
- **Data export with secrets** — `GET /api/data/export?include_secrets=true` exports decrypted secret variables, connection configs, and API key metadata for backup and migration.
- **Toolbox search** — search/filter for Variables and Connections in the Toolbox page.
- **AI job creation** — describe what you want in natural language, AI generates the full job configuration (name, task, schedule, timeout, notifications). Supports Anthropic Claude and OpenAI GPT. Set `KRONFORCE_AI_API_KEY` to enable. The AI panel sits beside the Builder form; AI never creates jobs directly — you review and save.
- **Named connections** — credential profiles for databases, APIs, and services. 14 protocol types: PostgreSQL, MySQL, SQLite, FTP, SFTP, HTTP (bearer/basic/header auth), Kafka, MQTT, RabbitMQ, Redis, MongoDB, SSH, SMTP, S3/MinIO. Encrypted at rest (AES-256-GCM), masked in API responses, test connectivity from the UI. Jobs reference a connection by name (`"connection": "prod-db"`) instead of embedding passwords. Connection dropdown in job create/edit form. Managed in Toolbox → Connections.
- **Product tour** — first-time user walkthrough with spotlight overlay highlighting each navigation element. Demo mode adds an intro explaining read-only access. Replayable from Settings. Responsive mobile positioning.
- **Docs search and navigation** — search input filters docs sections by content. Mobile gets a search input + "Jump to" section dropdown. Desktop sidebar highlights active section on scroll (scroll-spy).
- **Pipeline scheduling** — set cron or interval schedules on entire pipeline groups. The scheduler automatically triggers root jobs on schedule, and dependencies cascade from there. Configure via the new "Schedule" button on the Stages/Pipeline view, or via the API (`PUT /api/jobs/pipeline-schedule/{group}`). Schedules persist in settings and survive job changes.
- **Pipeline run history** — "History" button on the Stages view shows a modal with clustered pipeline runs, per-job status icons, overall status badge, and total duration. Click any status icon to view the execution detail.
- **Jenkins importer** — `scripts/kronforce-import-jenkins` parses Jenkinsfiles and config.xml into Kronforce jobs. `--pipeline` flag wires stages as dependency chains. Supports bulk import from directories, agent label targeting, retry/timeout extraction, and environment variable import.
- **Executions group filter** — `GET /api/executions?group=ETL` filters executions to only jobs in that group.
- **Demo mode banner** — fixed top banner in demo mode: "You are viewing a read-only demo of Kronforce" with a link to kronforce.dev and a dismiss button.
- **In-app migration docs** — "Migrating to Kronforce" section in the Docs page with crontab and Jenkins importer usage, and Airflow/Rundeck mapping tables.
- **Screenshots** — README hero image with expandable gallery (8 screenshots), website feature gallery (6 screenshots), migration guide pipeline screenshot.
- **Expanded demo seed data** — 35 jobs across 8 groups (ETL, Monitoring, Deploys, Maintenance, Reports, Data-Sync, Security, Notifications), 12 variables, 3 scripts, 5-stage ETL with archive, 4-stage deploys with post-checks and approval gate, 4-stage maintenance with fan-out/fan-in, 5-stage data-sync with parallel roots, calendar-scheduled reports, event-triggered alerts, and 7 rounds of pipeline runs for rich history.
- **Dependency cascade** — when a job succeeds, on-demand jobs that depend on it are automatically triggered if all their dependencies are now satisfied. Enables Jenkins-style pipeline execution: trigger the first job, the rest cascade automatically.
- **Group completion events** — when all jobs in a group have succeeded, a `group.completed` event is emitted. Use event triggers to react (send notification, trigger next pipeline, etc.).
- **Run Group button** — trigger all root jobs in a group with one click. Appears on the toolbar when a group is selected. Dependent jobs cascade from there.
- **Shell working directory** — optional `working_dir` field on shell tasks. Commands run from the specified directory.
- **Multi-type scripts** — Scripts page now supports both Rhai scripts and Dockerfiles with type selector, per-type syntax highlighting, and reference panels. Dockerfile highlighting covers instructions (FROM, RUN, COPY, etc.), strings, variables, and flags.
- **Docker Build task type** — build Docker images from stored Dockerfile scripts. Configure image tag, build args, and optionally run the container after build. 17 total task types.
- **Calendar schedule** — new schedule type for business-day expressions: "last day of month", "2nd Tuesday", "first Friday - 2 days", with month selection, time, offset, weekend skipping, and holiday exclusion. Visual builder in the Schedule tab with live preview.
- **Interval schedule** — "fixed interval from last completion" schedule type. Run again N seconds after the previous execution finishes. Prevents overlap for variable-duration jobs.
- **Timezone support** — optional `timezone` field on jobs (IANA format, e.g. "America/New_York") for timezone-aware schedule evaluation.
- **Message queue consumers** — 4 new consume/subscribe task types: Kafka Read, MQTT Subscribe, RabbitMQ Read, Redis Read. Consume messages from queues with configurable max message count, offsets, and timeouts. Messages appear in stdout for output extraction processing. Total task types: 16.
- **Concurrency controls** — `max_concurrent` field on jobs (default 0 = unlimited). Scheduler skips firing if the job already has that many running executions. Prevents overlapping cron runs.
- **Parameterized runs** — jobs can define parameter schemas (name, type, required, default). Trigger accepts runtime `params` in request body. Use `{{params.NAME}}` in task fields for substitution. UI shows a parameter form when triggering parameterized jobs.
- **Webhook triggers** — unique token-based URLs per job (`POST /api/webhooks/{token}`) that trigger without API key auth. Enable/disable via `POST/DELETE /api/jobs/{id}/webhook`. Accepts optional params in body. Copy-able URL in job detail view.
- **Live output streaming** — SSE endpoint (`GET /api/executions/{id}/stream`) streams stdout/stderr line-by-line during local execution. Execution detail modal auto-connects when viewing a running job, with auto-scroll and automatic refresh on completion.
- **Schedule window** — optional `starts_at` and `expires_at` fields on jobs to constrain when a schedule is active. "Run for 3 weeks then stop", "start next Monday", or any fixed window. Expired jobs auto-unschedule.
- **Skip dependencies on trigger** — manually trigger a blocked job with `?skip_deps=true` to bypass dependency checks for a single run. UI adds a "Run Anyway" button in the waiting-on-dependencies modal.
- **End-to-end tutorial** — new `docs/TUTORIAL.md` walks through controller setup, agent deployment, building a pipeline, and notifications
- **Cron builder rebuild** — full per-field controls for all 6 cron fields (second, minute, hour, day-of-month, month, day-of-week) with every/fixed/step/range modes
- **Group picker popover** — replaced group dropdown with searchable popover including inline group creation and deletion
- **Running/Failed job filters** — new filter buttons on jobs page to quickly find running or failed jobs
- **Clickable last run status** — click the last run indicator in jobs table to open execution details; click failed count to jump to history
- **Job detail action buttons** — Pause/Resume, Delete, and Edit buttons now appear on the job detail view alongside Trigger
- **Edit/Stop buttons in jobs table** — edit button opens job modal, stop button cancels running execution
- **Recent events on job detail** — Overview tab now shows the last 15 events for the job with severity, timestamp, kind, and message
- **Smart empty states** — filtered views show "No matching results" with clear-filters button instead of "Create a job" when filters return nothing
- **Agent connection command box** — agents page shows a copy-able connection command with real controller URL, binary/docker tabs
- **JS test framework** — CI now runs JavaScript unit tests for cron builder, formatting, and empty state logic

### Changed
- **Stage card status improvements** — pipeline stage cards now handle all execution states: pending re-runs show green with a re-run icon (instead of flashing to idle), cancelled and skipped get distinct icons, and labels are more descriptive.
- **Rate limiting defaults raised** — public limit 30 → 60/min, authenticated 120 → 300/min. Dashboard polling was too close to the old limit.
- **Rate limiting keyed by IP in demo mode** — authenticated middleware falls back to client IP when no API key is present, so demo users get individual rate limit buckets instead of sharing one.
- **Rate limit 429 page** — returns a styled HTML page when the browser requests HTML (with retry countdown and "Try Again" button), JSON for API calls. Frontend shows a toast on 429.
- **Removed time filter from jobs page** — the time range picker hid jobs whose last run was outside the window, which was confusing. Time filters remain on Executions and Events pages.
- **State column shows execution state** — Running and pending_approval executions now override the scheduling status in the jobs table State column

### Fixed
- **Agent callbacks now emit events** — execution.completed events were missing for agent-executed jobs, so failed agent jobs didn't appear in the events log
- **Approval flow preserves params** — trigger parameters are now stored on pending_approval executions and passed through when approved
- **JSON escape safety** — parameter substitution fallback now manually escapes dangerous characters instead of inserting raw values
- **Params validated as object** — trigger endpoint rejects non-object params with 400
- **Stale params in trigger modal** — frontend now fetches fresh job data before showing the parameter form
- **Streaming select loop hang** — fixed infinite loop when shell commands exit quickly (e.g. command not found)
- **cargo-deny** — license and advisory checks in CI with `deny.toml` config. Replaced unmaintained `rustls-pemfile` with `rustls-pki-types`.
- **Dependabot** — weekly automated dependency updates for Cargo and GitHub Actions
- **OIDC/SSO Authentication** — optional OpenID Connect login alongside existing API keys. Supports Okta, Azure AD, Google, Keycloak, and any standard OIDC provider. Configurable role mapping from IdP claims to Kronforce roles (admin/operator/viewer). Server-side sessions stored in SQLite with automatic cleanup.
- **Native TLS** — set `KRONFORCE_TLS_CERT` and `KRONFORCE_TLS_KEY` to serve HTTPS on both controller and agent; agent client auto-detects HTTPS when agents use port 443
- **Webhook notifications** — Slack, Microsoft Teams, PagerDuty, and generic webhook support alongside existing email and SMS channels
- **Prometheus metrics** — `/metrics` endpoint with execution counts, job/agent/group totals, and database health in Prometheus exposition format
- **Approval workflows** — jobs can require approval before execution (`approval_required` flag); triggering creates a `pending_approval` execution that must be approved via `POST /api/executions/{id}/approve` before running
- **SLA deadlines** — per-job completion deadline (HH:MM UTC) with configurable early warning; background monitor fires `sla.warning` and `sla.breach` events when running jobs approach or miss their deadline
- **Secret variables** — variables can be marked as secret; values are masked in API responses and the UI but still substituted into tasks at runtime
- **Job version history** — every create/update snapshots the full job definition; query via `GET /api/jobs/{id}/versions` for audit trail and rollback reference
- **Job priority** — `priority` field on jobs (default 0); higher priority jobs are scheduled first when multiple are due simultaneously
- **API key group scoping** — API keys can be restricted to specific job groups (`allowed_groups`), giving team-level isolation without full multi-tenancy
- **HA/Litestream replication** — Docker Compose setup for continuous SQLite replication to S3 with automatic restore on failover
- **Enhanced health endpoint** — `/api/health` now reports database status, file size, WAL size, and connection pool info
- **Graceful shutdown** — WAL checkpoint on SIGTERM/SIGINT ensures clean database state for replication
- **Output extraction targets** — extractions can now target "variable" (write to global var) or "output" (replace execution stdout with extracted values)
- **Regex full-match fallback** — extraction patterns without capture groups now return the full match instead of silently returning nothing
- **gRPC custom agent example** — `examples/grpc_agent.py` wraps grpcurl for calling gRPC services via the custom agent protocol
- **Getting Started page** — interactive in-app guide with step-by-step setup and action buttons
- **Migration guide** — docs for migrating from cron, Rundeck, and Airflow with feature mapping tables and step-by-step instructions
- **Crontab import tool** — `scripts/kronforce-import-crontab` reads crontab from stdin and creates Kronforce jobs via API, with dry-run mode, group assignment, and name prefixing
- **Job templates** — save any job as a reusable template, create new jobs from templates via "From Template" button on Jobs page; templates stored in SQLite with API (`GET/POST /api/templates`, `GET/DELETE /api/templates/{name}`)
- **Job Groups** — organize jobs into named groups with a dedicated Groups page, Default group for all new jobs, group filter on jobs list, and group dropdown in job create/edit modal
- **Dashboard Charts** — donut charts for execution outcomes, task types, and schedule types with SVG rendering
- **Dashboard Tabs** — tabbed layout (Overview, Activity, Infrastructure) to reduce scrolling
- **MCP Task Type** — call tools on MCP (Model Context Protocol) servers via stdio or HTTP transport, with tool discovery API and dynamic UI form
- **Execution Retry** — automatic retry on failure/timeout with configurable max retries, delay, and exponential backoff
- **API Rate Limiting** — per-IP (public), per-API-key (authenticated), and per-key (agent) rate limits with 429 responses and Retry-After headers
- **Audit Log** — append-only audit trail for all sensitive operations (key management, job CRUD, script changes, settings, variables, agent deregister) with query API
- **Connection Pooling** — r2d2 connection pool replaces single Mutex<Connection> for concurrent database access
- **Docker images on GHCR** — multi-arch (linux/amd64, linux/arm64) images published to `ghcr.io/mikemiles-dev/kronforce` on each release
- **Windows build** — x86_64 Windows binaries included in releases
- **MCP Server** — Kronforce acts as an MCP server at `POST /mcp`, exposing 10 tools with role-based access

### Changed
- Sidebar redesigned — 7 focused tabs: Dashboard, Monitor, Pipelines, Builder, Toolbox, Settings, Docs. Sub-tabs within Monitor (Jobs/Runs/Events), Pipelines (Stages/Map), and Toolbox (Scripts/Variables/Connections)
- Job create/edit modal — group dropdown, priority field, approval checkbox, SLA deadline fields in Advanced tab
- Trigger job endpoint returns `202 Accepted` instead of `200 OK`
- Bootstrap API keys only printed to stderr (never written to disk)
- Dashboard reorganized — pie charts under Overview, recent executions/groups under Activity tab
- Execution timestamps show full UTC on hover, relative time inline
- Execution detail modal shows job name (clickable), started/finished UTC timestamps, approve button for pending_approval
- Execution list — sortable columns (Job, Status, Started, Duration), job names always resolve correctly
- Latest execution per job highlighted with blue border and "latest" label
- Shareable URLs for execution details (`#/executions/{id}`)
- Map view shows group badges on nodes and group filter dropdown

### Fixed
- **Controller-to-agent dispatch auth** — standard agent endpoints now require authentication; controller sends dispatch key
- **Command injection** in Kafka task properties parameter
- **Privilege escalation** via `run_as` — username now validated
- **Missing authorization** on variable and script mutation endpoints (now require write role)
- **~50+ unsafe `.unwrap()` calls** replaced with proper error propagation across all DB/executor/API layers
- **SSRF protection** — HTTP tasks block private IPs, localhost, and cloud metadata endpoints
- **ReDoS protection** — regex patterns capped at 1024 characters in output rules
- **Credential exposure** — FTP credentials now passed via temp netrc file instead of command-line arguments
- **CORS and security headers** — X-Frame-Options, X-Content-Type-Options, Referrer-Policy
- **Foreign key constraint** on job deletion — now cascades to executions and queue items
- **Output extractions** — regex patterns without capture groups silently returned nothing; now falls back to full match
- **Executions page job names** — always fetches jobs before rendering so names resolve on fresh load
- **Modal click-to-close** — modals no longer close when clicking inside form fields or dragging
- **MCP client unwrap panic** — unsafe `.unwrap()` calls replaced with proper error handling
- **Sidebar user/health indicator** — restored username display and health dot

### Security
- **rustls-webpki updated** to 0.103.12 fixing RUSTSEC-2026-0098 (URI name constraints) and RUSTSEC-2026-0099 (wildcard name constraints)
- Native TLS support on controller and agent (rustls, no OpenSSL dependency)
- Agent dispatch authentication (controller → agent requests now require Bearer token)
- OIDC/OAuth2 SSO with server-side sessions
- API key group scoping for team isolation
- API rate limiting on all endpoints (configurable, 3 tiers)
- Audit logging for all state-changing operations
- SSRF protection on HTTP task URLs
- Secret variable masking in API and UI
- Approval workflow gates on job execution

## [0.1.0] - Initial Release

See [README.md](README.md) for the full feature set.
