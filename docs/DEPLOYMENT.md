# Deployment

## Download Pre-Built Binaries

Pre-built binaries are available on the [GitHub Releases](https://github.com/mikemiles-dev/kronforce/releases) page for tagged versions.

| Platform | Controller | Agent |
|---|---|---|
| Linux x86_64 | `kronforce-linux-amd64` | `kronforce-agent-linux-amd64` |
| Linux ARM64 | `kronforce-linux-arm64` | `kronforce-agent-linux-arm64` |
| macOS x86_64 | `kronforce-darwin-amd64` | `kronforce-agent-darwin-amd64` |
| macOS ARM64 (Apple Silicon) | `kronforce-darwin-arm64` | `kronforce-agent-darwin-arm64` |

### Install from Release

```bash
# Download (replace VERSION and PLATFORM)
curl -L -o kronforce https://github.com/mikemiles-dev/kronforce/releases/download/VERSION/kronforce-PLATFORM
curl -L -o kronforce-agent https://github.com/mikemiles-dev/kronforce/releases/download/VERSION/kronforce-agent-PLATFORM
chmod +x kronforce kronforce-agent

# Example: latest release on Linux x86_64
VERSION=$(curl -s https://api.github.com/repos/mikemiles-dev/kronforce/releases/latest | grep tag_name | cut -d'"' -f4)
curl -L -o kronforce https://github.com/mikemiles-dev/kronforce/releases/download/$VERSION/kronforce-linux-amd64
curl -L -o kronforce-agent https://github.com/mikemiles-dev/kronforce/releases/download/$VERSION/kronforce-agent-linux-amd64
chmod +x kronforce kronforce-agent
```

### Run the Controller

```bash
./kronforce
```

First startup prints bootstrap API keys to the console. Save the admin key (for dashboard login) and agent key (for connecting agents).

### Run an Agent

```bash
KRONFORCE_AGENT_KEY=kf_your_agent_key \
KRONFORCE_CONTROLLER_URL=http://controller-host:8080 \
  ./kronforce-agent
```

### Run as a systemd Service

```ini
# /etc/systemd/system/kronforce.service
[Unit]
Description=Kronforce Controller
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/kronforce
Environment=KRONFORCE_DB=/var/lib/kronforce/kronforce.db
Environment=KRONFORCE_BIND=0.0.0.0:8080
Environment=KRONFORCE_SCRIPTS_DIR=/var/lib/kronforce/scripts
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```ini
# /etc/systemd/system/kronforce-agent.service
[Unit]
Description=Kronforce Agent
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/kronforce-agent
Environment=KRONFORCE_CONTROLLER_URL=http://controller-host:8080
Environment=KRONFORCE_AGENT_KEY=kf_your_agent_key
Environment=KRONFORCE_AGENT_NAME=%H
Environment=KRONFORCE_AGENT_TAGS=linux,prod
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable --now kronforce
sudo systemctl enable --now kronforce-agent
```

### Verify Checksums

Each release includes a `checksums-sha256.txt` file:

```bash
sha256sum -c checksums-sha256.txt
```

## Quick Start (Docker Compose)

### Local Development — Full Stack

Run the controller and a standard agent together with pre-set keys:

```bash
docker compose -f docker-compose.full.yml up -d
```

- Dashboard: http://localhost:8080
- Admin key: `kf_docker_admin_key_change_in_prod`
- Agent key: `kf_docker_agent_key_change_in_prod`

To use custom keys:

```bash
KRONFORCE_ADMIN_KEY=kf_myadminkey KRONFORCE_AGENT_KEY=kf_myagentkey \
  docker compose -f docker-compose.full.yml up -d
```

### Production — Controller

```bash
docker compose up -d
```

On first startup, the controller generates and prints bootstrap API keys:

```bash
docker compose logs controller | grep "key"
```

You'll see two keys:
- **Admin key** (`kf_...`) — use this to log into the dashboard
- **Agent key** (`kf_...`) — give this to agents so they can connect

To pre-set keys instead of auto-generating:

```bash
KRONFORCE_BOOTSTRAP_ADMIN_KEY=kf_myadminkey \
KRONFORCE_BOOTSTRAP_AGENT_KEY=kf_myagentkey \
  docker compose up -d
```

Bootstrap keys are only used on first startup with an empty database. After that, manage keys in the Settings page.

### Production — Agent (separate machine)

On the machine where you want to run an agent:

```bash
KRONFORCE_AGENT_KEY=kf_your_agent_key \
KRONFORCE_CONTROLLER_URL=http://your-controller:8080 \
  docker compose -f docker-compose.agent.yml up -d
```

Or create a `.env` file:

```env
KRONFORCE_AGENT_KEY=kf_your_agent_key
KRONFORCE_CONTROLLER_URL=http://your-controller:8080
KRONFORCE_AGENT_NAME=prod-agent-1
KRONFORCE_AGENT_TAGS=linux,prod
```

Then:

```bash
docker compose -f docker-compose.agent.yml up -d
```

## Docker Compose Files

| File | Services | Use Case |
|---|---|---|
| `docker-compose.yml` | Controller only | Production controller deployment |
| `docker-compose.agent.yml` | Agent only | Production agent on a separate machine |
| `docker-compose.full.yml` | Controller + Agent | Local development and testing |

## Building from Source

### Prerequisites

- Rust 1.75+ (with cargo)
- SQLite3 development libraries

### Build

```bash
cargo build --release
```

Binaries are at `target/release/kronforce` (controller) and `target/release/kronforce-agent` (agent).

### Run

```bash
# Controller
./target/release/kronforce

# Agent (separate terminal or machine)
KRONFORCE_AGENT_KEY=kf_your_key \
KRONFORCE_CONTROLLER_URL=http://controller:8080 \
  ./target/release/kronforce-agent
```

## Configuration Reference

### Controller

| Variable | Default | Description |
|---|---|---|
| `KRONFORCE_DB` | `kronforce.db` | SQLite database path |
| `KRONFORCE_BIND` | `0.0.0.0:8080` | Listen address |
| `KRONFORCE_TICK_SECS` | `1` | Scheduler tick interval |
| `KRONFORCE_CALLBACK_URL` | `http://{BIND}` | URL agents use to report results back |
| `KRONFORCE_HEARTBEAT_TIMEOUT_SECS` | `30` | Seconds before marking an agent offline |
| `KRONFORCE_SCRIPTS_DIR` | `./scripts` | Directory for Rhai script files |
| `KRONFORCE_BOOTSTRAP_ADMIN_KEY` | (auto) | Pre-set admin API key on first startup |
| `KRONFORCE_BOOTSTRAP_AGENT_KEY` | (auto) | Pre-set agent API key on first startup |

### Agent

| Variable | Default | Description |
|---|---|---|
| `KRONFORCE_CONTROLLER_URL` | `http://localhost:8080` | Controller to register with |
| `KRONFORCE_AGENT_KEY` | (none) | API key with `agent` role — required |
| `KRONFORCE_AGENT_NAME` | hostname | Agent display name |
| `KRONFORCE_AGENT_TAGS` | (none) | Comma-separated tags for job targeting |
| `KRONFORCE_AGENT_ADDRESS` | hostname | Address the controller uses to reach this agent |
| `KRONFORCE_AGENT_BIND` | `0.0.0.0:8081` | Agent listen address |
| `KRONFORCE_HEARTBEAT_SECS` | `10` | Heartbeat interval |

### Custom Agent (Python)

```bash
KRONFORCE_AGENT_KEY=kf_your_key \
KRONFORCE_URL=http://controller:8080 \
  python3 examples/custom_agent.py
```

## Authentication

Kronforce uses API keys with four roles:

| Role | Access |
|---|---|
| `admin` | Full access — manage jobs, agents, API keys, settings |
| `operator` | Create/edit/trigger/delete jobs, view agents |
| `viewer` | Read-only — view jobs, executions, agents, events |
| `agent` | Agent endpoints only — register, poll, heartbeat, callback |

On first startup, the controller creates bootstrap `admin` and `agent` keys and prints them to the console. Save these keys — they are not shown again.

Create additional keys in the Settings page or via API:

```bash
curl -X POST http://localhost:8080/api/keys \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "CI pipeline", "role": "operator"}'
```

## Data Persistence

### Docker Volumes

The controller stores data in two volumes:
- `kronforce-data` — SQLite database (jobs, executions, agents, events, settings)
- `kronforce-scripts` — Rhai script files

To back up:

```bash
docker compose exec controller cp /data/kronforce.db /data/backup.db
docker cp $(docker compose ps -q controller):/data/backup.db ./kronforce-backup.db
```

### Data Retention

Configure auto-purge in Settings or via API:

```bash
curl -X PUT http://localhost:8080/api/settings \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"retention_days": "14"}'
```

Default: 7 days. Completed executions, events, and queue items older than this are automatically deleted.

## Scaling Agents

Deploy multiple agents by running `docker-compose.agent.yml` on different machines with unique names:

```bash
# Machine 1
KRONFORCE_AGENT_NAME=prod-web-1 KRONFORCE_AGENT_TAGS=web,prod \
  docker compose -f docker-compose.agent.yml up -d

# Machine 2
KRONFORCE_AGENT_NAME=prod-worker-1 KRONFORCE_AGENT_TAGS=worker,prod \
  docker compose -f docker-compose.agent.yml up -d
```

Use tag-based targeting to dispatch jobs to specific groups:

```json
{"target": {"type": "tagged", "tag": "web"}}
```

Or target all agents:

```json
{"target": {"type": "all"}}
```

## Troubleshooting

### Agent can't connect

```
ERROR: authentication failed — set KRONFORCE_AGENT_KEY with a valid agent API key
```

The agent key is missing or incorrect. Check:
1. The key has role `agent` (not `viewer` or `operator`)
2. The key matches one created on the controller
3. The `KRONFORCE_CONTROLLER_URL` is reachable from the agent

### Agent shows "offline"

The agent's heartbeat timed out (default: 30 seconds). Check:
1. Network connectivity between agent and controller
2. Agent process is still running: `docker compose -f docker-compose.agent.yml logs`
3. Firewall allows HTTP traffic on the controller port

### Database locked

SQLite uses WAL mode for concurrent access, but heavy load can cause lock contention. Solutions:
1. Increase `KRONFORCE_TICK_SECS` to reduce scheduler frequency
2. Reduce data retention period to keep the database smaller
3. For high-throughput deployments, consider running fewer concurrent jobs
