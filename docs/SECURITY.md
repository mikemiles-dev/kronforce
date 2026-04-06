# Security

Kronforce security controls, compliance readiness, and operational security guidance.

## Encryption

### In Transit (TLS)

Set `KRONFORCE_TLS_CERT` and `KRONFORCE_TLS_KEY` to serve HTTPS on both controller and agent. Uses rustls (no OpenSSL dependency).

```bash
KRONFORCE_TLS_CERT=/path/to/cert.pem \
KRONFORCE_TLS_KEY=/path/to/key.pem \
cargo run --bin kronforce
```

For production, use a reverse proxy (Caddy, nginx) for TLS termination, or deploy with the HA Docker Compose which includes Caddy with auto-TLS via Let's Encrypt.

### At Rest (Field Encryption)

Set `KRONFORCE_ENCRYPTION_KEY` to encrypt sensitive fields (secret variable values) using AES-256-GCM before storing in SQLite.

```bash
# Generate a strong key
openssl rand -base64 32

# Set it
KRONFORCE_ENCRYPTION_KEY=your-random-key-here cargo run --bin kronforce
```

Encrypted values are prefixed with `enc:` in the database. If the key is not set, values are stored in plaintext. Changing the key requires re-encrypting existing values.

For full database encryption, use volume-level encryption (AWS EBS encryption, LUKS, DigitalOcean encrypted volumes).

## Authentication

### API Keys

- 4 roles: `admin`, `operator`, `viewer`, `agent`
- Keys stored as SHA-256 hashes (raw key never persisted)
- Bootstrap keys auto-generated on first startup
- Group scoping: restrict keys to specific job groups
- IP allowlisting: restrict keys to specific source IPs (per-key)

### OIDC/SSO

- OpenID Connect with any standard provider (Okta, Azure AD, Google, Keycloak)
- Server-side sessions in SQLite with configurable TTL (default 24h)
- Role mapping from IdP claims
- Session cookie: `HttpOnly`, `SameSite=Lax`, `Secure` (when HTTPS)

### Agent Authentication

- Controller → agent dispatch authenticated with shared key
- Agent → controller authenticated with `KRONFORCE_AGENT_KEY`
- Constant-time key comparison to prevent timing attacks

## Authorization

| Role | Read | Write | Manage Keys | Agent Ops |
|---|---|---|---|---|
| Admin | Yes | Yes | Yes | Yes |
| Operator | Yes | Yes | No | No |
| Viewer | Yes | No | No | No |
| Agent | No | No | No | Yes |

API keys can be scoped to specific job groups (`allowed_groups`). Scoped keys can only see and manage jobs in their allowed groups. Admin keys bypass group scoping.

## Audit Logging

All state-changing operations are recorded in an append-only audit log:

- API key creation and revocation
- Job create, update, delete, trigger
- Script save and delete
- Settings changes
- Variable create, update, delete
- Agent deregistration
- Execution approval

Query via `GET /api/audit-log` (admin only). Separate retention from events (default 90 days).

## Rate Limiting

Three-tier rate limiting protects against abuse:

| Tier | Scope | Default | Config |
|---|---|---|---|
| Public | Per IP | 30 req/min | `KRONFORCE_RATE_LIMIT_PUBLIC` |
| Authenticated | Per API key | 120 req/min | `KRONFORCE_RATE_LIMIT_AUTHENTICATED` |
| Agent | Per API key | 600 req/min | `KRONFORCE_RATE_LIMIT_AGENT` |

Returns `429 Too Many Requests` with `Retry-After` header.

## Input Validation & Protections

- **SSRF protection**: HTTP tasks block private IPs, localhost, and cloud metadata endpoints
- **Command injection**: Kafka properties sanitized
- **ReDoS protection**: Regex patterns capped at 1024 characters
- **Privilege escalation**: `run_as` username validated
- **Credential handling**: FTP credentials via temp netrc file (not command-line args)
- **XSS prevention**: All user input escaped with `esc()` in frontend
- **SQL injection**: All queries use parameterized statements
- **CSRF**: Session cookies use `SameSite=Lax`
- **Security headers**: `X-Frame-Options: DENY`, `X-Content-Type-Options: nosniff`, `Referrer-Policy: strict-origin-when-cross-origin`

## Data Management

### Export

```bash
curl http://localhost:8080/api/data/export \
  -H "Authorization: Bearer kf_admin_key"
```

Returns full JSON export of all jobs, variables, templates, agents, and groups. Admin only.

### Deletion

```bash
curl -X DELETE http://localhost:8080/api/data/delete \
  -H "Authorization: Bearer kf_admin_key" \
  -H "X-Confirm-Delete: yes-delete-all-data"
```

Purges all jobs, executions, variables, templates, events, audit log, and sessions. Requires admin role and explicit confirmation header. Irreversible.

### Retention

Automatic data purging configured via Settings:
- `retention_days` (default 7): executions, events, queue items
- `audit_retention_days` (default 90): audit log entries

## Logging

### Standard (human-readable)
```bash
RUST_LOG=kronforce=info cargo run --bin kronforce
```

### JSON (for SIEM integration)
```bash
KRONFORCE_LOG_FORMAT=json RUST_LOG=kronforce=info cargo run --bin kronforce
```

JSON logs are compatible with Datadog, Splunk, CloudWatch, ELK, and any log aggregation service that parses structured JSON.

## Dependency Security

- `cargo audit` runs in CI on every push and pull request
- Checks for known vulnerabilities (RustSec advisory database)
- Checks for yanked crates
- All dependencies pinned via `Cargo.lock`

## Incident Response Checklist

If you suspect a security incident:

1. **Rotate keys immediately**: create new admin/agent keys, revoke compromised ones
2. **Check audit log**: `GET /api/audit-log` for unauthorized changes
3. **Check events**: `GET /api/events` for unusual activity
4. **Review sessions**: expired OIDC sessions are cleaned automatically; force-clear by restarting
5. **Export data**: `GET /api/data/export` for forensic backup
6. **Change encryption key**: if `KRONFORCE_ENCRYPTION_KEY` is compromised, rotate it and re-encrypt secrets
7. **Review agent keys**: check if any agent keys were used from unexpected IPs

## SOC 2 Control Mapping

| SOC 2 Criteria | Kronforce Control |
|---|---|
| CC6.1 Logical access | API key roles, OIDC/SSO, group scoping |
| CC6.2 Access provisioning | API key creation with role assignment |
| CC6.3 Access removal | Key revocation, session expiry |
| CC6.6 System boundaries | Rate limiting, IP allowlisting |
| CC6.7 Manage changes | Audit logging, job version history |
| CC6.8 Prevent unauthorized software | Dependency audit in CI |
| CC7.1 Detect security events | Event system, audit log |
| CC7.2 Monitor system components | Prometheus metrics, health endpoint |
| CC7.3 Evaluate security events | JSON logging for SIEM integration |
| CC8.1 Change management | Job versioning, audit trail |
| CC9.1 Risk mitigation | Encryption at rest, TLS, SSRF protection |
| A1.2 Recovery | Litestream replication, data export |
