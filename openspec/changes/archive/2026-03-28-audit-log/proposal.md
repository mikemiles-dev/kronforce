## Why

Kronforce has a general-purpose events table that logs system activity, but these events can be purged by retention policies and lack structure for compliance auditing. There is no dedicated, tamper-resistant record of who did what and when for sensitive operations like API key creation/revocation, job modifications, script changes, and settings updates. A separate audit log is needed for security accountability and incident investigation.

## What Changes

- Add a new `audit_log` database table that is separate from the events table and exempt from retention purging
- Automatically record audit entries for sensitive operations: API key create/revoke, job create/update/delete, script save/delete, settings changes, variable create/update/delete, agent deregister
- Each audit entry captures: timestamp, actor (API key ID + name), operation type, resource type + ID, and a details field with before/after diff where applicable
- Add a `GET /api/audit-log` API endpoint for querying audit entries with pagination and filtering by operation type, actor, and time range
- Audit log entries are append-only — no update or delete API is exposed
- Add a configurable retention period for audit logs (default: 90 days, separate from events retention)
- Add an audit log section to the dashboard Activity tab or as its own view

## Capabilities

### New Capabilities
- `audit-log`: Append-only audit trail for sensitive operations, including database schema, recording logic, query API, and UI display

### Modified Capabilities

## Impact

- **Database**: New `audit_log` table via migration. No changes to existing tables.
- **Backend**: New `src/db/audit.rs` module for audit log DB operations. New `src/api/audit.rs` for the query endpoint. Audit recording calls added to existing API handlers (jobs, scripts, auth, settings, variables, agents).
- **Frontend**: New audit log view or section in the Activity tab showing recent audit entries.
- **Dependencies**: None — uses existing SQLite and Axum infrastructure.
- **No breaking changes**: Existing events system is untouched. Audit log is purely additive.
