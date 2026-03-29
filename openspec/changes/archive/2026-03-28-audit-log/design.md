## Context

Kronforce currently logs system activity to an `events` table via `Db::log_event()` and `Db::log_audit()`. Events are general-purpose (execution completions, agent status changes, output triggers) and are subject to retention purging via `purge_old_events()`. The existing `log_audit()` method writes to the same events table — there is no dedicated audit storage.

API handlers already have access to the authenticated API key via the `AuthUser` extractor, and many handlers already call `log_and_notify()` which logs events. The audit recording calls will be placed alongside these existing event logging calls.

## Goals / Non-Goals

**Goals:**
- Dedicated append-only audit table separate from events, immune to retention purging by default
- Record all state-changing API operations with actor attribution
- Queryable via API with pagination, filtering by operation/actor/time
- Configurable audit retention (default 90 days) independent of event retention
- Minimal code footprint — a simple `Db::record_audit()` helper called from handlers

**Non-Goals:**
- Cryptographic tamper-proofing (hash chains, signed entries) — SQLite file integrity is sufficient for the threat model
- Real-time audit streaming or webhooks
- Audit log export (CSV, SIEM integration)
- Recording read operations (GET requests) — only state-changing operations
- Audit log in the dashboard UI beyond a simple list view

## Decisions

### 1. Separate `audit_log` table

```sql
CREATE TABLE audit_log (
    id TEXT PRIMARY KEY,
    timestamp TEXT NOT NULL,
    actor_key_id TEXT,
    actor_key_name TEXT,
    operation TEXT NOT NULL,
    resource_type TEXT NOT NULL,
    resource_id TEXT,
    details TEXT
);
CREATE INDEX idx_audit_log_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_log_operation ON audit_log(operation);
```

**Rationale:** Separate from events so retention policies don't accidentally purge audit data. The schema is intentionally flat — no foreign keys to avoid cascading deletes.

**Alternatives considered:**
- Tagging events with `is_audit=true` and skipping them in purge: Fragile, mixes concerns
- External audit service: Over-engineered for single-process deployment

### 2. `Db::record_audit()` convenience method

A single method that takes operation, resource type/id, actor info, and optional details JSON:

```rust
pub fn record_audit(
    &self,
    operation: &str,      // e.g. "key.created", "job.updated"
    resource_type: &str,  // e.g. "api_key", "job", "script"
    resource_id: Option<&str>,
    auth: &AuthUser,
    details: Option<&str>,
) -> Result<(), AppError>
```

Called from existing API handlers right after the operation succeeds. Details field stores JSON with before/after values for updates.

### 3. Operations to audit

| Operation | Resource Type | Trigger Location |
|-----------|--------------|------------------|
| `key.created` | api_key | `api/auth.rs` create_api_key |
| `key.revoked` | api_key | `api/auth.rs` revoke_api_key |
| `job.created` | job | `api/jobs.rs` create_job |
| `job.updated` | job | `api/jobs.rs` update_job |
| `job.deleted` | job | `api/jobs.rs` delete_job |
| `job.triggered` | job | `api/jobs.rs` trigger_job |
| `script.saved` | script | `api/scripts.rs` save_script |
| `script.deleted` | script | `api/scripts.rs` delete_script |
| `settings.updated` | settings | `api/settings.rs` update_settings |
| `variable.created` | variable | `api/variables.rs` create_variable |
| `variable.updated` | variable | `api/variables.rs` update_variable |
| `variable.deleted` | variable | `api/variables.rs` delete_variable |
| `agent.deregistered` | agent | `api/agents.rs` deregister_agent |

### 4. Query API: `GET /api/audit-log`

Parameters: `page`, `per_page`, `operation` (filter), `actor` (filter by key name), `since` (ISO timestamp). Returns paginated results newest-first. Requires admin role.

### 5. Audit retention via `purge_old_audit_log(days)`

Separate from event retention. Controlled by a new `audit_retention_days` setting (default 90). Called in the existing health monitor loop alongside `purge_old_events`.

## Risks / Trade-offs

- **Storage growth** → Audit log grows slower than events (only state-changing ops). At ~1KB per entry and 100 ops/day, 90 days is ~9MB. Negligible.
- **No tamper proofing** → An attacker with database file access can modify entries. Acceptable for the threat model — production deployments should restrict file access. Hash chains can be added later.
- **Performance of audit inserts** → One extra INSERT per state-changing API call. SQLite WAL mode handles this fine with no measurable latency impact.
