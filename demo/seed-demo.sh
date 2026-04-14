#!/bin/sh
#
# Seeds the demo instance with sample data and creates a viewer key.
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

# Create groups
for group in ETL Monitoring Deploys Maintenance Reports Data-Sync; do
    curl -sf -X POST "$URL/api/jobs/groups" -H "$AUTH" -H "$CT" -d "{\"name\": \"$group\"}" > /dev/null 2>&1 || true
done
echo "  Groups created"

# Create variables
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"LAST_ETL_COUNT","value":"0"}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"DEPLOY_VERSION","value":"2.3.0"}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"ENV","value":"production"}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"API_HOST","value":"https://api.example.com"}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"DB_PASSWORD","value":"s3cret-demo","secret":true}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"ETL_WORKERS","value":"4"}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"REPORT_MONTH","value":"2026-04"}' > /dev/null 2>&1 || true
echo "  Variables created"

# --- Monitoring group ---
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"health-check","description":"Check if the API is responding",
    "task":{"type":"http","method":"GET","url":"https://httpbin.org/get","expect_status":200},
    "schedule":{"type":"cron","value":"0 */5 * * * *"},"group":"Monitoring",
    "timeout_secs":30,"notifications":{"on_failure":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"disk-usage","description":"Check disk usage on the controller",
    "task":{"type":"shell","command":"df -h / | tail -1"},
    "schedule":{"type":"cron","value":"0 */10 * * * *"},"group":"Monitoring",
    "timeout_secs":10
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"uptime-check","description":"Report system uptime",
    "task":{"type":"shell","command":"uptime"},
    "schedule":{"type":"cron","value":"0 0 * * * *"},"group":"Monitoring"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"ssl-cert-check","description":"Check SSL certificate expiry",
    "task":{"type":"http","method":"GET","url":"https://example.com","expect_status":200},
    "schedule":{"type":"cron","value":"0 0 9 * * 1"},"group":"Monitoring",
    "timeout_secs":15,"notifications":{"on_failure":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"api-latency-test","description":"Measure API response time",
    "task":{"type":"http","method":"GET","url":"https://httpbin.org/delay/1"},
    "schedule":{"type":"interval","value":{"interval_secs":600}},"group":"Monitoring",
    "timeout_secs":10,"max_concurrent":1
}' > /dev/null 2>&1 || true

# --- ETL pipeline (4-stage: extract → transform → validate → load) ---
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-extract","description":"Extract data from source system",
    "task":{"type":"shell","command":"echo \"{\\\"records\\\": 1523, \\\"status\\\": \\\"complete\\\"}\" && echo \"Extracted 1523 records\""},
    "schedule":{"type":"on_demand"},"group":"ETL","timeout_secs":300,
    "output_rules":{"extractions":[{"name":"record_count","pattern":"Extracted (\\d+) records","type":"regex","write_to_variable":"LAST_ETL_COUNT"}],"assertions":[{"pattern":"complete","message":"ETL did not complete successfully"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-transform","description":"Transform extracted data using parallel workers",
    "task":{"type":"shell","command":"echo \"Transforming {{LAST_ETL_COUNT}} records with {{ETL_WORKERS}} workers...\" && sleep 1 && echo \"Transform complete\""},
    "schedule":{"type":"on_demand"},"group":"ETL","timeout_secs":600,
    "parameters":[{"name":"ETL_WORKERS","param_type":"select","required":false,"default":"4","options":["1","2","4","8"],"description":"Number of parallel workers"}]
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-validate","description":"Run data quality checks on transformed data",
    "task":{"type":"shell","command":"echo \"Validating {{LAST_ETL_COUNT}} records...\" && echo \"null_check: passed\" && echo \"schema_check: passed\" && echo \"Validation complete: 0 errors\""},
    "schedule":{"type":"on_demand"},"group":"ETL","timeout_secs":120,
    "output_rules":{"assertions":[{"pattern":"0 errors","message":"Data validation found errors"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-load","description":"Load validated data into warehouse",
    "task":{"type":"shell","command":"echo \"Loading data into warehouse...\" && echo \"Load complete: {{LAST_ETL_COUNT}} records\""},
    "schedule":{"type":"on_demand"},"group":"ETL","timeout_secs":600
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-failure-alert","description":"Send alert when ETL pipeline fails",
    "task":{"type":"shell","command":"echo \"ETL ALERT: Pipeline failure detected. Notifying on-call team.\""},
    "schedule":{"type":"event","value":{"kind_pattern":"execution.completed","severity":"error","job_name_filter":"etl-"}},
    "group":"ETL"
}' > /dev/null 2>&1 || true

# --- Deploys pipeline (4-stage: staging → smoke-test → production → notify) ---
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-staging","description":"Deploy latest build to staging",
    "task":{"type":"shell","command":"echo \"Deploying branch={{BRANCH}} to staging...\" && sleep 2 && echo \"version=2.4.1\" && echo \"Deploy complete\""},
    "schedule":{"type":"on_demand"},"group":"Deploys","timeout_secs":300,
    "retry_max":2,"retry_delay_secs":10,"retry_backoff":2.0,
    "parameters":[{"name":"BRANCH","param_type":"text","required":false,"default":"main","description":"Git branch to deploy"},{"name":"ENVIRONMENT","param_type":"select","required":true,"default":"staging","options":["staging","staging-2"],"description":"Target environment"}],
    "output_rules":{"extractions":[{"name":"version","pattern":"version=(\\S+)","type":"regex","write_to_variable":"DEPLOY_VERSION"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-smoke-test","description":"Run smoke tests against staging",
    "task":{"type":"http","method":"GET","url":"https://httpbin.org/status/200","expect_status":200},
    "schedule":{"type":"on_demand"},"group":"Deploys","timeout_secs":120
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-production","description":"Deploy to production (requires approval)",
    "task":{"type":"shell","command":"echo \"Deploying {{DEPLOY_VERSION}} to production...\" && sleep 3 && echo \"Production deploy complete\""},
    "schedule":{"type":"on_demand"},"group":"Deploys","timeout_secs":600,
    "approval_required":true,"priority":10,
    "sla_deadline":"06:00","sla_warning_mins":15,
    "notifications":{"on_failure":true,"on_success":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-notify","description":"Notify team after production deploy",
    "task":{"type":"shell","command":"echo \"DEPLOY NOTIFICATION: {{DEPLOY_VERSION}} deployed to production\""},
    "schedule":{"type":"event","value":{"kind_pattern":"execution.completed","job_name_filter":"deploy-production"}},
    "group":"Deploys"
}' > /dev/null 2>&1 || true

# --- Maintenance pipeline (3-stage: backup → log-rotate → cache-purge) ---
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"db-backup","description":"Full database backup with compression",
    "task":{"type":"shell","command":"echo \"Starting backup...\" && sleep 1 && echo \"Backed up 42MB\" && echo \"Backup complete\""},
    "schedule":{"type":"on_demand"},"group":"Maintenance",
    "timeout_secs":1800,"max_concurrent":1,
    "notifications":{"on_failure":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"log-rotate","description":"Rotate and compress old log files",
    "task":{"type":"shell","command":"echo \"Rotating logs...\" && echo \"Compressed 5 files, freed 128MB\""},
    "schedule":{"type":"on_demand"},"group":"Maintenance"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"cache-purge","description":"Clear stale cache entries",
    "task":{"type":"shell","command":"echo \"Purging cache entries older than 24h...\" && echo \"Removed 847 entries (12MB freed)\""},
    "schedule":{"type":"on_demand"},"group":"Maintenance"
}' > /dev/null 2>&1 || true

# --- Reports group (calendar-scheduled) ---
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"monthly-report","description":"Generate monthly business metrics report",
    "task":{"type":"shell","command":"echo \"Generating report for {{REPORT_MONTH}}...\" && echo \"Revenue: $142,300\" && echo \"Users: 8,421\" && echo \"Report saved\""},
    "schedule":{"type":"calendar","value":{"anchor":"day_1","offset_days":0,"hour":8,"minute":0,"months":[],"skip_weekends":true,"holidays":[]}},"group":"Reports",
    "timeout_secs":300,
    "parameters":[{"name":"REPORT_MONTH","param_type":"text","required":false,"default":"2026-04","description":"Month to report on (YYYY-MM)"}]
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"quarterly-audit","description":"Run quarterly compliance audit",
    "task":{"type":"shell","command":"echo \"Running audit checks...\" && echo \"Access logs: OK\" && echo \"Data retention: OK\" && echo \"Encryption: OK\" && echo \"Audit passed: 3/3 checks\""},
    "schedule":{"type":"calendar","value":{"anchor":"last_day","offset_days":-2,"hour":9,"minute":0,"months":[3,6,9,12],"skip_weekends":true,"holidays":[]}},"group":"Reports",
    "timeout_secs":600,"notifications":{"on_failure":true,"on_success":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"weekly-digest","description":"Compile weekly metrics digest",
    "task":{"type":"shell","command":"echo \"Compiling weekly digest...\" && echo \"Jobs run: 342\" && echo \"Success rate: 98.5%\" && echo \"Avg duration: 12s\" && echo \"Digest sent to team\""},
    "schedule":{"type":"calendar","value":{"anchor":"first_monday","offset_days":0,"hour":7,"minute":30,"months":[],"skip_weekends":false,"holidays":[]}},"group":"Reports"
}' > /dev/null 2>&1 || true

# --- Data-Sync pipeline (4-stage: users → inventory → reconcile → notify) ---
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sync-users","description":"Sync user data from LDAP to local database",
    "task":{"type":"shell","command":"echo \"Connecting to LDAP...\" && echo \"Fetched 234 users\" && echo \"Updated: 12, Added: 3, Deactivated: 1\" && echo \"Sync complete\""},
    "schedule":{"type":"on_demand"},"group":"Data-Sync","timeout_secs":120
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sync-inventory","description":"Pull latest inventory from vendor API",
    "task":{"type":"http","method":"GET","url":"https://httpbin.org/json","expect_status":200},
    "schedule":{"type":"on_demand"},"group":"Data-Sync","timeout_secs":60,
    "output_rules":{"extractions":[{"name":"payload","pattern":"\\{.*\\}","type":"regex"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sync-reconcile","description":"Reconcile local data against vendor source of truth",
    "task":{"type":"shell","command":"echo \"Reconciling inventory...\" && echo \"Matched: 1,847 items\" && echo \"Mismatched: 3 items\" && echo \"Reconciliation complete\""},
    "schedule":{"type":"on_demand"},"group":"Data-Sync","timeout_secs":180,
    "output_rules":{"assertions":[{"pattern":"Reconciliation complete","message":"Reconciliation did not finish"}],"triggers":[{"pattern":"Mismatched: [1-9]","severity":"warning"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"sync-notify","description":"Send sync summary to data team",
    "task":{"type":"shell","command":"echo \"Data sync pipeline complete. Sending summary to data-team@example.com\""},
    "schedule":{"type":"on_demand"},"group":"Data-Sync"
}' > /dev/null 2>&1 || true
echo "  Jobs created"

# --- Set up dependencies ---
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
DEPLOY_STAGING=$(get_id "deploy-staging")
DEPLOY_SMOKE=$(get_id "deploy-smoke-test")
DEPLOY_PROD=$(get_id "deploy-production")
DB_BACKUP=$(get_id "db-backup")
LOG_ROTATE=$(get_id "log-rotate")
CACHE_PURGE=$(get_id "cache-purge")
SYNC_USERS=$(get_id "sync-users")
SYNC_INVENTORY=$(get_id "sync-inventory")
SYNC_RECONCILE=$(get_id "sync-reconcile")
SYNC_NOTIFY=$(get_id "sync-notify")

# ETL: extract → transform → validate → load (4-stage pipeline)
[ -n "$ETL_EXTRACT" ] && [ -n "$ETL_TRANSFORM" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_TRANSFORM" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$ETL_EXTRACT\",\"within_secs\":3600}]}" > /dev/null 2>&1 || true
[ -n "$ETL_TRANSFORM" ] && [ -n "$ETL_VALIDATE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_VALIDATE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$ETL_TRANSFORM\",\"within_secs\":3600}]}" > /dev/null 2>&1 || true
[ -n "$ETL_VALIDATE" ] && [ -n "$ETL_LOAD" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_LOAD" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$ETL_VALIDATE\",\"within_secs\":3600}]}" > /dev/null 2>&1 || true

# Deploys: staging → smoke-test → production (3-stage with approval gate)
[ -n "$DEPLOY_STAGING" ] && [ -n "$DEPLOY_SMOKE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$DEPLOY_SMOKE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$DEPLOY_STAGING\",\"within_secs\":7200}]}" > /dev/null 2>&1 || true
[ -n "$DEPLOY_SMOKE" ] && [ -n "$DEPLOY_PROD" ] && \
    curl -sf -X PUT "$URL/api/jobs/$DEPLOY_PROD" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$DEPLOY_SMOKE\",\"within_secs\":7200}]}" > /dev/null 2>&1 || true

# Maintenance: backup → log-rotate + cache-purge (fan-out)
[ -n "$DB_BACKUP" ] && [ -n "$LOG_ROTATE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$LOG_ROTATE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$DB_BACKUP\",\"within_secs\":7200}]}" > /dev/null 2>&1 || true
[ -n "$DB_BACKUP" ] && [ -n "$CACHE_PURGE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$CACHE_PURGE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$DB_BACKUP\",\"within_secs\":7200}]}" > /dev/null 2>&1 || true

# Data-Sync: users + inventory (parallel roots) → reconcile → notify
[ -n "$SYNC_USERS" ] && [ -n "$SYNC_INVENTORY" ] && [ -n "$SYNC_RECONCILE" ] && \
    curl -sf -X PUT "$URL/api/jobs/$SYNC_RECONCILE" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$SYNC_USERS\",\"within_secs\":3600},{\"job_id\":\"$SYNC_INVENTORY\",\"within_secs\":3600}]}" > /dev/null 2>&1 || true
[ -n "$SYNC_RECONCILE" ] && [ -n "$SYNC_NOTIFY" ] && \
    curl -sf -X PUT "$URL/api/jobs/$SYNC_NOTIFY" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\":[{\"job_id\":\"$SYNC_RECONCILE\",\"within_secs\":3600}]}" > /dev/null 2>&1 || true
echo "  Dependencies configured"

# --- Enable webhooks on deploy-staging ---
echo "Setting up webhooks..."
[ -n "$DEPLOY_STAGING" ] && \
    curl -sf -X POST "$URL/api/jobs/$DEPLOY_STAGING/webhook" -H "$AUTH" > /dev/null 2>&1 || true
echo "  Webhooks enabled"

# --- Pipeline schedules ---
echo "Setting up pipeline schedules..."
# ETL pipeline runs daily at 6am
curl -sf -X PUT "$URL/api/jobs/pipeline-schedule/ETL" -H "$AUTH" -H "$CT" \
    -d '{"schedule":{"type":"cron","value":"0 0 6 * * *"}}' > /dev/null 2>&1 || true
# Maintenance pipeline runs daily at 3am
curl -sf -X PUT "$URL/api/jobs/pipeline-schedule/Maintenance" -H "$AUTH" -H "$CT" \
    -d '{"schedule":{"type":"cron","value":"0 0 3 * * *"}}' > /dev/null 2>&1 || true
# Data-Sync pipeline runs every 4 hours
curl -sf -X PUT "$URL/api/jobs/pipeline-schedule/Data-Sync" -H "$AUTH" -H "$CT" \
    -d '{"schedule":{"type":"interval","value":{"interval_secs":14400}}}' > /dev/null 2>&1 || true
echo "  Pipeline schedules configured"

# --- Trigger jobs to generate execution history ---
echo "Generating execution history..."

# Run monitoring jobs
for job_name in health-check disk-usage uptime-check; do
    JOB_ID=$(get_id "$job_name")
    [ -n "$JOB_ID" ] && curl -sf -X POST "$URL/api/jobs/$JOB_ID/trigger" -H "$AUTH" > /dev/null 2>&1 || true
done
sleep 3

# Run ETL pipeline (triggers root, cascade handles rest)
[ -n "$ETL_EXTRACT" ] && curl -sf -X POST "$URL/api/jobs/$ETL_EXTRACT/trigger" -H "$AUTH" > /dev/null 2>&1 || true
sleep 8

# Run maintenance pipeline
[ -n "$DB_BACKUP" ] && curl -sf -X POST "$URL/api/jobs/$DB_BACKUP/trigger" -H "$AUTH" > /dev/null 2>&1 || true
sleep 5

# Run Data-Sync pipeline (two parallel roots)
[ -n "$SYNC_USERS" ] && curl -sf -X POST "$URL/api/jobs/$SYNC_USERS/trigger" -H "$AUTH" > /dev/null 2>&1 || true
[ -n "$SYNC_INVENTORY" ] && curl -sf -X POST "$URL/api/jobs/$SYNC_INVENTORY/trigger" -H "$AUTH" > /dev/null 2>&1 || true
sleep 5

# Run deploy-staging (cascade will stop at deploy-production due to approval gate)
[ -n "$DEPLOY_STAGING" ] && curl -sf -X POST "$URL/api/jobs/$DEPLOY_STAGING/trigger" -H "$AUTH" > /dev/null 2>&1 || true
sleep 5

# Second round of ETL for run history
[ -n "$ETL_EXTRACT" ] && curl -sf -X POST "$URL/api/jobs/$ETL_EXTRACT/trigger" -H "$AUTH" > /dev/null 2>&1 || true
sleep 8

# More monitoring runs
for job_name in health-check disk-usage api-latency-test; do
    JOB_ID=$(get_id "$job_name")
    [ -n "$JOB_ID" ] && curl -sf -X POST "$URL/api/jobs/$JOB_ID/trigger" -H "$AUTH" > /dev/null 2>&1 || true
done
sleep 2

# Third round of maintenance for history
[ -n "$DB_BACKUP" ] && curl -sf -X POST "$URL/api/jobs/$DB_BACKUP/trigger" -H "$AUTH" > /dev/null 2>&1 || true
sleep 5

echo "  Executions triggered"

# Create a read-only viewer key for the demo
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
    echo ""
    echo "  Share this URL for read-only access:"
    echo "  https://demo.kronforce.dev"
    echo "  (login with the viewer key above)"
fi
echo ""
echo "  Data loaded:"
echo "    - 6 groups (ETL, Monitoring, Deploys, Maintenance, Reports, Data-Sync)"
echo "    - 24 jobs across all schedule types"
echo "    - 7 variables (1 secret)"
echo "    - 4-stage ETL pipeline with data quality validation"
echo "    - 3-stage deploy pipeline with approval gate"
echo "    - 3-stage maintenance pipeline with fan-out"
echo "    - 4-stage data-sync pipeline with parallel roots"
echo "    - 3 pipeline schedules (ETL daily, Maintenance daily, Data-Sync every 4h)"
echo "    - Calendar-scheduled reports (monthly, quarterly, weekly)"
echo "    - Parameterized jobs, webhooks, output rules, event triggers"
echo "    - Multiple pipeline runs for history view"
echo ""
