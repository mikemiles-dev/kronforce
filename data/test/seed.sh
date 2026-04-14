#!/bin/bash
#
# Seed Kronforce with test data
#
# Usage:
#   ./data/test/seed.sh kf_your_admin_key             # provide key as argument
#   KRONFORCE_URL=http://host:8080 ./data/test/seed.sh kf_key
#
# Creates: 6 groups, 24 jobs (various types/schedules/pipelines), 7 global variables
# Requires: curl, python3 or jq

URL="${KRONFORCE_URL:-http://localhost:8080}"
KEY="${1:-}"
SEED_FILE="$(dirname "$0")/seed_data.json"

if [ -z "$KEY" ]; then
    echo "Usage: $0 <admin-api-key>"
    echo ""
    echo "Provide your admin API key as the first argument."
    echo "Get it from the Kronforce dashboard: Settings > API Keys"
    exit 1
fi

if [ ! -f "$SEED_FILE" ]; then
    echo "Error: seed_data.json not found at $SEED_FILE"
    exit 1
fi

AUTH="Authorization: Bearer $KEY"
CT="Content-Type: application/json"

echo "Seeding Kronforce at $URL"
echo "========================="

# Check connectivity
if ! curl -sf "$URL/api/health" > /dev/null 2>&1; then
    echo "Error: Cannot reach $URL/api/health"
    echo "Is Kronforce running?"
    exit 1
fi

echo ""

# Cleanup stale groups from previous buggy seed runs
curl -sf -X PUT "$URL/api/jobs/rename-group" -H "$AUTH" -H "$CT" -d '{"old_name":"0","new_name":"Default"}' > /dev/null 2>&1 || true

# Create groups
echo "Creating groups..."
for group in ETL Monitoring Deploys Maintenance Reports Data-Sync; do
    curl -sf -X POST "$URL/api/jobs/groups" -H "$AUTH" -H "$CT" -d "{\"name\": \"$group\"}" > /dev/null 2>&1 && echo "  + Group: $group" || echo "  ~ Group: $group (may already exist)"
done

echo ""

# Create variables
echo "Creating variables..."
VARS=$(python3 -c "
import json
d = json.load(open('$SEED_FILE'))
for v in d['variables']:
    print(json.dumps(v))
" 2>/dev/null || jq -c '.variables[]' "$SEED_FILE")
echo "$VARS" | while read -r var; do
    name=$(echo "$var" | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])" 2>/dev/null || echo "$var" | jq -r '.name')
    curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d "$var" > /dev/null 2>&1 && echo "  + Variable: $name" || echo "  ~ Variable: $name (may already exist)"
done

echo ""

# Create jobs
echo "Creating jobs..."
JOBS=$(python3 -c "
import json
d = json.load(open('$SEED_FILE'))
for j in d['jobs']:
    print(json.dumps(j))
" 2>/dev/null || jq -c '.jobs[]' "$SEED_FILE")
echo "$JOBS" | while read -r job; do
    name=$(echo "$job" | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])" 2>/dev/null || echo "$job" | jq -r '.name')
    resp=$(curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d "$job" 2>&1)
    if [ $? -eq 0 ]; then
        echo "  + Job: $name"
    else
        echo "  ~ Job: $name (may already exist or invalid)"
    fi
done

# Set up dependencies between jobs
echo ""
echo "Setting up dependencies..."

get_job_id() {
    curl -sf "$URL/api/jobs?per_page=100" -H "$AUTH" | python3 -c "
import json, sys
for j in json.load(sys.stdin).get('data', []):
    if j['name'] == '$1':
        print(j['id'])
        break
" 2>/dev/null
}

ETL_EXTRACT_ID=$(get_job_id "etl-extract")
ETL_TRANSFORM_ID=$(get_job_id "etl-transform")
ETL_VALIDATE_ID=$(get_job_id "etl-validate")
ETL_LOAD_ID=$(get_job_id "etl-load")
DEPLOY_STAGING_ID=$(get_job_id "deploy-staging")
DEPLOY_SMOKE_ID=$(get_job_id "deploy-smoke-test")
DEPLOY_PROD_ID=$(get_job_id "deploy-production")
DB_BACKUP_ID=$(get_job_id "db-backup")
LOG_ROTATE_ID=$(get_job_id "log-rotate")
CACHE_PURGE_ID=$(get_job_id "cache-purge")
SYNC_USERS_ID=$(get_job_id "sync-users")
SYNC_INVENTORY_ID=$(get_job_id "sync-inventory")
SYNC_RECONCILE_ID=$(get_job_id "sync-reconcile")
SYNC_NOTIFY_ID=$(get_job_id "sync-notify")

