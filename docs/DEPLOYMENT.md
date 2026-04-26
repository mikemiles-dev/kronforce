# Deployment

## Download Pre-Built Binaries

Pre-built binaries are available on the [GitHub Releases](https://github.com/mikemiles-dev/kronforce/releases) page for tagged versions.

| Platform | Controller | Agent |
|---|---|---|
| Linux x86_64 | `kronforce-linux-amd64` | `kronforce-agent-linux-amd64` |
| macOS ARM64 (Apple Silicon) | `kronforce-darwin-arm64` | `kronforce-agent-darwin-arm64` |
| Windows x86_64 | `kronforce-windows-amd64.exe` | `kronforce-agent-windows-amd64.exe` |

Linux ARM64 is available via the Docker image (`linux/arm64`).

### Windows Support

Shell commands execute via `cmd /C` on Windows instead of `sh -c` on Unix. Most features work natively, but some task types depend on external CLI tools.

**Works out of the box:**

| Feature | Notes |
|---|---|
| Controller + Dashboard | Full functionality |
| HTTP tasks | Native (reqwest, no shell needed) |
| Rhai scripts | Embedded engine, all built-in functions work |
| Shell tasks | Uses `cmd /C` — write commands for Windows (e.g., `dir`, `echo`, PowerShell) |
| FTP/SFTP tasks | Uses `curl` (built into Windows 10+) |

**Requires CLI tools on PATH:**

| Task Type | Tool Needed | Install |
|---|---|---|
| SQL (Postgres) | `psql` | [postgresql.org/download/windows](https://www.postgresql.org/download/windows/) |
| SQL (MySQL) | `mysql` | [dev.mysql.com/downloads](https://dev.mysql.com/downloads/mysql/) |
| SQL (SQLite) | `sqlite3` | [sqlite.org/download](https://www.sqlite.org/download.html) |
| Kafka | `kafka-console-producer` | [kafka.apache.org](https://kafka.apache.org/downloads) |
| RabbitMQ | `amqp-publish` | [github.com/selency/amqp-publish](https://github.com/selency/amqp-publish) |
| MQTT | `mosquitto_pub` | [mosquitto.org/download](https://mosquitto.org/download/) |
| Redis | `redis-cli` | [github.com/microsoftarchive/redis](https://github.com/microsoftarchive/redis/releases) |

**Not supported on Windows:**

| Feature | Reason |
|---|---|
| `run_as` (sudo) | No equivalent on Windows. Jobs always run as the current user. |

**Mixed environments (recommended):**

Run the controller on Windows and agents on Linux — this works perfectly. The controller handles scheduling, the API, and the dashboard (all cross-platform). Agents handle task execution on their own OS. Shell commands, SQL tools, and messaging CLI tools only need to be available on the machines running the agents, not the controller. Target jobs to agents using `{"target": {"type": "any"}}` or `{"target": {"type": "tagged", "tag": "linux"}}` to keep execution on Unix.

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

## Docker Image

Pre-built Docker images are published to GitHub Container Registry on each release:

```bash
# Pull the latest image
docker pull ghcr.io/mikemiles-dev/kronforce:latest

# Or a specific version
docker pull ghcr.io/mikemiles-dev/kronforce:0.1.0-alpha
```

Available for `linux/amd64` and `linux/arm64`. The image includes both the controller and agent binaries.

## Quick Start (Docker Compose)

### Local Development — Full Stack

Run the controller and a standard agent together:

```bash
docker compose -f deploy/docker/docker-compose.full.yml up -d
```

On first startup, keys are auto-generated and printed to the container logs. Retrieve them:

```bash
docker compose -f deploy/docker/docker-compose.full.yml logs controller | grep "key"
```

- Dashboard: http://localhost:8080

To pre-set keys instead of auto-generating:

```bash
KRONFORCE_ADMIN_KEY=kf_myadminkey KRONFORCE_AGENT_KEY=kf_myagentkey \
  docker compose -f deploy/docker/docker-compose.full.yml up -d
```

### Production — Controller

```bash
docker compose -f deploy/docker/docker-compose.yml up -d
```

On first startup, the controller auto-generates bootstrap API keys and prints them to the logs:

```bash
docker compose -f deploy/docker/docker-compose.yml logs controller | grep "key"
```

Save these keys immediately — they are only shown once.

You'll see two keys:
- **Admin key** (`kf_...`) — use this to log into the dashboard
- **Agent key** (`kf_...`) — give this to agents so they can connect

To pre-set keys instead of auto-generating:

```bash
KRONFORCE_BOOTSTRAP_ADMIN_KEY=kf_myadminkey \
KRONFORCE_BOOTSTRAP_AGENT_KEY=kf_myagentkey \
  docker compose -f deploy/docker/docker-compose.yml up -d
```

Bootstrap keys are only used on first startup with an empty database. After that, manage keys in the Settings page.

### Production — Agent (separate machine)

On the machine where you want to run an agent:

```bash
KRONFORCE_AGENT_KEY=kf_your_agent_key \
KRONFORCE_CONTROLLER_URL=http://your-controller:8080 \
  docker compose -f deploy/docker/docker-compose.agent.yml up -d
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
docker compose -f deploy/docker/docker-compose.agent.yml up -d
```

## Docker Compose Files

| File | Services | Use Case |
|---|---|---|
| `deploy/docker/docker-compose.yml` | Controller only | Production controller deployment |
| `deploy/docker/docker-compose.agent.yml` | Agent only | Production agent on a separate machine |
| `deploy/docker/docker-compose.full.yml` | Controller + Agent | Local development and testing |

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
| `KRONFORCE_RATE_LIMIT_ENABLED` | `true` | Enable/disable API rate limiting |
| `KRONFORCE_RATE_LIMIT_PUBLIC` | `30` | Max requests/min for public endpoints (per IP) |
| `KRONFORCE_RATE_LIMIT_AUTHENTICATED` | `120` | Max requests/min for authenticated endpoints (per key) |
| `KRONFORCE_RATE_LIMIT_AGENT` | `600` | Max requests/min for agent endpoints (per key) |
| `KRONFORCE_BOOTSTRAP_ADMIN_KEY` | (auto) | Pre-set admin API key on first startup |
| `KRONFORCE_BOOTSTRAP_AGENT_KEY` | (auto) | Pre-set agent API key on first startup |
| `KRONFORCE_OIDC_ISSUER` | (none) | OIDC issuer URL (enables SSO) |
| `KRONFORCE_OIDC_CLIENT_ID` | (none) | OAuth2 client ID |
| `KRONFORCE_OIDC_CLIENT_SECRET` | (none) | OAuth2 client secret |
| `KRONFORCE_OIDC_REDIRECT_URI` | auto | OAuth2 callback URL |
| `KRONFORCE_OIDC_ROLE_CLAIM` | `groups` | Claim path for role mapping |
| `KRONFORCE_OIDC_ADMIN_VALUES` | (none) | Claim values → admin role |
| `KRONFORCE_OIDC_OPERATOR_VALUES` | (none) | Claim values → operator role |
| `KRONFORCE_OIDC_DEFAULT_ROLE` | `viewer` | Fallback role |
| `KRONFORCE_OIDC_SESSION_TTL_SECS` | `86400` | SSO session lifetime |
| `KRONFORCE_AI_API_KEY` | (none) | Anthropic or OpenAI API key — enables AI job creation. Can also be set from Settings UI without restart. |
| `KRONFORCE_AI_PROVIDER` | `anthropic` | AI provider: `anthropic` or `openai`. Can also be set from Settings UI. |
| `KRONFORCE_AI_MODEL` | auto | Model override (default: `claude-sonnet-4-5-20250514` / `gpt-4o`). Can also be set from Settings UI. |
| `KRONFORCE_ENCRYPTION_KEY` | (none) | AES-256 key for encrypting connection credentials at rest |
| `KRONFORCE_DEMO_MODE` | `false` | Read-only demo mode — disables auth |
| `KRONFORCE_MCP_ENABLED` | `true` | Enable MCP server endpoint at `POST /mcp` |

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
# Unrestricted operator key
curl -X POST http://localhost:8080/api/keys \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "CI pipeline", "role": "operator"}'

# Team-scoped key (can only access ETL and Monitoring groups)
curl -X POST http://localhost:8080/api/keys \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"name": "data-team", "role": "operator", "allowed_groups": ["ETL", "Monitoring"]}'
```

### Team Isolation via Group Scoping

API keys can be restricted to specific job groups using `allowed_groups`. A scoped key can only view, create, edit, and trigger jobs in its allowed groups. Admin keys always see everything.

This enables team-level isolation without full multi-tenancy:
- **Platform team**: Admin key, sees all groups
- **Data team**: Operator key scoped to `["ETL", "Data Quality"]`
- **SRE team**: Operator key scoped to `["Monitoring", "Deploys"]`
- **Dashboard viewer**: Viewer key scoped to `["Monitoring"]`

### OIDC/SSO (Optional)

Enable enterprise SSO by setting OIDC environment variables. Both API key and SSO login work side-by-side.

```bash
KRONFORCE_OIDC_ISSUER=https://login.microsoftonline.com/{tenant}/v2.0 \
KRONFORCE_OIDC_CLIENT_ID=your-client-id \
KRONFORCE_OIDC_CLIENT_SECRET=your-client-secret \
KRONFORCE_OIDC_ADMIN_VALUES=KF-Admins \
KRONFORCE_OIDC_OPERATOR_VALUES=KF-Operators \
cargo run --bin kronforce
```

The login screen shows "Sign in with SSO" when OIDC is configured. Users are redirected to your IdP, and on successful login a session cookie is set.

**Role mapping**: The controller reads the claim at `KRONFORCE_OIDC_ROLE_CLAIM` (default: `groups`) from the ID token and maps values to roles:

| Variable | Example | Effect |
|---|---|---|
| `KRONFORCE_OIDC_ADMIN_VALUES` | `KF-Admins,platform-team` | Users with these group values get `admin` role |
| `KRONFORCE_OIDC_OPERATOR_VALUES` | `KF-Operators,dev-team` | Users with these group values get `operator` role |
| `KRONFORCE_OIDC_DEFAULT_ROLE` | `viewer` | Everyone else gets this role (default: `viewer`) |

**Nested claims**: Use dot-notation for nested claim paths (e.g., `resource_access.kronforce.roles` for Keycloak).

**Sessions**: Stored in SQLite, expire after `KRONFORCE_OIDC_SESSION_TTL_SECS` (default: 24 hours). Expired sessions are cleaned up automatically. Agents always use API keys, not SSO.

## Data Persistence

### Docker Volumes

The controller stores data in two volumes:
- `kronforce-data` — SQLite database (jobs, executions, agents, events, settings)
- `kronforce-scripts` — Rhai script files

To back up:

```bash
docker compose -f deploy/docker/docker-compose.yml exec controller cp /data/kronforce.db /data/backup.db
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

### Audit Log Retention

Audit log entries have separate retention from events/executions. Configure via Settings:

```bash
curl -X PUT http://localhost:8080/api/settings \
  -H "Authorization: Bearer kf_admin_key" \
  -H "Content-Type: application/json" \
  -d '{"audit_retention_days": "365"}'
```

Default: 90 days. The audit log is not affected by the regular `retention_days` setting.

## High Availability

Kronforce uses SQLite, which means a single controller instance. For disaster recovery and failover, use [Litestream](https://litestream.io/) to continuously replicate the database to S3-compatible storage.

### How It Works

1. Litestream runs as a sidecar, watching the SQLite WAL file
2. WAL changes are streamed to S3 every second
3. Full snapshots are taken hourly
4. On startup, Litestream restores from the latest replica if no local database exists
5. On shutdown, Kronforce checkpoints the WAL for a clean handoff

### Setup

```bash
# Set your S3 credentials
export LITESTREAM_REPLICA_URL=s3://my-bucket/kronforce
export AWS_ACCESS_KEY_ID=your-key
export AWS_SECRET_ACCESS_KEY=your-secret
export AWS_REGION=us-east-1

# Start with replication
docker compose -f deploy/docker/docker-compose.ha.yml up -d

# View logs
docker compose -f deploy/docker/docker-compose.ha.yml logs -f
```

### Failover Procedure

1. **Primary goes down**: The S3 replica has data up to ~1 second before failure
2. **Start a new controller**: Point it at the same S3 bucket
3. **Litestream restores automatically**: The database is rebuilt from S3 before Kronforce starts
4. **Update DNS/load balancer**: Route traffic to the new instance
5. **Agents reconnect**: Agents will re-register on their next heartbeat

```bash
# On the new machine
export LITESTREAM_REPLICA_URL=s3://my-bucket/kronforce  # Same bucket
docker compose -f deploy/docker/docker-compose.ha.yml up -d
```

### S3-Compatible Storage

Litestream supports any S3-compatible storage:
- **AWS S3**
- **MinIO** (self-hosted)
- **DigitalOcean Spaces**
- **Backblaze B2**
- **Google Cloud Storage** (via S3 compatibility)

### Health Endpoint

The enhanced health endpoint reports database status:

```bash
curl http://localhost:8080/api/health
```

```json
{
  "status": "ok",
  "db": {
    "ok": true,
    "size_bytes": 524288,
    "wal_size_bytes": 32768,
    "pool_size": 8
  }
}
```

Use `status: "ok"` for load balancer health checks. A `"degraded"` status means the database is unreachable.

### Graceful Shutdown

On SIGTERM/SIGINT, Kronforce:
1. Stops accepting new connections
2. Checkpoints the WAL (flushes all pending writes to the main database file)
3. Exits cleanly

This ensures Litestream captures the final state. Docker's `stop_grace_period: 30s` gives time for the checkpoint.

## Scaling Agents

Deploy multiple agents by running `docker-compose.agent.yml` on different machines with unique names:

```bash
# Machine 1
KRONFORCE_AGENT_NAME=prod-web-1 KRONFORCE_AGENT_TAGS=web,prod \
  docker compose -f deploy/docker/docker-compose.agent.yml up -d

# Machine 2
KRONFORCE_AGENT_NAME=prod-worker-1 KRONFORCE_AGENT_TAGS=worker,prod \
  docker compose -f deploy/docker/docker-compose.agent.yml up -d
```

Use tag-based targeting to dispatch jobs to specific groups:

```json
{"target": {"type": "tagged", "tag": "web"}}
```

Or target all agents:

```json
{"target": {"type": "all"}}
```

## Cloud Deployment

Kronforce runs anywhere Docker runs. The same image works on every cloud provider.

### AWS

**EC2:**
```bash
# SSH into your EC2 instance
sudo yum install -y docker && sudo systemctl start docker   # Amazon Linux
# or: sudo apt install -y docker.io                          # Ubuntu

docker run -d --name kronforce \
  -p 8080:8080 \
  -v kronforce-data:/data \
  ghcr.io/mikemiles-dev/kronforce:latest
```

**ECS (Fargate):** Create a task definition with image `ghcr.io/mikemiles-dev/kronforce:latest`, port mapping 8080, and an EFS volume mounted at `/data` for persistence.

**ECS (EC2):** Same as Fargate but with an EC2 launch type. Mount an EBS volume at `/data`.

### Azure

**VM:**
```bash
# SSH into your Azure VM
sudo apt install -y docker.io

docker run -d --name kronforce \
  -p 8080:8080 \
  -v kronforce-data:/data \
  ghcr.io/mikemiles-dev/kronforce:latest
```

**Container Instances (ACI):**
```bash
az container create \
  --resource-group mygroup \
  --name kronforce \
  --image ghcr.io/mikemiles-dev/kronforce:latest \
  --ports 8080 \
  --cpu 1 --memory 1
```

**App Service:** Create a Web App for Containers, set the image to `ghcr.io/mikemiles-dev/kronforce:latest`, and configure port 8080.

### Google Cloud

**Compute Engine:**
```bash
# SSH into your GCE instance
sudo apt install -y docker.io

docker run -d --name kronforce \
  -p 8080:8080 \
  -v kronforce-data:/data \
  ghcr.io/mikemiles-dev/kronforce:latest
```

**Cloud Run:**
```bash
gcloud run deploy kronforce \
  --image ghcr.io/mikemiles-dev/kronforce:latest \
  --port 8080 \
  --allow-unauthenticated
```

Note: Cloud Run is stateless — the SQLite database resets on each deployment. Use Litestream replication to S3/GCS for persistence, or use a VM with a persistent disk instead.

### DigitalOcean

**Droplet:**
```bash
# SSH into your droplet
curl -fsSL https://get.docker.com | sh

docker run -d --name kronforce \
  -p 8080:8080 \
  -v kronforce-data:/data \
  ghcr.io/mikemiles-dev/kronforce:latest
```

**App Platform:** Create a new app, select "Docker Hub / GHCR", enter `ghcr.io/mikemiles-dev/kronforce:latest`, set HTTP port to 8080.

### Agents on Cloud

Deploy agents on any cloud VM the same way:

```bash
docker run -d --name kronforce-agent \
  -e KRONFORCE_CONTROLLER_URL=https://your-controller:8080 \
  -e KRONFORCE_AGENT_KEY=kf_your_agent_key \
  -e KRONFORCE_AGENT_NAME=cloud-agent-1 \
  -e KRONFORCE_AGENT_TAGS=aws,prod \
  ghcr.io/mikemiles-dev/kronforce:latest \
  kronforce-agent
```

Agents connect outbound to the controller — no inbound ports needed except for standard agents (which need port 8081 open for the controller to push jobs).

### Persistence Note

SQLite stores all data in a single file at `/data/kronforce.db`. On cloud platforms:
- **VMs**: Use a persistent disk/volume mounted at `/data`
- **Containers (ECS, Cloud Run, ACI)**: Mount a network volume (EFS, Filestore, Azure Files) at `/data`
- **Serverless**: Use Litestream to replicate to S3/GCS — see the [High Availability](#high-availability) section

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
2. Agent process is still running: `docker compose -f deploy/docker/docker-compose.agent.yml logs`
3. Firewall allows HTTP traffic on the controller port

### Database locked

SQLite uses WAL mode for concurrent access, but heavy load can cause lock contention. Solutions:
1. Increase `KRONFORCE_TICK_SECS` to reduce scheduler frequency
2. Reduce data retention period to keep the database smaller
3. For high-throughput deployments, consider running fewer concurrent jobs
