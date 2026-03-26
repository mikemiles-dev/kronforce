## 1. Dependencies and Module Setup

- [x] 1.1 Add `lettre` crate to `Cargo.toml` (with `tokio1-native-tls` feature for async SMTP)
- [x] 1.2 Create `src/notifications.rs` module with structs: `EmailConfig`, `SmsConfig`, `NotificationRecipients`
- [x] 1.3 Register module in `src/lib.rs`

## 2. Database: Job Notification Config

- [x] 2.1 Add migration to add `notifications_json TEXT` column to jobs table
- [x] 2.2 Add `JobNotificationConfig` struct to `src/models.rs` with `on_failure`, `on_success`, `on_assertion_failure` booleans and optional `recipients` override
- [x] 2.3 Add `notifications: Option<JobNotificationConfig>` field to Job struct
- [x] 2.4 Update `row_to_job()` in `db/helpers.rs` to read `notifications_json` column
- [x] 2.5 Update job SELECT queries in `db/jobs.rs` to include `notifications_json`
- [x] 2.6 Update `insert_job()` and `update_job()` to store `notifications_json`

## 3. Backend: Notification Dispatch

- [x] 3.1 Implement `load_email_config(db) -> Option<EmailConfig>` that reads `notification_email` setting
- [x] 3.2 Implement `load_sms_config(db) -> Option<SmsConfig>` that reads `notification_sms` setting
- [x] 3.3 Implement `load_recipients(db) -> NotificationRecipients` that reads `notification_recipients` setting
- [x] 3.4 Implement `send_email(config, to, subject, body) -> Result<()>` using `lettre` SMTP transport
- [x] 3.5 Implement `send_sms(config, to, body) -> Result<()>` using `reqwest` POST to webhook URL with basic auth
- [x] 3.6 Implement `send_notification(db, subject, body, recipient_override)` — loads configs, dispatches to enabled channels, logs `notification.sent`/`notification.failed` events
- [x] 3.7 Add `notification.sent` and `notification.failed` event kinds

## 4. Backend: Post-Execution Notification

- [x] 4.1 In `executor/local.rs` after output rules processing, check job's notification config and call `send_notification` for matching conditions (on_failure, on_success, on_assertion_failure)
- [x] 4.2 In `api/callbacks.rs` after output rules processing, same notification check for agent-executed jobs
- [x] 4.3 Build notification subject and body: job name, status, execution ID (short), timestamp, stderr excerpt (first 500 chars) for failures
- [x] 4.4 Use job-level recipient overrides if present, otherwise fall back to global recipients
- [x] 4.5 Spawn notification dispatch as async task (tokio::spawn) — never block execution result

## 5. Backend: System Alerts

- [x] 5.1 In `controller.rs` agent health monitor loop, after detecting agent offline transition, read `notification_system_alerts` setting
- [x] 5.2 If `agent_offline: true`, call `send_notification` with subject `[Kronforce] Agent 'name' went offline` and body with agent details

## 6. API: Job Notification Config

- [x] 6.1 Add `notifications` field to `CreateJobRequest` and `UpdateJobRequest` in `api/jobs.rs`
- [x] 6.2 Pass `notifications` through to job creation and update handlers

## 7. Frontend: Settings — Notifications Card

- [x] 7.1 Add "Notifications" card to the Settings page with three subsections: Email, SMS, Recipients
- [x] 7.2 Email subsection: inputs for SMTP host, port, username, password, from address, TLS checkbox, enabled toggle, "Send Test" button
- [x] 7.3 SMS subsection: inputs for webhook URL, auth user, auth password, from number, enabled toggle
- [x] 7.4 Recipients subsection: textarea for email addresses (one per line), textarea for phone numbers (one per line)
- [x] 7.5 System Alerts subsection: checkbox for "Agent went offline"
- [x] 7.6 Save button that PUTs all notification settings to `/api/settings`
- [x] 7.7 Load existing notification settings when Settings page opens
- [x] 7.8 "Send Test" button calls a new `POST /api/notifications/test` endpoint

## 8. Frontend: Job Modal — Notification Toggles

- [x] 8.1 Add "Notifications" subsection in the job modal Advanced section with checkboxes: "Notify on failure", "Notify on success", "Notify on assertion failure"
- [x] 8.2 Add optional recipient override field (email addresses, comma-separated)
- [x] 8.3 Update `submitJobForm()` to collect notification config and include in job body
- [x] 8.4 Update `openEditModal()` to populate notification checkboxes from existing job data
- [x] 8.5 Update `openCreateModal()` to reset notification checkboxes

## 9. API: Test Notification Endpoint

- [x] 9.1 Add `POST /api/notifications/test` endpoint that sends a test notification to the first configured recipient
- [x] 9.2 Return success/failure JSON response with details