# ETL: extract → transform → validate → load
[ -n "$ETL_EXTRACT_ID" ] && [ -n "$ETL_TRANSFORM_ID" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_TRANSFORM_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$ETL_EXTRACT_ID\", \"within_secs\": 3600}]}" > /dev/null 2>&1 \
        && echo "  + etl-transform depends on etl-extract" || echo "  ~ etl-transform dependency failed"
[ -n "$ETL_TRANSFORM_ID" ] && [ -n "$ETL_VALIDATE_ID" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_VALIDATE_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$ETL_TRANSFORM_ID\", \"within_secs\": 3600}]}" > /dev/null 2>&1 \
        && echo "  + etl-validate depends on etl-transform" || echo "  ~ etl-validate dependency failed"
[ -n "$ETL_VALIDATE_ID" ] && [ -n "$ETL_LOAD_ID" ] && \
    curl -sf -X PUT "$URL/api/jobs/$ETL_LOAD_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$ETL_VALIDATE_ID\", \"within_secs\": 3600}]}" > /dev/null 2>&1 \
        && echo "  + etl-load depends on etl-validate" || echo "  ~ etl-load dependency failed"

# Deploys: staging → smoke-test → production
[ -n "$DEPLOY_STAGING_ID" ] && [ -n "$DEPLOY_SMOKE_ID" ] && \
    curl -sf -X PUT "$URL/api/jobs/$DEPLOY_SMOKE_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$DEPLOY_STAGING_ID\", \"within_secs\": 7200}]}" > /dev/null 2>&1 \
        && echo "  + deploy-smoke-test depends on deploy-staging" || echo "  ~ deploy-smoke-test dependency failed"
[ -n "$DEPLOY_SMOKE_ID" ] && [ -n "$DEPLOY_PROD_ID" ] && \
    curl -sf -X PUT "$URL/api/jobs/$DEPLOY_PROD_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$DEPLOY_SMOKE_ID\", \"within_secs\": 7200}]}" > /dev/null 2>&1 \
        && echo "  + deploy-production depends on deploy-smoke-test" || echo "  ~ deploy-production dependency failed"

# Maintenance: backup → log-rotate + cache-purge (fan-out)
[ -n "$DB_BACKUP_ID" ] && [ -n "$LOG_ROTATE_ID" ] && \
    curl -sf -X PUT "$URL/api/jobs/$LOG_ROTATE_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$DB_BACKUP_ID\", \"within_secs\": 7200}]}" > /dev/null 2>&1 \
        && echo "  + log-rotate depends on db-backup" || echo "  ~ log-rotate dependency failed"
[ -n "$DB_BACKUP_ID" ] && [ -n "$CACHE_PURGE_ID" ] && \
    curl -sf -X PUT "$URL/api/jobs/$CACHE_PURGE_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$DB_BACKUP_ID\", \"within_secs\": 7200}]}" > /dev/null 2>&1 \
        && echo "  + cache-purge depends on db-backup" || echo "  ~ cache-purge dependency failed"

# Data-Sync: users + inventory (parallel) → reconcile → notify
[ -n "$SYNC_USERS_ID" ] && [ -n "$SYNC_INVENTORY_ID" ] && [ -n "$SYNC_RECONCILE_ID" ] && \
    curl -sf -X PUT "$URL/api/jobs/$SYNC_RECONCILE_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$SYNC_USERS_ID\", \"within_secs\": 3600}, {\"job_id\": \"$SYNC_INVENTORY_ID\", \"within_secs\": 3600}]}" > /dev/null 2>&1 \
        && echo "  + sync-reconcile depends on sync-users + sync-inventory" || echo "  ~ sync-reconcile dependency failed"
[ -n "$SYNC_RECONCILE_ID" ] && [ -n "$SYNC_NOTIFY_ID" ] && \
    curl -sf -X PUT "$URL/api/jobs/$SYNC_NOTIFY_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$SYNC_RECONCILE_ID\", \"within_secs\": 3600}]}" > /dev/null 2>&1 \
        && echo "  + sync-notify depends on sync-reconcile" || echo "  ~ sync-notify dependency failed"

# --- Webhooks ---
echo ""
echo "Setting up webhooks..."
[ -n "$DEPLOY_STAGING_ID" ] && \
    curl -sf -X POST "$URL/api/jobs/$DEPLOY_STAGING_ID/webhook" -H "$AUTH" > /dev/null 2>&1 \
        && echo "  + Webhook on deploy-staging" || echo "  ~ Webhook failed"

# --- Pipeline Schedules ---
echo ""
echo "Setting up pipeline schedules..."
curl -sf -X PUT "$URL/api/jobs/pipeline-schedule/ETL" -H "$AUTH" -H "$CT" \
    -d '{"schedule":{"type":"cron","value":"0 0 6 * * *"}}' > /dev/null 2>&1 \
    && echo "  + ETL pipeline: daily at 6am" || echo "  ~ ETL schedule failed"
curl -sf -X PUT "$URL/api/jobs/pipeline-schedule/Maintenance" -H "$AUTH" -H "$CT" \
    -d '{"schedule":{"type":"cron","value":"0 0 3 * * *"}}' > /dev/null 2>&1 \
    && echo "  + Maintenance pipeline: daily at 3am" || echo "  ~ Maintenance schedule failed"
curl -sf -X PUT "$URL/api/jobs/pipeline-schedule/Data-Sync" -H "$AUTH" -H "$CT" \
    -d '{"schedule":{"type":"interval","value":{"interval_secs":14400}}}' > /dev/null 2>&1 \
    && echo "  + Data-Sync pipeline: every 4 hours" || echo "  ~ Data-Sync schedule failed"

# --- Trigger pipelines to build execution history ---
echo ""
echo "Triggering pipelines for execution history..."

# Run 1: ETL pipeline
[ -n "$ETL_EXTRACT_ID" ] && curl -sf -X POST "$URL/api/jobs/$ETL_EXTRACT_ID/trigger" -H "$AUTH" > /dev/null 2>&1 && echo "  + ETL pipeline run 1" || true
sleep 8

# Run 1: Maintenance pipeline
[ -n "$DB_BACKUP_ID" ] && curl -sf -X POST "$URL/api/jobs/$DB_BACKUP_ID/trigger" -H "$AUTH" > /dev/null 2>&1 && echo "  + Maintenance pipeline run 1" || true
sleep 5

# Run 1: Data-Sync pipeline (parallel roots)
[ -n "$SYNC_USERS_ID" ] && curl -sf -X POST "$URL/api/jobs/$SYNC_USERS_ID/trigger" -H "$AUTH" > /dev/null 2>&1 || true
[ -n "$SYNC_INVENTORY_ID" ] && curl -sf -X POST "$URL/api/jobs/$SYNC_INVENTORY_ID/trigger" -H "$AUTH" > /dev/null 2>&1 && echo "  + Data-Sync pipeline run 1" || true
sleep 5

# Run 1: Deploy pipeline (will stop at production due to approval)
[ -n "$DEPLOY_STAGING_ID" ] && curl -sf -X POST "$URL/api/jobs/$DEPLOY_STAGING_ID/trigger" -H "$AUTH" > /dev/null 2>&1 && echo "  + Deploy pipeline run 1" || true
sleep 5

# Run 2: ETL pipeline (second run for history)
[ -n "$ETL_EXTRACT_ID" ] && curl -sf -X POST "$URL/api/jobs/$ETL_EXTRACT_ID/trigger" -H "$AUTH" > /dev/null 2>&1 && echo "  + ETL pipeline run 2" || true
sleep 8

# Run 2: Maintenance
[ -n "$DB_BACKUP_ID" ] && curl -sf -X POST "$URL/api/jobs/$DB_BACKUP_ID/trigger" -H "$AUTH" > /dev/null 2>&1 && echo "  + Maintenance pipeline run 2" || true
sleep 5

# Run 2: Data-Sync
[ -n "$SYNC_USERS_ID" ] && curl -sf -X POST "$URL/api/jobs/$SYNC_USERS_ID/trigger" -H "$AUTH" > /dev/null 2>&1 || true
[ -n "$SYNC_INVENTORY_ID" ] && curl -sf -X POST "$URL/api/jobs/$SYNC_INVENTORY_ID/trigger" -H "$AUTH" > /dev/null 2>&1 && echo "  + Data-Sync pipeline run 2" || true
sleep 5

# Monitoring jobs
for job_name in health-check disk-usage uptime-check api-latency-test; do
    JOB_ID=$(get_job_id "$job_name")
    [ -n "$JOB_ID" ] && curl -sf -X POST "$URL/api/jobs/$JOB_ID/trigger" -H "$AUTH" > /dev/null 2>&1 || true
done
echo "  + Monitoring jobs triggered"
sleep 3

# Run 3: ETL pipeline (third run)
[ -n "$ETL_EXTRACT_ID" ] && curl -sf -X POST "$URL/api/jobs/$ETL_EXTRACT_ID/trigger" -H "$AUTH" > /dev/null 2>&1 && echo "  + ETL pipeline run 3" || true

echo ""
echo "========================="
echo "Seed complete!"
echo ""
echo "What was created:"
echo "  - 6 groups: ETL, Monitoring, Deploys, Maintenance, Reports, Data-Sync"
echo "  - 24 jobs across all schedule types"
echo "  - 7 variables (LAST_ETL_COUNT, DEPLOY_VERSION, ENV, API_HOST, ALERT_EMAIL, ETL_WORKERS, REPORT_MONTH)"
echo "  - Pipelines:"
echo "      ETL: extract -> transform -> validate -> load (4-stage)"
echo "      Deploys: staging -> smoke-test -> production (3-stage with approval gate)"
echo "      Maintenance: backup -> log-rotate + cache-purge (fan-out)"
echo "      Data-Sync: users + inventory -> reconcile -> notify (parallel roots)"
echo "  - Pipeline schedules: ETL (daily 6am), Maintenance (daily 3am), Data-Sync (every 4h)"
echo "  - Calendar schedules: monthly-report, quarterly-audit, weekly-digest"
echo "  - Parameterized: etl-transform (workers), deploy-staging (branch, env), monthly-report (month)"
echo "  - Webhook enabled on deploy-staging"
echo "  - Multiple pipeline runs for history view"
echo ""
echo "Open the dashboard: $URL"
