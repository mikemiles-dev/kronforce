# Performance Guide

Expected limits, tuning guidance, and operational baselines for Kronforce deployments.

## Architecture Constraints

Kronforce uses a single SQLite database with WAL mode. This gives excellent read concurrency and good write throughput for most workloads, but it has inherent limits compared to client-server databases like PostgreSQL.

**Single controller** — one process handles scheduling, API, dashboard, and database. No horizontal scaling (use Litestream for HA/failover, not load distribution).

## Expected Baselines

These are conservative estimates based on SQLite WAL mode with the default r2d2 connection pool (8 connections, 5s timeout):

| Metric | Expected Range | Notes |
|---|---|---|
| **Concurrent jobs** | 50-200 | Limited by connection pool and executor threads |
| **Total jobs in DB** | 1,000-10,000 | List queries slow above ~5,000 without pagination |
| **Executions per day** | 5,000-50,000 | Depends on output size and retention |
| **API requests/sec** | 200-500 | Read-heavy workloads scale better than write-heavy |
| **Scheduler tick** | 1 second (default) | All due jobs evaluated per tick |
| **Agent count** | 10-50 standard agents | Heartbeat traffic scales linearly |
| **Custom agents polling** | 50-100 | Each polls every 2-5 seconds |
| **Database size** | 100MB-2GB typical | Grows with execution output storage |
| **Startup time** | < 2 seconds | Migration + job cache load |

## Tuning Parameters

### Connection Pool

```bash
KRONFORCE_DB_POOL_SIZE=8      # Default. Increase for more concurrent API requests.
KRONFORCE_DB_TIMEOUT_SECS=5   # Default. Increase if you see "pool timeout" errors.
```

**When to increase pool size:**
- API response times climbing under load
- "pool timeout" errors in logs
- Many concurrent agents reporting results

**Recommended range:** 4-32. Beyond 32, SQLite's write lock becomes the bottleneck, not pool size.

### Scheduler Tick

```bash
KRONFORCE_TICK_SECS=1   # Default. How often the scheduler checks for due jobs.
```

**When to increase:**
- You have 1,000+ scheduled jobs and the tick is taking > 500ms
- CPU usage is consistently high from the scheduler loop
- You don't need second-precision scheduling

Setting to `5` or `10` reduces CPU usage significantly on large job sets. Cron precision drops to match.

### Rate Limiting

```bash
KRONFORCE_RATE_LIMIT_PUBLIC=30          # Requests/min per IP
KRONFORCE_RATE_LIMIT_AUTHENTICATED=120  # Requests/min per API key
KRONFORCE_RATE_LIMIT_AGENT=600          # Requests/min per agent key
```

**Agent rate limit:** Each agent poll is one request. An agent polling every 2 seconds = 30 req/min. The default 600/min supports 20 agents polling every 2 seconds with headroom.

**Increase agent limit** if you have many agents or fast poll intervals.

### Data Retention

```bash
# Via API or Settings page
retention_days=7           # Default. Auto-purge old executions, events, queue items.
audit_retention_days=90    # Default. Audit log kept longer than operational data.
```

**Impact of high retention:**
- Database size grows linearly with execution count and output size
- List queries slow as table size increases
- Disk I/O increases for purge operations

**Recommendation:** Keep retention at 7-14 days for active environments. Use 30+ days only if you need historical output for compliance.

### Output Size

Execution stdout/stderr is stored in SQLite. Large outputs (>100KB per execution) are the #1 cause of database bloat.

**Mitigations:**
- Output is capped at 256KB per stream (stdout/stderr) — truncation flag set when exceeded
- Use output extraction to capture the data you need, then keep retention low
- For jobs producing large output, redirect to files and capture only a summary

## SQLite WAL Tuning

Kronforce sets these PRAGMAs automatically:

```sql
PRAGMA journal_mode=WAL;      -- Write-Ahead Logging for concurrent reads
PRAGMA foreign_keys=ON;       -- Referential integrity
PRAGMA busy_timeout=5000;     -- 5s wait on lock contention
```

**WAL checkpoint:** Kronforce checkpoints on graceful shutdown (SIGTERM). The background health monitor also triggers implicit checkpoints during normal operation. If the WAL file grows very large (>100MB), consider:

1. Reducing write frequency (fewer jobs, longer tick interval)
2. Increasing pool size (more connections = more checkpoint opportunities)
3. Manually running `PRAGMA wal_checkpoint(TRUNCATE)` during low-traffic periods

## Monitoring Performance

### Prometheus Metrics

Scrape `GET /metrics` for real-time performance indicators:

```
kronforce_executions_total{status="succeeded"} 1234
kronforce_executions_total{status="failed"} 56
kronforce_jobs_total 42
kronforce_agents_total 5
kronforce_db_ok 1
kronforce_db_size_bytes 524288
kronforce_db_wal_size_bytes 32768
```

**Key metrics to alert on:**
- `kronforce_db_ok = 0` — database unreachable
- `kronforce_db_wal_size_bytes > 100000000` — WAL growing too large
- `kronforce_executions_total{status="failed"}` increasing — job failures

### Health Endpoint

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

Use `status` for load balancer health checks. Monitor `db.size_bytes` for growth trends.

### Logs

```bash
RUST_LOG=kronforce=debug cargo run --bin kronforce
```

Key log patterns to watch:
- `pool error` — connection pool exhausted
- `scheduler tick took Xms` — tick latency (not logged by default, add if needed)
- `failed to dispatch to agent` — agent connectivity issues
- `WAL checkpoint` — shutdown/checkpoint events

## Scaling Strategies

### Vertical Scaling (Single Controller)

1. **More CPU** — helps scheduler tick and concurrent request handling
2. **More RAM** — SQLite caches in memory; more RAM = fewer disk reads
3. **Fast SSD** — SQLite performance is I/O bound; NVMe helps significantly
4. **Increase pool size** — more concurrent DB connections (up to 32)

### Horizontal Scaling (Multiple Controllers)

Kronforce doesn't support active-active clustering. For scaling beyond a single controller:

1. **Shard by group** — run separate Kronforce instances for different job groups
2. **Dedicated agent pools** — route heavy workloads to dedicated agents with tags
3. **Offload output** — redirect large job output to external storage (S3, NFS), capture only summaries in Kronforce
4. **Litestream for HA** — not for scaling, but for failover and disaster recovery

### When to Consider Alternatives

If you consistently hit these limits, Kronforce may not be the right fit:

- **>10,000 concurrent jobs** — consider Airflow with Kubernetes executor
- **>100 agents** — consider a message queue-based system
- **>10GB database** — consider PostgreSQL-backed schedulers
- **Multi-region active-active** — consider cloud-native schedulers (AWS EventBridge, GCP Cloud Scheduler)

## Quick Reference

| Symptom | Likely Cause | Fix |
|---|---|---|
| Slow API responses | Pool exhaustion | Increase `DB_POOL_SIZE` |
| "pool timeout" errors | Too many concurrent writes | Increase `DB_POOL_SIZE` and `DB_TIMEOUT_SECS` |
| High CPU usage | Fast scheduler tick + many jobs | Increase `TICK_SECS` to 5-10 |
| Large database file | High retention + large output | Reduce `retention_days`, truncate output |
| Large WAL file | Heavy write load | Checkpoint manually or reduce write frequency |
| Agent dispatch failures | Network/TLS issues | Check agent connectivity, verify keys |
| Slow job list page | Too many jobs | Use filters, groups, and pagination |
