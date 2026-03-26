### Requirement: Notification module sends email via SMTP
The system SHALL provide a `send_email` function that connects to the configured SMTP server and sends a plain-text email to the specified recipients.

#### Scenario: Sending email successfully
- **WHEN** `send_email` is called with valid SMTP config and recipients
- **THEN** the email is delivered and a `notification.sent` event is logged

#### Scenario: SMTP connection failure
- **WHEN** the SMTP server is unreachable
- **THEN** a `notification.failed` event is logged with the error message

### Requirement: Notification module sends SMS via webhook
The system SHALL provide a `send_sms` function that POSTs to the configured webhook URL with the message body.

#### Scenario: Sending SMS successfully
- **WHEN** `send_sms` is called with valid webhook config and phone numbers
- **THEN** the HTTP POST is sent and a `notification.sent` event is logged

#### Scenario: Webhook failure
- **WHEN** the webhook returns a non-2xx status
- **THEN** a `notification.failed` event is logged with the status code

### Requirement: Notifications dispatched asynchronously
All notification sending SHALL be spawned as async tasks that do not block the main execution flow.

#### Scenario: Notification does not delay execution result
- **WHEN** a job completes and triggers a notification
- **THEN** the execution result is saved immediately and the notification is sent in the background

### Requirement: Notification events logged
Each notification attempt SHALL be logged as an event with kind `notification.sent` (success) or `notification.failed` (error).

#### Scenario: Successful notification logged
- **WHEN** a notification is sent successfully
- **THEN** an event with kind `notification.sent` and severity `info` is created with details about the channel and recipient

#### Scenario: Failed notification logged
- **WHEN** a notification fails to send
- **THEN** an event with kind `notification.failed` and severity `error` is created with the error message
