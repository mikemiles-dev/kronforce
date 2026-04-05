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
for group in ETL Monitoring Deploys Maintenance; do
    curl -sf -X POST "$URL/api/jobs/groups" -H "$AUTH" -H "$CT" -d "{\"name\": \"$group\"}" > /dev/null 2>&1 || true
done
echo "  Groups created"

# Create variables
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"LAST_ETL_COUNT","value":"0"}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"DEPLOY_VERSION","value":"1.0.0"}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"ENV","value":"production"}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"API_HOST","value":"https://api.example.com"}' > /dev/null 2>&1 || true
curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d '{"name":"DB_PASSWORD","value":"s3cret-demo","secret":true}' > /dev/null 2>&1 || true
echo "  Variables created"

# Create sample jobs
curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"health-check","task":{"type":"http","method":"GET","url":"https://httpbin.org/get","expect_status":200},
    "schedule":{"type":"cron","value":"0 */5 * * * *"},"group":"Monitoring",
    "notifications":{"on_failure":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"disk-usage","task":{"type":"shell","command":"df -h / | tail -1"},
    "schedule":{"type":"cron","value":"0 */10 * * * *"},"group":"Monitoring"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"uptime-check","task":{"type":"shell","command":"uptime"},
    "schedule":{"type":"cron","value":"0 0 * * * *"},"group":"Monitoring"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-extract","task":{"type":"shell","command":"echo \"Extracted 142 records complete\""},
    "schedule":{"type":"cron","value":"0 0 6 * * *"},"group":"ETL",
    "output_rules":{"extractions":[{"name":"record_count","pattern":"Extracted (\\d+) records","type":"regex","write_to_variable":"LAST_ETL_COUNT","target":"variable"}],"assertions":[{"pattern":"complete"}],"triggers":[]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-transform","task":{"type":"shell","command":"echo \"Transforming {{LAST_ETL_COUNT}} records\""},
    "schedule":{"type":"on_demand"},"group":"ETL"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"etl-load","task":{"type":"shell","command":"echo \"Loading {{LAST_ETL_COUNT}} records to {{ENV}}\""},
    "schedule":{"type":"on_demand"},"group":"ETL"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-staging","task":{"type":"shell","command":"echo \"Deploying version=2.4.1 to staging\""},
    "schedule":{"type":"on_demand"},"group":"Deploys",
    "retry_max":2,"retry_delay_secs":10,"retry_backoff":2.0,
    "output_rules":{"extractions":[{"name":"version","pattern":"version=(\\S+)","type":"regex","write_to_variable":"DEPLOY_VERSION","target":"variable"}]}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"deploy-production","task":{"type":"shell","command":"echo \"Deploying {{DEPLOY_VERSION}} to production\""},
    "schedule":{"type":"on_demand"},"group":"Deploys",
    "approval_required":true,"priority":10,
    "sla_deadline":"06:00","sla_warning_mins":15,
    "notifications":{"on_failure":true,"on_success":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"db-backup","task":{"type":"shell","command":"echo \"Backup completed: 524MB\""},
    "schedule":{"type":"cron","value":"0 0 3 * * *"},"group":"Maintenance",
    "timeout_secs":3600,"notifications":{"on_failure":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"log-rotate","task":{"type":"shell","command":"echo \"Rotated 12 log files\""},
    "schedule":{"type":"cron","value":"0 0 4 * * *"},"group":"Maintenance"
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"ssl-cert-check","task":{"type":"http","method":"GET","url":"https://example.com","expect_status":200},
    "schedule":{"type":"cron","value":"0 0 9 * * 1"},"group":"Monitoring",
    "notifications":{"on_failure":true}
}' > /dev/null 2>&1 || true

curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d '{
    "name":"api-latency-test","task":{"type":"http","method":"GET","url":"https://httpbin.org/delay/1"},
    "schedule":{"type":"cron","value":"0 */10 * * * *"},"group":"Monitoring",
    "output_rules":{"triggers":[{"pattern":"timeout|error","severity":"error"}]}
}' > /dev/null 2>&1 || true
echo "  Jobs created"

# Trigger a few jobs to generate execution history
echo "Triggering sample executions..."
for job_name in health-check disk-usage uptime-check etl-extract deploy-staging db-backup; do
    JOB_ID=$(curl -sf "$URL/api/jobs?per_page=100" -H "$AUTH" | python3 -c "
import json,sys
data=json.load(sys.stdin)
for j in data.get('data',[]):
    if j['name']=='$job_name':
        print(j['id'])
        break
" 2>/dev/null || true)
    if [ -n "$JOB_ID" ]; then
        curl -sf -X POST "$URL/api/jobs/$JOB_ID/trigger" -H "$AUTH" > /dev/null 2>&1 || true
        sleep 1
    fi
done
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
echo "    - 4 groups (ETL, Monitoring, Deploys, Maintenance)"
echo "    - 12 jobs with various schedules and features"
echo "    - 5 variables (1 secret)"
echo "    - 8 job templates (built-in)"
echo "    - Sample executions triggered"
echo ""
