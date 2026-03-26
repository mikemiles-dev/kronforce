## Context

The system has a settings key-value table for runtime configuration (currently holds `retention_days`). Events are emitted for all significant actions. The health monitor loop in `controller.rs` already detects agent offline transitions. Post-execution processing in `executor/local.rs` and `api/callbacks.rs` already runs output rules after each execution.

## Goals / Non-Goals

**Goals:**
- Email notifications via SMTP
- SMS notifications via HTTP webhook (Twilio-compatible)
- System alerts for agent offline events
- Per-job notification toggles (failure, success, assertion failure)
- Notification logging in the events feed
- Configuration through the Settings UI

**Non-Goals:**
- Push notifications / browser notifications
- Slack/Discord/Teams integrations (can be done with webhook SMS channel or event-triggered Rhai scripts)
- Notification templates / customizable message formatting
- Rate limiting or deduplication of notifications

## Decisions

### 1. Channel configuration stored as settings

**Decision**: Store notification channel config as JSON in the existing `settings` table using two keys:

- `notification_email` → `{ "enabled": true, "smtp_host": "smtp.gmail.com", "smtp_port": 587, "username": "...", "password": "...", "from": "alerts@example.com", "tls": true }`
- `notification_sms` → `{ "enabled": true, "webhook_url": "https://api.twilio.com/2010-04-01/Accounts/.../Messages.json", "auth_user": "...", "auth_pass": "...", "from_number": "+1234567890" }`
- `notification_recipients` → `{ "emails": ["ops@example.com"], "phones": ["+1234567890"] }`
- `notification_system_alerts` → `{ "agent_offline": true }`

**Rationale**: Reuses the existing settings infrastructure. No new tables. The UI already has a Settings page with a pattern for adding cards. JSON values give flexibility to add fields without migrations.

### 2. Per-job notification config as JSON on the Job

**Decision**: Add `notifications_json TEXT` column to the jobs table via migration. Stores:

```json
{
    "on_failure": true,
    "on_success": false,
    "on_assertion_failure": true,
    "recipients": {
        "emails": ["specific@example.com"],
        "phones": []
    }
}
```

If `recipients` is empty or absent, falls back to the global `notification_recipients` from settings. The `notifications` field on the Job struct is `Option<JobNotificationConfig>` — null means no notifications configured (default).

**Rationale**: Per-job config is the right granularity. Most jobs won't have notifications; the ones that do can override recipients for targeted alerting.

### 3. New `src/notifications.rs` module

**Decision**: Create a standalone notification module with:

```rust
pub async fn send_notification(subject: &str, body: &str, db: &Db)
pub async fn send_email(config: &EmailConfig, to: &[String], subject: &str, body: &str) -> Result<()>
pub async fn send_sms(config: &SmsConfig, to: &[String], body: &str) -> Result<()>
```

`send_notification` reads channel configs from the DB, checks if channels are enabled, and dispatches to email/SMS. It logs a `notification.sent` or `notification.failed` event.

**Rationale**: Standalone module keeps notification logic separate from execution and API code. The `send_notification` function is the single entry point — callers just provide subject and body.

### 4. Email via `lettre` crate

**Decision**: Use the `lettre` crate for SMTP email. It's the standard Rust email library with async support, TLS, and authentication.

```rust
let mailer = SmtpTransport::relay(&config.smtp_host)?
    .port(config.smtp_port)
    .credentials(Credentials::new(config.username, config.password))
    .build();
```

**Rationale**: `lettre` is well-maintained, supports STARTTLS/TLS, and handles connection pooling. No need to shell out to `sendmail`.

### 5. SMS via generic HTTP webhook

**Decision**: SMS sends a POST request to the configured webhook URL with a JSON body: `{ "To": phone, "From": from_number, "Body": message }`. Supports basic auth via `auth_user`/`auth_pass`. This is compatible with Twilio's API format but works with any webhook endpoint.

**Rationale**: Generic webhook approach supports Twilio, Vonage, and custom SMS gateways without provider-specific code. Users can also point it at a Slack webhook or any HTTP endpoint that accepts POST with a body field.

### 6. Notification dispatch points

**Decision**: Notifications are sent from three places:

1. **Post-execution** (executor/local.rs and api/callbacks.rs): After output rules run, check job's notification config. If `on_failure` and status is failed/timed_out, send. If `on_success` and succeeded, send. If `on_assertion_failure` and assertion just failed the job, send.

2. **Agent health monitor** (controller.rs): When an agent transitions to offline and `notification_system_alerts.agent_offline` is true, send notification.

3. All dispatch is async (tokio::spawn) — never blocks the main flow.

**Rationale**: These are the natural trigger points where we already have the context needed to decide whether to notify.

### 7. Notification message format

**Decision**: Simple plain-text format:

- **Subject**: `[Kronforce] Job 'etl-pipeline' failed` or `[Kronforce] Agent 'gpu-node' went offline`
- **Body**: Key details — job name, status, execution ID (truncated), timestamp, stderr excerpt (first 500 chars for failures)

Email gets a slightly richer body with more details. SMS gets a condensed one-liner.

**Rationale**: Keep it simple. No HTML email templates — plain text is universally readable and easier to maintain.

## Risks / Trade-offs

- **SMTP credentials stored in plaintext in settings** → Acceptable for a self-hosted tool. The settings table is behind API key auth. Could add encryption later.
- **No rate limiting** → A job running every second with `on_failure: true` and constantly failing would spam notifications. Acceptable risk — operators should fix the root cause. Could add cooldown later.
- **SMS webhook is Twilio-specific format** → The `To`/`From`/`Body` JSON format is Twilio's convention. Other providers may need different fields. Acceptable as a starting point — the webhook can be adapted.
