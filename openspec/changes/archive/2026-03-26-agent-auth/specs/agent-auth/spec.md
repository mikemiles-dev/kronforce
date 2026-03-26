## ADDED Requirements

### Requirement: Agent endpoints require shared secret when configured
When `KRONFORCE_AGENT_KEY` is set on the controller, all agent endpoints SHALL require the key in the `Authorization: Bearer <key>` header.

#### Scenario: Agent key set and correct key provided
- **WHEN** the controller has `KRONFORCE_AGENT_KEY` set and a request includes the correct key in the Authorization header
- **THEN** the request is processed normally

#### Scenario: Agent key set and wrong key provided
- **WHEN** the controller has `KRONFORCE_AGENT_KEY` set and a request includes an incorrect key
- **THEN** the request is rejected with 401 Unauthorized

#### Scenario: Agent key set and no key provided
- **WHEN** the controller has `KRONFORCE_AGENT_KEY` set and a request has no Authorization header
- **THEN** the request is rejected with 401 Unauthorized

#### Scenario: Agent key not configured
- **WHEN** the controller has no `KRONFORCE_AGENT_KEY` environment variable
- **THEN** agent endpoints are open without authentication (backwards compatible)

### Requirement: Standard agent sends key in all requests
When `KRONFORCE_AGENT_KEY` is set in the agent's environment, the standard agent binary SHALL include the key as `Authorization: Bearer <key>` in all HTTP requests to the controller.

#### Scenario: Agent key configured on agent
- **WHEN** the agent has `KRONFORCE_AGENT_KEY` set
- **THEN** registration, heartbeat, and callback requests include the Authorization header

#### Scenario: Agent key not configured on agent
- **WHEN** the agent has no `KRONFORCE_AGENT_KEY`
- **THEN** requests are sent without an Authorization header

### Requirement: Agent key stored in controller config and AppState
The controller SHALL read `KRONFORCE_AGENT_KEY` from the environment on startup and store it in `AppState` for the middleware to access.

#### Scenario: Key available in AppState
- **WHEN** the controller starts with `KRONFORCE_AGENT_KEY=mysecret`
- **THEN** `AppState.agent_key` is `Some("mysecret")`

#### Scenario: Key not set
- **WHEN** the controller starts without `KRONFORCE_AGENT_KEY`
- **THEN** `AppState.agent_key` is `None`

### Requirement: Agent auth middleware applied to agent route group
A separate `agent_auth_middleware` SHALL be applied to agent endpoints as a distinct route layer, separate from the existing API key middleware.

#### Scenario: Agent endpoints protected
- **WHEN** agent auth is enabled
- **THEN** register, poll, heartbeat, callback, and task-type discovery endpoints all require the agent key

#### Scenario: Dashboard API endpoints unaffected
- **WHEN** agent auth is enabled
- **THEN** dashboard API endpoints continue to use the existing API key auth, not the agent key
