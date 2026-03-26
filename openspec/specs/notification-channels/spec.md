### Requirement: Email channel configured via settings
The system SHALL support an SMTP email notification channel configured through the Settings UI and stored in the `notification_email` settings key.

#### Scenario: Configuring email channel
- **WHEN** the admin enters SMTP host, port, username, password, from address, and TLS toggle in the Settings Notifications card
- **THEN** the configuration is saved as JSON to the `notification_email` setting

#### Scenario: Email channel disabled
- **WHEN** the `notification_email` setting has `enabled: false` or does not exist
- **THEN** no email notifications are sent regardless of other configuration

#### Scenario: Test email
- **WHEN** the admin clicks "Send Test" in the email channel config
- **THEN** a test email is sent to the first configured recipient and a success/failure toast is shown

### Requirement: SMS channel configured via settings
The system SHALL support an SMS notification channel via HTTP webhook configured through the Settings UI and stored in the `notification_sms` settings key.

#### Scenario: Configuring SMS channel
- **WHEN** the admin enters webhook URL, auth user, auth password, and from number in the Settings Notifications card
- **THEN** the configuration is saved as JSON to the `notification_sms` setting

#### Scenario: SMS sends via HTTP POST
- **WHEN** an SMS notification is dispatched
- **THEN** the system sends a POST request to the webhook URL with JSON body `{ "To": phone, "From": from_number, "Body": message }` and basic auth credentials

### Requirement: Global recipients configured via settings
The system SHALL store default notification recipients in the `notification_recipients` settings key with `emails` and `phones` arrays.

#### Scenario: Configuring recipients
- **WHEN** the admin enters email addresses and phone numbers in the Settings Notifications card
- **THEN** the recipients are saved as JSON arrays in the `notification_recipients` setting

#### Scenario: Recipients used as defaults
- **WHEN** a notification is triggered and the job has no recipient overrides
- **THEN** the global recipients from `notification_recipients` are used
