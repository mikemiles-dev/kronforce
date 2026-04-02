# Changelog

All notable changes to Kronforce will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha] - 2026-03-29

### Added
- **Job Groups** — organize jobs into named groups with a dedicated Groups page, Default group for all new jobs, group filter on jobs list, and group dropdown in job create/edit modal
- **Dashboard Charts** — donut charts for execution outcomes, task types, and schedule types with SVG rendering
- **Dashboard Tabs** — tabbed layout (Overview, Charts, Activity, Infrastructure) to reduce scrolling
- **MCP Task Type** — call tools on MCP (Model Context Protocol) servers via stdio or HTTP transport, with tool discovery API and dynamic UI form
- **Execution Retry** — automatic retry on failure/timeout with configurable max retries, delay, and exponential backoff
- **API Rate Limiting** — per-IP (public), per-API-key (authenticated), and per-key (agent) rate limits with 429 responses and Retry-After headers
- **Audit Log** — append-only audit trail for all sensitive operations (key management, job CRUD, script changes, settings, variables, agent deregister) with query API
- **Connection Pooling** — r2d2 connection pool replaces single Mutex<Connection> for concurrent database access
- **Chart Stats API** — `GET /api/stats/charts` endpoint for dashboard chart data
- **Groups API** — `GET/POST /api/jobs/groups`, `PUT /api/jobs/bulk-group`, `PUT /api/jobs/rename-group`
- **Audit Log API** — `GET /api/audit-log` (admin only, paginated, filterable)
- **Groups page** in sidebar with card grid, rename, delete, and share button
- **Groups stat card** on dashboard linking to groups page
- **Top Groups summary** on dashboard Overview tab
- **In-app docs** for Groups, Retry, Rate Limiting, and Audit Log
- **Docker images on GHCR** — multi-arch (linux/amd64, linux/arm64) images published to `ghcr.io/mikemiles-dev/kronforce` on each release
- **Windows build** — x86_64 Windows binaries included in releases (controller, dashboard, HTTP tasks, and Rhai scripts supported; shell/FTP/messaging tasks require Unix tools)
- **MCP Server** — Kronforce acts as an MCP server at `POST /mcp`, exposing 10 tools (list/get/create/trigger jobs, executions, agents, groups, events, stats) with role-based access via existing API keys

### Changed
- Sidebar reorganized — Executions and Map indented under Jobs as sub-entries
- Job create/edit modal — group field moved to main tab with accent-colored label, changed from text input to dropdown
- Trigger job endpoint returns `202 Accepted` instead of `200 OK`
- Bootstrap API keys no longer written to plaintext file — only printed to stderr
- Duplicate timeline mapping code extracted into helper function
- Docker compose files now pull from GHCR by default with local build fallback
- Dockerfile updated with missing build dependencies (build.rs, web/, migrations/)
- Dashboard reorganized — pie charts moved under Overview, recent executions/groups moved to Activity tab, Charts tab removed
- Execution timestamps now show full UTC on hover, relative time inline
- Execution detail modal shows job name (clickable), started/finished UTC timestamps
- Latest execution per job highlighted with blue border and "latest" label in execution list
- Shareable URLs for execution details (`#/executions/{id}`)
- Map view shows group badges on nodes and group filter dropdown
- Code architecture documentation added (`docs/CODE_ARCHITECTURE.md`)

### Fixed
- **Command injection** in Kafka task properties parameter
- **Privilege escalation** via `run_as` — username now validated
- **Missing authorization** on variable and script mutation endpoints (now require write role)
- **~50+ unsafe `.unwrap()` calls** replaced with proper error propagation across all DB/executor/API layers
- **SSRF protection** — HTTP tasks block private IPs, localhost, and cloud metadata endpoints
- **ReDoS protection** — regex patterns capped at 1024 characters in output rules
- **Credential exposure** — FTP credentials now passed via temp netrc file instead of command-line arguments
- **CORS and security headers** — X-Frame-Options, X-Content-Type-Options, Referrer-Policy
- **Foreign key constraint** on job deletion — now cascades to executions and queue items
- **Flaky config test** — serialized env var access with mutex
- **Agent callback retry** — bounded loop with capped exponential backoff
- **Scheduler cache failure** — preserves stale cache instead of dropping all jobs
- Input validation for job names, cron expressions, script size, group names
- `alert()` replaced with `toast()` in frontend variables page
- **Modal click-to-close** — modals no longer close when clicking inside form fields or dragging (tracks mousedown origin)
- **Executions page job names** — job names now resolve correctly instead of showing UUID hashes
- **MCP client unwrap panic** — 4 unsafe `.unwrap()` calls replaced with proper error handling
- **Base64 file size limit** — corrected from 7MB to 6.7MB (actual 5MB binary limit)
- **Retry scheduling failure** — now logged instead of silently dropped
- **Tooltip DOM leak** — orphaned timeline tooltips cleaned up on rapid hover
- **File upload state leak** — file push data cleared when reopening job modal
- **Custom agent state leak** — stale custom agent data cleared on new modal open
- **Event listener accumulation** — time range popup no longer adds duplicate listeners

### Security
- API rate limiting on all endpoints (configurable, 3 tiers)
- Audit logging for all state-changing operations
- Authorization checks on variables and scripts APIs
- SSRF protection on HTTP task URLs
- Command injection fix in Kafka properties
- run_as username validation
- Credential handling improvements (FTP netrc, removed bootstrap-keys.txt)

## [0.1.0] - Initial Release

See [README.md](README.md) for the full feature set.
