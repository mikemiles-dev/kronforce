# Kronforce Demo Instance

Live read-only demo at [demo.kronforce.dev](https://demo.kronforce.dev).

## Deploy Your Own

### Requirements

- A VPS with Docker and Docker Compose ($5-6/month: Hetzner, DigitalOcean, Fly.io)
- A domain pointed at the VPS IP

### Setup

```bash
# Clone and configure
git clone https://github.com/mikemiles-dev/kronforce.git
cd kronforce/demo
cp .env.example .env

# Edit .env: set your admin key and domain
nano .env

# Start everything (Kronforce + Caddy + auto-seed)
docker compose up -d

# Check logs for the viewer key
docker compose logs seed
```

On first startup:
1. Kronforce starts with the admin key from `.env`
2. The seed container waits for health, then loads sample data
3. A read-only viewer key is printed in the seed logs
4. Caddy obtains a TLS certificate automatically

### What Gets Created

| Category | Count | Details |
|---|---|---|
| Groups | 4 | ETL, Monitoring, Deploys, Maintenance |
| Jobs | 12 | Shell, HTTP, cron, on-demand, with retries, output rules, approvals, SLA |
| Variables | 5 | LAST_ETL_COUNT, DEPLOY_VERSION, ENV, API_HOST, DB_PASSWORD (secret) |
| Templates | 8 | Built-in (HTTP Health Check, ETL Extract, Deploy with Approval, etc.) |
| Executions | 6+ | Sample runs triggered on startup |

### Sharing

Share the viewer key — it has read-only access (can browse jobs, executions, events, but can't create, edit, or trigger anything).

### Reset

```bash
docker compose down -v   # Destroys all data
docker compose up -d     # Fresh start with new seed
```
