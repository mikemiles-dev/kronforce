# Testing

## Running the Test Suite

```bash
cargo test          # Run all 236+ tests
cargo test --test db_tests    # Run only database tests
cargo test --test config_tests  # Run only config tests
cargo clippy        # Lint check (zero warnings required)
```

## Seed Data

Load sample jobs, groups, and variables into a running Kronforce instance for demo or manual testing.

### What Gets Created

| Category | Count | Details |
|----------|-------|---------|
| **Groups** | 4 | ETL, Monitoring, Deploys, Maintenance |
| **Jobs** | 12 | Mix of shell, HTTP, cron, on-demand, with retries and output rules |
| **Variables** | 5 | LAST_ETL_COUNT, DEPLOY_VERSION, ENV, API_HOST, ALERT_EMAIL |

### Loading Seed Data

```bash
# Start the controller
cargo run --bin kronforce

# In another terminal, load the seed data (use your admin API key)
./data/test/seed.sh kf_your_admin_key

# Or with a custom URL
KRONFORCE_URL=http://192.168.1.10:8080 ./data/test/seed.sh kf_your_admin_key
```

The script creates groups, variables, and jobs via the REST API. It's idempotent-ish — running it twice will fail on duplicate job names but won't corrupt anything.

### Sample Jobs Overview

**Monitoring group:**
- `health-check` — HTTP GET to httpbin.org every minute
- `disk-usage` — `df -h /` every 5 minutes
- `uptime-check` — `uptime` every hour
- `ssl-cert-check` — HTTPS check weekly on Monday
- `api-latency-test` — HTTP response time test every 10 minutes

**ETL group:**
- `etl-extract` — Simulates data extraction, outputs record count, writes to `LAST_ETL_COUNT` variable via output extraction
- `etl-transform` — Uses `{{LAST_ETL_COUNT}}` variable substitution
- `etl-load` — Final pipeline step, also uses variable substitution

**Deploys group:**
- `deploy-staging` — Simulates deploy with retry (2 retries, 10s delay, 2x backoff), extracts version number to `DEPLOY_VERSION` variable
- `deploy-production` — Uses `{{DEPLOY_VERSION}}` variable, 1 retry with 30s delay

**Maintenance group:**
- `db-backup` — Nightly at 3 AM, failure notifications enabled
- `log-rotate` — Nightly at 4 AM

### Features Demonstrated

| Feature | Jobs that demonstrate it |
|---------|------------------------|
| Cron scheduling | health-check, disk-usage, uptime-check, etl-extract, db-backup, log-rotate |
| HTTP tasks | health-check, ssl-cert-check, api-latency-test |
| Shell tasks | disk-usage, uptime-check, etl-*, deploy-*, db-backup, log-rotate |
| Output extraction (regex) | etl-extract, deploy-staging |
| Variable substitution | etl-transform, etl-load, deploy-production |
| Variable write-back | etl-extract (→ LAST_ETL_COUNT), deploy-staging (→ DEPLOY_VERSION) |
| Output assertions | etl-extract (checks for "complete") |
| Execution retry | deploy-staging (2 retries, exponential), deploy-production (1 retry) |
| Job notifications | db-backup (on_failure), ssl-cert-check (on_failure) |
| Job groups | All jobs assigned to one of 4 groups |

## Testing MCP

### MCP Client (Kronforce calling external MCP servers)

```bash
# Install MCP Python SDK (requires Python 3.10+)
python3 -m venv .venv
source .venv/bin/activate
pip install -r examples/requirements.txt

# Start the test MCP server
python examples/mcp_test_server.py
# Server runs on http://localhost:8000/mcp

# Create an MCP job in Kronforce:
# Task type: MCP Tool
# Server URL: http://localhost:8000/mcp
# Click "Discover Tools" → select "greet"
# Arguments: {"name": "World"}
# Trigger the job
```

### MCP Server (AI assistants calling Kronforce)

```bash
# Test the MCP server endpoint
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}'

# Discover tools
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'

# Call a tool (list jobs)
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_jobs","arguments":{}}}'

# Trigger a job via MCP
curl -X POST http://localhost:8080/mcp \
  -H "Authorization: Bearer kf_your_key" \
  -H "Content-Type: application/json" \
  -H "Accept: application/json, text/event-stream" \
  -d '{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"trigger_job","arguments":{"name":"health-check"}}}'
```

### MCP Agent (Custom agent bridging to MCP servers)

```bash
# Start the MCP test server first
python examples/mcp_test_server.py

# In another terminal, start the MCP agent
KRONFORCE_AGENT_KEY=kf_your_agent_key \
MCP_SERVER_URL=http://localhost:8000/mcp \
python examples/mcp_agent.py

# Configure task types in the dashboard:
# Agents page → mcp-agent → Add task type "mcp_tool":
#   - tool (text, required)
#   - arguments (textarea, optional)
# Create a Custom Agent job targeting this agent
```

### gRPC Agent

```bash
# Install grpcurl
brew install grpcurl

# Start a test gRPC server (e.g., grpcurl's own reflection test)
# Or use any gRPC service with reflection enabled

# Start the gRPC agent
KRONFORCE_AGENT_KEY=kf_your_agent_key python3 examples/grpc_agent.py

# Configure task types in the dashboard:
# Agents page → grpc-agent → Add task type "grpc_call":
#   - address (text, required)
#   - service (text, required)
#   - method (text, required)
#   - data (textarea, optional)
#   - plaintext (select, optional): Yes / No (TLS)
# Create a Custom Agent job targeting this agent
```

## Testing Agents

### Standard Agent

```bash
# Start the controller
cargo run --bin kronforce

# In another terminal, start an agent
KRONFORCE_CONTROLLER_URL=http://localhost:8080 \
KRONFORCE_AGENT_KEY=kf_your_agent_key \
KRONFORCE_AGENT_NAME=test-agent \
cargo run --bin kronforce-agent
```

### Custom Agent (Python)

```bash
pip install requests
KRONFORCE_AGENT_KEY=kf_your_agent_key python examples/custom_agent.py
```

## Test Coverage

| Area | Test File | Count |
|------|-----------|-------|
| Cron parser | `tests/cron_parser_tests.rs` | 38 |
| Database CRUD + groups | `tests/db_tests.rs` | 40 |
| DAG / dependencies | `tests/dag_tests.rs` | 14 |
| Query helpers/filters | `tests/helpers_tests.rs` | 14 |
| Error mapping | `tests/error_tests.rs` | 11 |
| Model serialization | `tests/model_tests.rs` | 16 |
| Factory/builders | `tests/factory_tests.rs` | 16 |
| Output rules | `tests/output_rules_tests.rs` | 17 |
| Post-execution processing | `tests/post_execution_tests.rs` | 14 |
| Variable substitution | `tests/variables_tests.rs` | 17 |
| Config parsing | `tests/config_tests.rs` | 14 |
| API key generation | `tests/api_key_tests.rs` | 5 |

## Docker Testing

```bash
# Build and run locally
docker compose -f deploy/docker/docker-compose.full.yml up -d

# View logs for bootstrap keys
docker compose -f deploy/docker/docker-compose.full.yml logs controller | grep "key"

# Load seed data
./data/test/seed.sh kf_your_admin_key

# Open dashboard
open http://localhost:8080
```

## Resetting Test Data

```bash
# Delete the database and restart
rm kronforce.db kronforce.db-wal kronforce.db-shm
cargo run --bin kronforce
# New bootstrap keys will be printed

# Or generate a new admin key without losing data
cargo run --bin kronforce -- --reset-admin-key
```
