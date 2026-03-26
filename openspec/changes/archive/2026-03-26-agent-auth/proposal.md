## Why

Agent endpoints (register, poll, heartbeat, callback, task-type discovery) are completely unauthenticated. Anyone who knows the controller URL can register a rogue agent, poll for jobs containing sensitive data, or submit fake execution results. A shared agent secret provides a simple barrier that prevents unauthorized access without the complexity of per-agent key management.

## What Changes

- **Shared agent secret**: A single pre-shared key (`KRONFORCE_AGENT_KEY`) configured as an environment variable on the controller. When set, all agent endpoints require the key in the `Authorization: Bearer <key>` header.
- **Agent binary sends the key**: The standard agent reads `KRONFORCE_AGENT_KEY` from its environment and includes it in all requests to the controller.
- **Custom agent protocol updated**: Custom agents must include the key in their register, poll, heartbeat, and callback requests when agent auth is enabled.
- **Backwards compatible**: If `KRONFORCE_AGENT_KEY` is not set, agent endpoints remain open (current behavior). This makes it opt-in.
- **UI and docs updated**: Settings page shows whether agent auth is enabled. Docs and wizard updated with the key requirement.

## Capabilities

### New Capabilities
- `agent-auth`: Shared secret authentication for agent endpoints

### Modified Capabilities

## Impact

- **Config**: New `KRONFORCE_AGENT_KEY` environment variable on controller and agent
- **API middleware**: Agent endpoints get a separate auth check for the agent key
- **Agent binary**: Reads key from env and sends in headers
- **Python example**: Updated to include key in requests
- **Frontend**: Settings page shows agent auth status
- **Docs**: Updated protocol docs with auth requirement
