#!/bin/sh
#
# Seeds the demo instance with comprehensive sample data.
# Runs once on first startup via the seed container.
set -e

URL="${KRONFORCE_URL:-http://kronforce:8080}"
KEY="${ADMIN_KEY}"

echo "Waiting for Kronforce at $URL..."
for i in $(seq 1 30); do
    if curl -sf "$URL/api/health" > /dev/null 2>&1; then
        break
    fi
    sleep 1
done

AUTH="Authorization: Bearer $KEY"
CT="Content-Type: application/json"

# Check if already seeded (jobs exist)
EXISTING=$(curl -sf "$URL/api/jobs?per_page=1" -H "$AUTH" 2>/dev/null | grep -o '"total":[0-9]*' | grep -o '[0-9]*' || echo "0")
if [ "$EXISTING" != "0" ]; then
    echo "Demo already seeded ($EXISTING jobs). Skipping."
    exit 0
fi

echo "Seeding demo data..."

# =============================================
# GROUPS
# =============================================
for group in ETL Monitoring Deploys Maintenance Reports Data-Sync Security Notifications; do
    curl -sf -X POST "$URL/api/jobs/groups" -H "$AUTH" -H "$CT" -d "{\"name\": \"$group\"}" > /dev/null 2>&1 || true
done
echo "  Groups created (8)"

# =============================================
# VARIABLES
# =============================================
for v in \
    '{"name":"LAST_ETL_COUNT","value":"0"}' \
    '{"name":"DEPLOY_VERSION","value":"2.3.0"}' \
    '{"name":"ENV","value":"production"}' \
    '{"name":"API_HOST","value":"https://api.example.com"}' \
    '{"name":"DB_PASSWORD","value":"s3cret-demo","secret":true}' \
    '{"name":"ETL_WORKERS","value":"4"}' \
    '{"name":"REPORT_MONTH","value":"2026-04"}' \
    '{"name":"SLACK_WEBHOOK","value":"https://hooks.slack.example.com/T00/B00/xxxx","secret":true}' \
    '{"name":"DEPLOY_BRANCH","value":"main"}' \
    '{"name":"LAST_AUDIT_STATUS","value":"passed"}' \
    '{"name":"INVENTORY_COUNT","value":"1847"}' \
    '{"name":"CACHE_HIT_RATE","value":"94.2"}' \
; do
    curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d "$v" > /dev/null 2>&1 || true
done
echo "  Variables created (12)"

# =============================================
# SCRIPTS (Rhai + Dockerfile)
# =============================================
curl -sf -X PUT "$URL/api/scripts/etl-helper" -H "$AUTH" -H "$CT" -d '{
    "content": "// ETL helper script\nlet count = 1523;\nlet status = \"complete\";\nprint(`Extracted ${count} records: ${status}`);\ncount",
    "script_type": "rhai"
}' > /dev/null 2>&1 || true

curl -sf -X PUT "$URL/api/scripts/report-generator" -H "$AUTH" -H "$CT" -d '{
    "content": "FROM python:3.11-slim\nRUN pip install --no-cache-dir pandas jinja2\nCOPY generate_report.py /app/\nWORKDIR /app\nCMD [\"python\", \"generate_report.py\"]",
    "script_type": "dockerfile"
}' > /dev/null 2>&1 || true

curl -sf -X PUT "$URL/api/scripts/healthcheck" -H "$AUTH" -H "$CT" -d '{
    "content": "// Health check helper\nlet endpoints = [\"api\", \"web\", \"worker\"];\nfor ep in endpoints {\n    print(`Checking ${ep}... OK`);\n}\nprint(\"All services healthy\");\ntrue",
    "script_type": "rhai"
}' > /dev/null 2>&1 || true
echo "  Scripts created (3)"

