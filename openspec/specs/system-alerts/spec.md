### Requirement: System alert for agent going offline
When an agent transitions from online to offline and the `notification_system_alerts.agent_offline` setting is true, the system SHALL send a notification.

#### Scenario: Agent goes offline with alerts enabled
- **WHEN** an agent's heartbeat times out and `notification_system_alerts.agent_offline` is true
- **THEN** a notification is sent with subject `[Kronforce] Agent 'name' went offline` and body including agent name, hostname, and last heartbeat time

#### Scenario: Agent goes offline with alerts disabled
- **WHEN** an agent's heartbeat times out and `notification_system_alerts.agent_offline` is false or not configured
- **THEN** no notification is sent (the `agent.offline` event is still logged)

### Requirement: System alerts configured in Settings UI
The Settings page SHALL have a "System Alerts" subsection within the Notifications card with toggles for each system alert type.

#### Scenario: Enabling agent offline alerts
- **WHEN** the admin toggles "Agent went offline" in the system alerts section
- **THEN** the `notification_system_alerts` setting is updated with `agent_offline: true`

#### Scenario: System alerts use global recipients
- **WHEN** a system alert notification is triggered
- **THEN** it is sent to the global recipients configured in `notification_recipients`
