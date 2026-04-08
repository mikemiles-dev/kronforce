# Changelog

All notable changes to Kronforce will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha] - 2026-04-03

### Added
- **Skip dependencies on trigger** — manually trigger a blocked job with `?skip_deps=true` to bypass dependency checks for a single run. UI adds a "Run Anyway" button in the waiting-on-dependencies modal.
- **End-to-end tutorial** — new `docs/TUTORIAL.md` walks through controller setup, agent deployment, building a pipeline, and notifications
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
- Sidebar redesigned — compact icon-only buttons with flyout submenus for Jobs (Jobs/Groups/Executions), Tools (Scripts/Variables), and Manage (Agents/Settings)
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