# =============================================
# MONITORING GROUP (7 jobs)
# =============================================
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"health-check","description":"Check if the primary API is responding with 200 OK",
    "task":{"type":"http","method":"GET","url":"https://httpbin.org/get","expect_status":200},
    "schedule":{"type":"cron","value":"0 */5 * * * *"},"group":"Monitoring",
    "timeout_secs":30,"notifications":{"on_failure":true},
    "output_rules":{"triggers":[{"pattern":"error|timeout","severity":"error"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"disk-usage","description":"Check disk usage and alert if above 85%",
    "task":{"type":"shell","command":"echo \"Filesystem      Size  Used Avail Use%\" && echo \"/dev/sda1       50G   32G   18G  64%\" && echo \"Disk usage: 64%\""},
    "schedule":{"type":"cron","value":"0 */10 * * * *"},"group":"Monitoring",
    "timeout_secs":10,
    "output_rules":{"extractions":[{"name":"usage_pct","pattern":"Disk usage: (\\d+)%","type":"regex"}],"triggers":[{"pattern":"[89][0-9]%|100%","severity":"warning"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"uptime-check","description":"Report system uptime and load average",
    "task":{"type":"shell","command":"echo \"System uptime: 42 days, 7:23\" && echo \"Load average: 0.42 0.38 0.35\" && echo \"Memory: 2048MB/4096MB (50%)\" && echo \"Status: healthy\""},
    "schedule":{"type":"cron","value":"0 0 * * * *"},"group":"Monitoring",
    "output_rules":{"assertions":[{"pattern":"healthy","message":"System is not in healthy state"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"ssl-cert-check","description":"Verify SSL certificates have not expired",
    "task":{"type":"http","method":"GET","url":"https://example.com","expect_status":200},
    "schedule":{"type":"cron","value":"0 0 9 * * 1"},"group":"Monitoring",
    "timeout_secs":15,"notifications":{"on_failure":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"api-latency-test","description":"Measure API response latency (must be < 2s)",
    "task":{"type":"http","method":"GET","url":"https://httpbin.org/delay/1","expect_status":200},
    "schedule":{"type":"interval","value":{"interval_secs":600}},"group":"Monitoring",
    "timeout_secs":10,"max_concurrent":1,
    "output_rules":{"triggers":[{"pattern":"timeout","severity":"error"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"dns-resolution","description":"Verify DNS resolves correctly for all production domains",
    "task":{"type":"shell","command":"echo \"Resolving api.example.com... 93.184.216.34 (OK)\" && echo \"Resolving cdn.example.com... 93.184.216.35 (OK)\" && echo \"Resolving app.example.com... 93.184.216.36 (OK)\" && echo \"All 3 domains resolved successfully\""},
    "schedule":{"type":"cron","value":"0 */30 * * * *"},"group":"Monitoring",
    "timeout_secs":15,
    "output_rules":{"assertions":[{"pattern":"resolved successfully","message":"DNS resolution failed"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"cache-hit-rate","description":"Monitor cache hit rate and extract metrics",
    "task":{"type":"shell","command":"echo \"Cache stats:\" && echo \"  Hits: 14,823\" && echo \"  Misses: 912\" && echo \"  Hit rate: 94.2%\" && echo \"  Evictions: 47\""},
    "schedule":{"type":"interval","value":{"interval_secs":1800}},"group":"Monitoring",
    "timeout_secs":10,
    "output_rules":{"extractions":[{"name":"hit_rate","pattern":"Hit rate: ([\\d.]+)%","type":"regex","write_to_variable":"CACHE_HIT_RATE"}],"triggers":[{"pattern":"Hit rate: [0-7][0-9]","severity":"warning"}]}
}' > /dev/null 2>&1 || true

# =============================================
# ETL PIPELINE (5-stage: extract → transform → validate → load → archive)
# =============================================
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-extract","description":"Extract raw data from source databases and APIs",
    "task":{"type":"shell","command":"echo \"Connecting to source DB...\" && echo \"{\\\"records\\\": 1523, \\\"tables\\\": 4, \\\"status\\\": \\\"complete\\\"}\" && echo \"Extracted 1523 records from 4 tables\""},
    "schedule":{"type":"on_demand"},"group":"ETL","timeout_secs":300,
    "output_rules":{"extractions":[{"name":"record_count","pattern":"Extracted (\\d+) records","type":"regex","write_to_variable":"LAST_ETL_COUNT"}],"assertions":[{"pattern":"complete","message":"ETL extraction did not complete"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-transform","description":"Transform and normalize extracted data using parallel workers",
    "task":{"type":"shell","command":"echo \"Transforming {{LAST_ETL_COUNT}} records with {{ETL_WORKERS}} workers...\" && echo \"Deduplication: removed 23 duplicates\" && echo \"Normalization: 1500 records processed\" && echo \"Transform complete\""},
    "schedule":{"type":"on_demand"},"group":"ETL","timeout_secs":600,
    "parameters":[{"name":"ETL_WORKERS","param_type":"select","required":false,"default":"4","options":["1","2","4","8","16"],"description":"Number of parallel transform workers"}]
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-validate","description":"Run data quality checks — null detection, schema conformance, referential integrity",
    "task":{"type":"shell","command":"echo \"Running validation suite on {{LAST_ETL_COUNT}} records...\" && echo \"  null_check: passed (0 nulls in required fields)\" && echo \"  schema_check: passed (all types match)\" && echo \"  ref_integrity: passed (0 orphaned references)\" && echo \"  range_check: passed (all values in bounds)\" && echo \"Validation complete: 0 errors, 0 warnings\""},
    "schedule":{"type":"on_demand"},"group":"ETL","timeout_secs":120,
    "output_rules":{"assertions":[{"pattern":"0 errors","message":"Data validation found errors — pipeline halted"}],"triggers":[{"pattern":"[1-9]+ warnings","severity":"warning"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-load","description":"Load validated data into the analytics warehouse",
    "task":{"type":"shell","command":"echo \"Loading data into warehouse...\" && echo \"Batch 1/3: 500 rows inserted\" && echo \"Batch 2/3: 500 rows inserted\" && echo \"Batch 3/3: 500 rows inserted\" && echo \"Load complete: {{LAST_ETL_COUNT}} records in 3 batches\""},
    "schedule":{"type":"on_demand"},"group":"ETL","timeout_secs":600
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-archive","description":"Archive raw source files to cold storage after successful load",
    "task":{"type":"shell","command":"echo \"Archiving source files...\" && echo \"Compressed 4 files (23MB -> 8MB)\" && echo \"Uploaded to s3://archive/etl/2026-04-15/\" && echo \"Archive complete\""},
    "schedule":{"type":"on_demand"},"group":"ETL","timeout_secs":300
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-failure-alert","description":"Send alert when any ETL stage fails",
    "task":{"type":"shell","command":"echo \"ETL ALERT: Pipeline failure detected\" && echo \"Notifying #data-eng Slack channel and on-call team\""},
    "schedule":{"type":"event","value":{"kind_pattern":"execution.completed","severity":"error","job_name_filter":"etl-"}},
    "group":"ETL"
}' > /dev/null 2>&1 || true

# =============================================
# DEPLOYS PIPELINE (4-stage: staging → smoke → production → post-deploy)
# =============================================
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-staging","description":"Deploy latest build to staging environment",
    "task":{"type":"shell","command":"echo \"Deploying branch={{BRANCH}} to {{ENVIRONMENT}}...\" && echo \"Pulling image: ghcr.io/company/app:latest\" && echo \"Rolling update: 3/3 pods ready\" && echo \"version=2.4.1\" && echo \"Deploy to staging complete\""},
    "schedule":{"type":"on_demand"},"group":"Deploys","timeout_secs":300,
    "retry_max":2,"retry_delay_secs":10,"retry_backoff":2.0,
    "parameters":[{"name":"BRANCH","param_type":"text","required":false,"default":"main","description":"Git branch to deploy"},{"name":"ENVIRONMENT","param_type":"select","required":true,"default":"staging","options":["staging","staging-2","staging-perf"],"description":"Target staging environment"}],
    "output_rules":{"extractions":[{"name":"version","pattern":"version=(\\S+)","type":"regex","write_to_variable":"DEPLOY_VERSION"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-smoke-test","description":"Run smoke tests and integration checks against staging",
    "task":{"type":"shell","command":"echo \"Running smoke tests against staging...\" && echo \"  GET /health: 200 OK (42ms)\" && echo \"  GET /api/users: 200 OK (128ms)\" && echo \"  POST /api/orders: 201 Created (234ms)\" && echo \"  GET /api/search: 200 OK (89ms)\" && echo \"All 4 smoke tests passed\""},
    "schedule":{"type":"on_demand"},"group":"Deploys","timeout_secs":120,
    "output_rules":{"assertions":[{"pattern":"All.*passed","message":"Smoke tests failed — blocking production deploy"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-production","description":"Deploy to production (requires manual approval)",
    "task":{"type":"shell","command":"echo \"Deploying {{DEPLOY_VERSION}} to production...\" && echo \"Blue/green switch: canary at 10%\" && echo \"Canary metrics OK after 30s\" && echo \"Promoting to 100%\" && echo \"Production deploy complete: {{DEPLOY_VERSION}}\""},
    "schedule":{"type":"on_demand"},"group":"Deploys","timeout_secs":600,
    "approval_required":true,"priority":10,
    "sla_deadline":"06:00","sla_warning_mins":15,
    "notifications":{"on_failure":true,"on_success":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-post-checks","description":"Run post-deployment health checks and metric verification",
    "task":{"type":"shell","command":"echo \"Post-deploy verification for {{DEPLOY_VERSION}}...\" && echo \"  Error rate: 0.02% (threshold: 1%)\" && echo \"  P99 latency: 245ms (threshold: 500ms)\" && echo \"  Active connections: 1,247\" && echo \"All post-deploy checks passed\""},
    "schedule":{"type":"on_demand"},"group":"Deploys","timeout_secs":120,
    "output_rules":{"assertions":[{"pattern":"All post-deploy checks passed","message":"Post-deploy checks failed — consider rollback"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-notify","description":"Notify team on Slack after production deploy",
    "task":{"type":"shell","command":"echo \"DEPLOY NOTIFICATION: {{DEPLOY_VERSION}} deployed to production\" && echo \"Sent to #deploys and #general\""},
    "schedule":{"type":"event","value":{"kind_pattern":"execution.completed","job_name_filter":"deploy-production"}},
    "group":"Deploys"
}' > /dev/null 2>&1 || true

# =============================================
# MAINTENANCE PIPELINE (4-stage: backup → log-rotate + cache-purge → vacuum)
# =============================================
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"db-backup","description":"Full database backup with compression and checksum verification",
    "task":{"type":"shell","command":"echo \"Starting database backup...\" && echo \"Dumping tables: users, orders, products, events\" && echo \"Compressing: 524MB -> 142MB (73% reduction)\" && echo \"Checksum: sha256=a1b2c3d4e5f6...\" && echo \"Uploaded to s3://backups/db/2026-04-15.sql.gz\" && echo \"Backup complete: 142MB\""},
    "schedule":{"type":"on_demand"},"group":"Maintenance",
    "timeout_secs":1800,"max_concurrent":1,
    "notifications":{"on_failure":true},
    "output_rules":{"extractions":[{"name":"backup_size","pattern":"Backup complete: (\\S+)","type":"regex"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"log-rotate","description":"Rotate, compress, and archive old log files",
    "task":{"type":"shell","command":"echo \"Rotating logs...\" && echo \"  app.log: 234MB -> rotated (compressed: 18MB)\" && echo \"  access.log: 512MB -> rotated (compressed: 41MB)\" && echo \"  error.log: 8MB -> rotated (compressed: 1MB)\" && echo \"Compressed 3 files, freed 694MB\" && echo \"Archived to s3://logs/archive/\""},
    "schedule":{"type":"on_demand"},"group":"Maintenance"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"cache-purge","description":"Clear stale cache entries and rebuild hot cache",
    "task":{"type":"shell","command":"echo \"Purging cache entries older than 24h...\" && echo \"Removed 847 stale entries (12MB freed)\" && echo \"Rebuilding hot cache: 200 keys preloaded\" && echo \"New hit rate: 96.1%\""},
    "schedule":{"type":"on_demand"},"group":"Maintenance",
    "output_rules":{"extractions":[{"name":"cache_hit_rate","pattern":"hit rate: ([\\d.]+)%","type":"regex","write_to_variable":"CACHE_HIT_RATE"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"db-vacuum","description":"Run VACUUM and ANALYZE on database tables",
    "task":{"type":"shell","command":"echo \"Running VACUUM on 4 tables...\" && echo \"  users: reclaimed 12MB\" && echo \"  orders: reclaimed 45MB\" && echo \"  events: reclaimed 89MB\" && echo \"  products: OK (no dead tuples)\" && echo \"Running ANALYZE...\" && echo \"Statistics updated for 4 tables\" && echo \"Total space reclaimed: 146MB\""},
    "schedule":{"type":"on_demand"},"group":"Maintenance","timeout_secs":3600
}' > /dev/null 2>&1 || true

# =============================================
# REPORTS GROUP (4 jobs — calendar-scheduled)
# =============================================
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"monthly-report","description":"Generate monthly business metrics report on the 1st of each month",
    "task":{"type":"shell","command":"echo \"Generating report for {{REPORT_MONTH}}...\" && echo \"Revenue: $142,300 (+8.2% MoM)\" && echo \"Active users: 8,421 (+342)\" && echo \"Orders: 3,847\" && echo \"Churn rate: 2.1%\" && echo \"Report saved to /reports/{{REPORT_MONTH}}.pdf\""},
    "schedule":{"type":"calendar","value":{"anchor":"day_1","offset_days":0,"hour":8,"minute":0,"months":[],"skip_weekends":true,"holidays":[]}},"group":"Reports",
    "timeout_secs":300,
    "parameters":[{"name":"REPORT_MONTH","param_type":"text","required":false,"default":"2026-04","description":"Month to report on (YYYY-MM)"},{"name":"FORMAT","param_type":"select","required":false,"default":"pdf","options":["pdf","csv","html"],"description":"Report output format"}]
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"quarterly-audit","description":"Run quarterly compliance audit (Mar, Jun, Sep, Dec — 2 days before month end)",
    "task":{"type":"shell","command":"echo \"Running Q2 2026 compliance audit...\" && echo \"  Access control review: PASSED\" && echo \"  Data retention policy: PASSED\" && echo \"  Encryption at rest: PASSED\" && echo \"  PII handling: PASSED\" && echo \"  Backup verification: PASSED\" && echo \"Audit complete: 5/5 checks passed\""},
    "schedule":{"type":"calendar","value":{"anchor":"last_day","offset_days":-2,"hour":9,"minute":0,"months":[3,6,9,12],"skip_weekends":true,"holidays":[]}},"group":"Reports",
    "timeout_secs":600,
    "notifications":{"on_failure":true,"on_success":true},
    "output_rules":{"extractions":[{"name":"audit_result","pattern":"Audit complete: (.*?)$","type":"regex","write_to_variable":"LAST_AUDIT_STATUS"}],"assertions":[{"pattern":"checks passed","message":"Compliance audit failed — immediate review required"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"weekly-digest","description":"Compile weekly ops digest every Monday morning",
    "task":{"type":"shell","command":"echo \"Compiling weekly ops digest...\" && echo \"Week of 2026-04-13:\" && echo \"  Jobs executed: 342\" && echo \"  Success rate: 98.5%\" && echo \"  Avg duration: 12s\" && echo \"  Failures: 5 (etl-validate x2, deploy-staging x3)\" && echo \"  Slowest job: db-backup (4m 23s)\" && echo \"Digest sent to ops-team@example.com\""},
    "schedule":{"type":"calendar","value":{"anchor":"first_monday","offset_days":0,"hour":7,"minute":30,"months":[],"skip_weekends":false,"holidays":[]}},"group":"Reports"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sla-report","description":"Generate SLA compliance report on the 15th of each month",
    "task":{"type":"shell","command":"echo \"SLA Compliance Report\" && echo \"Period: 2026-04-01 to 2026-04-15\" && echo \"  API uptime: 99.97% (target: 99.9%)\" && echo \"  P99 latency: 312ms (target: 500ms)\" && echo \"  Error rate: 0.03% (target: 0.1%)\" && echo \"  Incident response: 4m avg (target: 15m)\" && echo \"All SLA targets met\""},
    "schedule":{"type":"calendar","value":{"anchor":"day_15","offset_days":0,"hour":10,"minute":0,"months":[],"skip_weekends":true,"holidays":[]}},"group":"Reports",
    "notifications":{"on_success":true}
}' > /dev/null 2>&1 || true

# =============================================
# DATA-SYNC PIPELINE (5-stage: users + inventory → reconcile → notify → metrics)
# =============================================
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sync-users","description":"Sync user directory from corporate LDAP",
    "task":{"type":"shell","command":"echo \"Connecting to ldap://ldap.corp.example.com...\" && echo \"Fetched 234 user records\" && echo \"Diff: Updated: 12, Added: 3, Deactivated: 1, Unchanged: 218\" && echo \"User sync complete\""},
    "schedule":{"type":"on_demand"},"group":"Data-Sync","timeout_secs":120
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sync-inventory","description":"Pull latest product inventory from vendor API",
    "task":{"type":"http","method":"GET","url":"https://httpbin.org/json","expect_status":200},
    "schedule":{"type":"on_demand"},"group":"Data-Sync","timeout_secs":60,
    "output_rules":{"extractions":[{"name":"payload","pattern":"\\{.*\\}","type":"regex","write_to_variable":"INVENTORY_COUNT"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sync-reconcile","description":"Reconcile local data against vendor source and flag mismatches",
    "task":{"type":"shell","command":"echo \"Reconciling inventory against vendor feed...\" && echo \"Matched: 1,847 items\" && echo \"Price changes: 23 items updated\" && echo \"New items: 5 added\" && echo \"Discontinued: 2 flagged\" && echo \"Mismatched: 3 items (requires review)\" && echo \"Reconciliation complete\""},
    "schedule":{"type":"on_demand"},"group":"Data-Sync","timeout_secs":180,
    "output_rules":{"assertions":[{"pattern":"Reconciliation complete","message":"Reconciliation did not finish"}],"triggers":[{"pattern":"Mismatched: [1-9]","severity":"warning"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sync-notify","description":"Send sync summary to data team with diff stats",
    "task":{"type":"shell","command":"echo \"Data sync pipeline complete\" && echo \"Summary: 234 users synced, 1847 inventory items reconciled\" && echo \"Alerts: 3 mismatched items flagged for review\" && echo \"Sent to data-team@example.com and #data-ops Slack\""},
    "schedule":{"type":"on_demand"},"group":"Data-Sync"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sync-metrics","description":"Update sync metrics dashboard and detect drift trends",
    "task":{"type":"shell","command":"echo \"Updating sync metrics...\" && echo \"  Sync latency: 23s (p50), 45s (p99)\" && echo \"  Data freshness: 4h 12m\" && echo \"  Drift rate: 0.16% (stable)\" && echo \"Metrics pushed to Grafana\""},
    "schedule":{"type":"on_demand"},"group":"Data-Sync"
}' > /dev/null 2>&1 || true

# =============================================
# SECURITY GROUP (4 jobs — mixed schedules)
# =============================================
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"vuln-scan","description":"Run vulnerability scan on container images",
    "task":{"type":"shell","command":"echo \"Scanning 12 container images...\" && echo \"  ghcr.io/company/api:2.4.1 - 0 critical, 2 high, 5 medium\" && echo \"  ghcr.io/company/web:1.8.0 - 0 critical, 0 high, 3 medium\" && echo \"  ghcr.io/company/worker:3.1.0 - 0 critical, 1 high, 4 medium\" && echo \"Scan complete: 0 critical, 3 high, 12 medium across 12 images\""},
    "schedule":{"type":"cron","value":"0 0 2 * * *"},"group":"Security",
    "timeout_secs":600,"notifications":{"on_failure":true},
    "output_rules":{"triggers":[{"pattern":"[1-9]+ critical","severity":"error"}],"assertions":[{"pattern":"0 critical","message":"Critical vulnerabilities detected"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"secret-rotation","description":"Rotate API keys and database credentials on schedule",
    "task":{"type":"shell","command":"echo \"Rotating credentials...\" && echo \"  DB read-replica password: rotated\" && echo \"  Redis auth token: rotated\" && echo \"  AWS IAM temp credentials: refreshed\" && echo \"3 credentials rotated successfully\""},
    "schedule":{"type":"calendar","value":{"anchor":"day_1","offset_days":0,"hour":3,"minute":0,"months":[],"skip_weekends":false,"holidays":[]}},"group":"Security",
    "timeout_secs":120,"notifications":{"on_failure":true,"on_success":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"access-audit","description":"Audit active API keys and access patterns for anomalies",
    "task":{"type":"shell","command":"echo \"Auditing API access...\" && echo \"Active keys: 14\" && echo \"Keys unused >30d: 2 (flagged for review)\" && echo \"Anomalous patterns: 0\" && echo \"Geo-suspicious: 0\" && echo \"Access audit passed\""},
    "schedule":{"type":"cron","value":"0 0 8 * * *"},"group":"Security",
    "output_rules":{"triggers":[{"pattern":"Anomalous patterns: [1-9]","severity":"warning"},{"pattern":"Geo-suspicious: [1-9]","severity":"error"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"security-alert","description":"React to security events and escalate",
    "task":{"type":"shell","command":"echo \"SECURITY ALERT: Escalating to security team\" && echo \"Incident logged in PagerDuty\""},
    "schedule":{"type":"event","value":{"kind_pattern":"execution.completed","severity":"error","job_name_filter":"vuln-scan"}},
    "group":"Security"
}' > /dev/null 2>&1 || true

# =============================================
# NOTIFICATIONS GROUP (3 jobs — event-triggered)
# =============================================
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"slack-on-failure","description":"Send Slack notification on any job failure",
    "task":{"type":"shell","command":"echo \"Sending failure notification to #alerts...\" && echo \"Channel: #alerts\" && echo \"Message: Job execution failed\" && echo \"Notification sent\""},
    "schedule":{"type":"event","value":{"kind_pattern":"execution.completed","severity":"error"}},
    "group":"Notifications"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"pagerduty-critical","description":"Page on-call when critical infrastructure fails",
    "task":{"type":"shell","command":"echo \"PAGING ON-CALL: Critical infrastructure failure\" && echo \"Severity: P1\" && echo \"PagerDuty incident created: INC-4821\""},
    "schedule":{"type":"event","value":{"kind_pattern":"execution.completed","severity":"error","job_name_filter":"health-check"}},
    "group":"Notifications","priority":10
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"daily-summary","description":"Send daily execution summary to management",
    "task":{"type":"shell","command":"echo \"Daily Kronforce Summary\" && echo \"Date: 2026-04-15\" && echo \"Total executions: 87\" && echo \"Succeeded: 84 (96.6%)\" && echo \"Failed: 3\" && echo \"Avg duration: 8.4s\" && echo \"Sent to management@example.com\""},
    "schedule":{"type":"cron","value":"0 0 18 * * 1-5"},"group":"Notifications"
}' > /dev/null 2>&1 || true

echo "  Jobs created (35)"

# =============================================
# DEPENDENCIES
# =============================================
echo "Setting up dependencies..."

get_id() {
    curl -sf "$URL/api/jobs?per_page=100" -H "$AUTH" | python3 -c "
import json,sys
for j in json.load(sys.stdin).get('data',[]):
    if j['name']=='$1':
        print(j['id'])
        break
" 2>/dev/null || true
}

ETL_EXTRACT=$(get_id "etl-extract")
ETL_TRANSFORM=$(get_id "etl-transform")
ETL_VALIDATE=$(get_id "etl-validate")
ETL_LOAD=$(get_id "etl-load")
ETL_ARCHIVE=$(get_id "etl-archive")
DEPLOY_STAGING=$(get_id "deploy-staging")
DEPLOY_SMOKE=$(get_id "deploy-smoke-test")
DEPLOY_PROD=$(get_id "deploy-production")
DEPLOY_POST=$(get_id "deploy-post-checks")
DB_BACKUP=$(get_id "db-backup")
LOG_ROTATE=$(get_id "log-rotate")
CACHE_PURGE=$(get_id "cache-purge")
DB_VACUUM=$(get_id "db-vacuum")
SYNC_USERS=$(get_id "sync-users")
SYNC_INVENTORY=$(get_id "sync-inventory")
SYNC_RECONCILE=$(get_id "sync-reconcile")
SYNC_NOTIFY=$(get_id "sync-notify")
SYNC_METRICS=$(get_id "sync-metrics")

# ETL: extract → transform → validate → load → archive (5-stage)
[ -n "$ETL_EXTRACT" ] && [ -n "$ETL_TRANSFORM" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_TRANSFORM" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$ETL_EXTRACT\",\"within_secs\":3600}]}" > /dev/null 2>&1
[ -n "$ETL_TRANSFORM" ] && [ -n "$ETL_VALIDATE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_VALIDATE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$ETL_TRANSFORM\",\"within_secs\":3600}]}" > /dev/null 2>&1
[ -n "$ETL_VALIDATE" ] && [ -n "$ETL_LOAD" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_LOAD" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$ETL_VALIDATE\",\"within_secs\":3600}]}" > /dev/null 2>&1
[ -n "$ETL_LOAD" ] && [ -n "$ETL_ARCHIVE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_ARCHIVE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$ETL_LOAD\",\"within_secs\":3600}]}" > /dev/null 2>&1

# Deploys: staging → smoke → production → post-checks (4-stage with approval)
[ -n "$DEPLOY_STAGING" ] && [ -n "$DEPLOY_SMOKE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$DEPLOY_SMOKE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$DEPLOY_STAGING\",\"within_secs\":7200}]}" > /dev/null 2>&1
[ -n "$DEPLOY_SMOKE" ] && [ -n "$DEPLOY_PROD" ] && \
    curl -sf -X PUT "$URL/api/jobs/$DEPLOY_PROD" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$DEPLOY_SMOKE\",\"within_secs\":7200}]}" > /dev/null 2>&1
[ -n "$DEPLOY_PROD" ] && [ -n "$DEPLOY_POST" ] && \
    curl -sf -X PUT "$URL/api/jobs/$DEPLOY_POST" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$DEPLOY_PROD\",\"within_secs\":7200}]}" > /dev/null 2>&1

# Maintenance: backup → log-rotate + cache-purge (fan-out) → vacuum (fan-in)
[ -n "$DB_BACKUP" ] && [ -n "$LOG_ROTATE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$LOG_ROTATE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$DB_BACKUP\",\"within_secs\":7200}]}" > /dev/null 2>&1
[ -n "$DB_BACKUP" ] && [ -n "$CACHE_PURGE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$CACHE_PURGE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$DB_BACKUP\",\"within_secs\":7200}]}" > /dev/null 2>&1
[ -n "$LOG_ROTATE" ] && [ -n "$CACHE_PURGE" ] && [ -n "$DB_VACUUM" ] && \
    curl -sf -X PUT "$URL/api/jobs/$DB_VACUUM" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$LOG_ROTATE\",\"within_secs\":7200},{\"job_id\":\"$CACHE_PURGE\",\"within_secs\":7200}]}" > /dev/null 2>&1

# Data-Sync: users + inventory (parallel) → reconcile → notify + metrics (fan-out)
[ -n "$SYNC_USERS" ] && [ -n "$SYNC_INVENTORY" ] && [ -n "$SYNC_RECONCILE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$SYNC_RECONCILE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$SYNC_USERS\",\"within_secs\":3600},{\"job_id\":\"$SYNC_INVENTORY\",\"within_secs\":3600}]}" > /dev/null 2>&1
[ -n "$SYNC_RECONCILE" ] && [ -n "$SYNC_NOTIFY" ] && \
    curl -sf -X PUT "$URL/api/jobs/$SYNC_NOTIFY" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$SYNC_RECONCILE\",\"within_secs\":3600}]}" > /dev/null 2>&1
[ -n "$SYNC_RECONCILE" ] && [ -n "$SYNC_METRICS" ] && \
    curl -sf -X PUT "$URL/api/jobs/$SYNC_METRICS" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$SYNC_RECONCILE\",\"within_secs\":3600}]}" > /dev/null 2>&1
echo "  Dependencies configured"

# =============================================
# WEBHOOKS
# =============================================
echo "Setting up webhooks..."
[ -n "$DEPLOY_STAGING" ] && curl -sf -X POST "$URL/api/jobs/$DEPLOY_STAGING/webhook" -H "$AUTH" > /dev/null 2>&1 || true
echo "  Webhooks enabled"

# =============================================
# PIPELINE SCHEDULES
# =============================================
echo "Setting up pipeline schedules..."
curl -sf -X PUT "$URL/api/jobs/pipeline-schedule/ETL" -H "$AUTH" -H "$CT" \
    -d '{"schedule":{"type":"cron","value":"0 0 6 * * *"}}' > /dev/null 2>&1 || true
curl -sf -X PUT "$URL/api/jobs/pipeline-schedule/Maintenance" -H "$AUTH" -H "$CT" \
    -d '{"schedule":{"type":"cron","value":"0 0 3 * * *"}}' > /dev/null 2>&1 || true
curl -sf -X PUT "$URL/api/jobs/pipeline-schedule/Data-Sync" -H "$AUTH" -H "$CT" \
    -d '{"schedule":{"type":"interval","value":{"interval_secs":14400}}}' > /dev/null 2>&1 || true
echo "  Pipeline schedules configured (3)"

# =============================================
# GENERATE EXECUTION HISTORY (multiple pipeline runs)
# =============================================
echo "Generating execution history (this takes ~90 seconds)..."

trigger() { JID=$(get_id "$1"); [ -n "$JID" ] && curl -sf -X POST "$URL/api/jobs/$JID/trigger" -H "$AUTH" > /dev/null 2>&1; }

# Run 1: All pipelines
trigger "etl-extract"
trigger "db-backup"
trigger "sync-users"; trigger "sync-inventory"
trigger "health-check"; trigger "disk-usage"; trigger "uptime-check"
trigger "vuln-scan"
sleep 10

# Run 2: ETL + Monitoring
trigger "etl-extract"
trigger "health-check"; trigger "api-latency-test"; trigger "dns-resolution"; trigger "cache-hit-rate"
sleep 10

# Run 3: Maintenance + Data-Sync
trigger "db-backup"
trigger "sync-users"; trigger "sync-inventory"
trigger "deploy-staging"
sleep 10

# Run 4: ETL again + security
trigger "etl-extract"
trigger "access-audit"
trigger "health-check"; trigger "disk-usage"
sleep 10

# Run 5: All pipelines again for rich history
trigger "db-backup"
trigger "sync-users"; trigger "sync-inventory"
trigger "health-check"; trigger "uptime-check"; trigger "ssl-cert-check"
sleep 10

# Run 6: ETL
trigger "etl-extract"
trigger "cache-hit-rate"; trigger "dns-resolution"
sleep 10

# Run 7: Monitoring burst
trigger "health-check"; trigger "disk-usage"; trigger "api-latency-test"
trigger "uptime-check"
sleep 5

echo "  Execution history generated"

# =============================================
# VIEWER KEY
# =============================================
echo ""
echo "Creating demo viewer key..."
VIEWER_RESP=$(curl -sf -X POST "$URL/api/keys" -H "$AUTH" -H "$CT" -d '{"name":"demo-viewer","role":"viewer"}' 2>/dev/null || echo "")
VIEWER_KEY=$(echo "$VIEWER_RESP" | python3 -c "import json,sys; print(json.load(sys.stdin).get('raw_key',''))" 2>/dev/null || echo "")

echo ""
echo "============================================="
echo "  Demo seeded successfully!"
echo "============================================="
echo ""
echo "  Admin key:  ${KEY}"
if [ -n "$VIEWER_KEY" ]; then
    echo "  Viewer key: ${VIEWER_KEY}"
fi
echo ""
echo "  Data loaded:"
echo "    - 8 groups"
echo "    - 35 jobs across all schedule types"
echo "    - 12 variables (2 secrets)"
echo "    - 3 scripts (2 Rhai, 1 Dockerfile)"
echo "    - Pipelines:"
echo "        ETL: 5-stage (extract->transform->validate->load->archive)"
echo "        Deploys: 4-stage with approval gate"
echo "        Maintenance: 4-stage with fan-out and fan-in"
echo "        Data-Sync: 5-stage with parallel roots and fan-out"
echo "    - 3 pipeline schedules"
echo "    - 4 calendar-scheduled jobs"
echo "    - 4 event-triggered jobs"
echo "    - 3 parameterized jobs"
echo "    - 7 rounds of pipeline runs for history"
echo ""
