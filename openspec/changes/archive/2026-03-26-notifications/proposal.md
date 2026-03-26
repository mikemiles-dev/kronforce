## Why

When jobs fail, agents go offline, or output assertions trip, nobody knows unless they're watching the dashboard. Operators need proactive alerts via email and SMS so they can respond to issues without polling the UI. Notification preferences should be configurable at both the system level (global alerts for agent events) and the job level (per-job failure notifications).

## What Changes

- **Notification channels** configured in Settings: SMTP email (server, port, from address, credentials) and SMS via webhook (Twilio-compatible URL + auth). Channels are stored as settings in the database.
- **System-level alerts**: Automatically notify when an agent goes offline. Configurable in Settings — toggle on/off, select channels, add recipient addresses/numbers.
- **Per-job notification settings**: Each job gets a `notifications` config with toggles for: notify on failure, notify on success, notify on assertion failure. Plus recipient overrides (defaults to system recipients if not set).
- **Notification dispatch**: A notification module that sends email via SMTP and SMS via HTTP webhook. Runs asynchronously after events fire — doesn't block execution.
- **Notification log**: Each sent notification is logged as an event (`notification.sent` / `notification.failed`) visible in the events feed.
- **Settings UI**: New "Notifications" card in Settings page for configuring channels and system alerts. Job modal gets notification checkboxes in the Advanced section.

## Capabilities

### New Capabilities
- `notification-channels`: SMTP email and SMS webhook channel configuration stored in settings
- `notification-dispatch`: Async notification sending with email/SMS support and event logging
- `job-notifications`: Per-job notification preferences (on failure, success, assertion failure)
- `system-alerts`: Global alerts for system events (agent offline)

### Modified Capabilities

## Impact

- **Models**: New `NotificationConfig` on Job struct
- **Database**: Notification settings stored via existing settings table (no new tables)
- **Backend**: New `src/notifications.rs` module for dispatch logic
- **Executor/Callbacks**: After execution completes, check job notification config and send if needed
- **Controller**: Agent health monitor sends notifications on agent offline
- **Frontend**: Settings page gets notification channel config; job modal gets notification toggles
- **Dependencies**: `lettre` crate for SMTP email; `reqwest` already available for SMS webhooks
