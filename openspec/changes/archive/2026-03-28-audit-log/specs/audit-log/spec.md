## ADDED Requirements

### Requirement: Audit log database table
The system SHALL store audit entries in a dedicated `audit_log` table separate from the `events` table. Each entry SHALL contain: id (UUID), timestamp (RFC3339), actor_key_id, actor_key_name, operation, resource_type, resource_id, and details (JSON string). The table SHALL be created via a database migration.

#### Scenario: Migration creates audit_log table
- **WHEN** the controller starts and runs migrations
- **THEN** the `audit_log` table is created with columns: id, timestamp, actor_key_id, actor_key_name, operation, resource_type, resource_id, details, and indexes on timestamp and operation

#### Scenario: Table is separate from events
- **WHEN** the events retention purge runs
- **THEN** the `audit_log` table is not affected

### Requirement: Audit recording for API key operations
The system SHALL record an audit entry when an API key is created or revoked.

#### Scenario: API key created
- **WHEN** an admin creates a new API key via `POST /api/keys`
- **THEN** an audit entry is recorded with operation `key.created`, resource_type `api_key`, resource_id set to the new key's UUID, and details containing the key name and role

#### Scenario: API key revoked
- **WHEN** an admin revokes an API key via `DELETE /api/keys/{id}`
- **THEN** an audit entry is recorded with operation `key.revoked`, resource_type `api_key`, and resource_id set to the revoked key's UUID

### Requirement: Audit recording for job operations
The system SHALL record an audit entry when a job is created, updated, deleted, or manually triggered.

#### Scenario: Job created
- **WHEN** a user creates a job via `POST /api/jobs`
- **THEN** an audit entry is recorded with operation `job.created`, resource_type `job`, resource_id set to the job UUID, and details containing the job name

#### Scenario: Job updated
- **WHEN** a user updates a job via `PUT /api/jobs/{id}`
- **THEN** an audit entry is recorded with operation `job.updated`, resource_type `job`, resource_id set to the job UUID, and details containing changed fields

#### Scenario: Job deleted
- **WHEN** a user deletes a job via `DELETE /api/jobs/{id}`
- **THEN** an audit entry is recorded with operation `job.deleted`, resource_type `job`, and resource_id set to the job UUID

#### Scenario: Job manually triggered
- **WHEN** a user triggers a job via `POST /api/jobs/{id}/trigger`
- **THEN** an audit entry is recorded with operation `job.triggered`, resource_type `job`, and resource_id set to the job UUID

### Requirement: Audit recording for script operations
The system SHALL record an audit entry when a script is saved or deleted.

#### Scenario: Script saved
- **WHEN** a user saves a script via `PUT /api/scripts/{name}`
- **THEN** an audit entry is recorded with operation `script.saved`, resource_type `script`, and resource_id set to the script name

#### Scenario: Script deleted
- **WHEN** a user deletes a script via `DELETE /api/scripts/{name}`
- **THEN** an audit entry is recorded with operation `script.deleted`, resource_type `script`, and resource_id set to the script name

### Requirement: Audit recording for settings and variable operations
The system SHALL record audit entries for settings updates, variable creates/updates/deletes, and agent deregistrations.

#### Scenario: Settings updated
- **WHEN** a user updates settings via `PUT /api/settings`
- **THEN** an audit entry is recorded with operation `settings.updated`, resource_type `settings`

#### Scenario: Variable created
- **WHEN** a user creates a variable via `POST /api/variables`
- **THEN** an audit entry is recorded with operation `variable.created`, resource_type `variable`, and resource_id set to the variable name

#### Scenario: Variable updated
- **WHEN** a user updates a variable via `PUT /api/variables/{name}`
- **THEN** an audit entry is recorded with operation `variable.updated`, resource_type `variable`, and resource_id set to the variable name

#### Scenario: Variable deleted
- **WHEN** a user deletes a variable via `DELETE /api/variables/{name}`
- **THEN** an audit entry is recorded with operation `variable.deleted`, resource_type `variable`, and resource_id set to the variable name

#### Scenario: Agent deregistered
- **WHEN** a user deregisters an agent via `DELETE /api/agents/{id}`
- **THEN** an audit entry is recorded with operation `agent.deregistered`, resource_type `agent`, and resource_id set to the agent UUID

### Requirement: Audit log query API
The system SHALL provide a `GET /api/audit-log` endpoint that returns paginated audit entries newest-first. The endpoint SHALL require admin role. It SHALL support optional query parameters: `page`, `per_page`, `operation`, `actor` (key name substring match), and `since` (ISO timestamp).

#### Scenario: List audit entries
- **WHEN** an admin requests `GET /api/audit-log`
- **THEN** the system returns a paginated response with audit entries ordered by timestamp descending

#### Scenario: Filter by operation
- **WHEN** an admin requests `GET /api/audit-log?operation=job.created`
- **THEN** only audit entries with operation `job.created` are returned

#### Scenario: Filter by actor
- **WHEN** an admin requests `GET /api/audit-log?actor=admin`
- **THEN** only audit entries where the actor key name contains "admin" are returned

#### Scenario: Non-admin denied
- **WHEN** a non-admin user requests `GET /api/audit-log`
- **THEN** the system returns 403 Forbidden

#### Scenario: No update or delete endpoints
- **WHEN** any user attempts to PUT, POST, or DELETE on `/api/audit-log`
- **THEN** the system returns 405 Method Not Allowed (no such routes exist)

### Requirement: Audit log retention
The system SHALL support a configurable retention period for audit log entries, independent of events retention. The default SHALL be 90 days. Entries older than the retention period SHALL be purged periodically.

#### Scenario: Default retention
- **WHEN** no `audit_retention_days` setting is configured
- **THEN** audit entries older than 90 days are purged

#### Scenario: Custom retention
- **WHEN** `audit_retention_days` is set to 365 in settings
- **THEN** audit entries older than 365 days are purged and entries within 365 days are preserved

#### Scenario: Retention runs periodically
- **WHEN** the controller health monitor loop runs
- **THEN** audit log purging is executed alongside existing event/execution purging

### Requirement: Audit entry actor attribution
Every audit entry SHALL record the acting API key's UUID and name. If auth is disabled (no API keys configured), the actor fields SHALL be null.

#### Scenario: Authenticated action
- **WHEN** a user with API key "deploy-bot" performs a job update
- **THEN** the audit entry's actor_key_id is set to the key's UUID and actor_key_name is "deploy-bot"

#### Scenario: Unauthenticated action (auth disabled)
- **WHEN** no API keys are configured and a user creates a job
- **THEN** the audit entry's actor_key_id and actor_key_name are null
