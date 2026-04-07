#!/bin/bash
#
# Seed Kronforce with test data
#
# Usage:
#   ./examples/seed.sh                              # uses localhost:8080, prompts for key
#   ./examples/seed.sh kf_your_admin_key             # provide key as argument
#   KRONFORCE_URL=http://host:8080 ./examples/seed.sh kf_key
#
# Creates: 4 groups, 12 jobs (various types/schedules), 5 global variables
# Requires: curl, jq (optional, for pretty output)

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

# Create groups (hardcoded to avoid shell parsing issues)
echo "Creating groups..."
for group in ETL Monitoring Deploys Maintenance; do
    curl -sf -X POST "$URL/api/jobs/groups" -H "$AUTH" -H "$CT" -d "{\"name\": \"$group\"}" > /dev/null 2>&1 && echo "  ✓ Group: $group" || echo "  ⚠ Group: $group (may already exist)"
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
    curl -sf -X POST "$URL/api/variables" -H "$AUTH" -H "$CT" -d "$var" > /dev/null 2>&1 && echo "  ✓ Variable: $name" || echo "  ⚠ Variable: $name (may already exist)"
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
CREATED=0
FAILED=0
echo "$JOBS" | while read -r job; do
    name=$(echo "$job" | python3 -c "import json,sys; print(json.load(sys.stdin)['name'])" 2>/dev/null || echo "$job" | jq -r '.name')
    resp=$(curl -sf -X POST "$URL/api/jobs" -H "$AUTH" -H "$CT" -d "$job" 2>&1)
    if [ $? -eq 0 ]; then
        echo "  ✓ Job: $name"
    else
        echo "  ⚠ Job: $name (may already exist or invalid)"
    fi
done

# Set up dependencies between jobs
echo ""
echo "Setting up dependencies..."

# Helper to get job ID by name
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
ETL_LOAD_ID=$(get_job_id "etl-load")
DEPLOY_STAGING_ID=$(get_job_id "deploy-staging")
DEPLOY_PROD_ID=$(get_job_id "deploy-production")
DB_BACKUP_ID=$(get_job_id "db-backup")
LOG_ROTATE_ID=$(get_job_id "log-rotate")

# ETL pipeline: extract → transform → load
if [ -n "$ETL_EXTRACT_ID" ] && [ -n "$ETL_TRANSFORM_ID" ]; then
    curl -sf -X PUT "$URL/api/jobs/$ETL_TRANSFORM_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$ETL_EXTRACT_ID\", \"within_secs\": 3600}]}" > /dev/null 2>&1 \
        && echo "  ✓ etl-transform depends on etl-extract" || echo "  ⚠ etl-transform dependency failed"
fi
if [ -n "$ETL_TRANSFORM_ID" ] && [ -n "$ETL_LOAD_ID" ]; then
    curl -sf -X PUT "$URL/api/jobs/$ETL_LOAD_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$ETL_TRANSFORM_ID\", \"within_secs\": 3600}]}" > /dev/null 2>&1 \
        && echo "  ✓ etl-load depends on etl-transform" || echo "  ⚠ etl-load dependency failed"
fi

# Deploy pipeline: staging → production
if [ -n "$DEPLOY_STAGING_ID" ] && [ -n "$DEPLOY_PROD_ID" ]; then
    curl -sf -X PUT "$URL/api/jobs/$DEPLOY_PROD_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$DEPLOY_STAGING_ID\", \"within_secs\": 7200}]}" > /dev/null 2>&1 \
        && echo "  ✓ deploy-production depends on deploy-staging" || echo "  ⚠ deploy-production dependency failed"
fi

# Maintenance: log-rotate after db-backup
if [ -n "$DB_BACKUP_ID" ] && [ -n "$LOG_ROTATE_ID" ]; then
    curl -sf -X PUT "$URL/api/jobs/$LOG_ROTATE_ID" -H "$AUTH" -H "$CT" \
        -d "{\"depends_on\": [{\"job_id\": \"$DB_BACKUP_ID\", \"within_secs\": 7200}]}" > /dev/null 2>&1 \
        && echo "  ✓ log-rotate depends on db-backup" || echo "  ⚠ log-rotate dependency failed"
fi

echo ""
echo "========================="
echo "Seed complete!"
echo ""
echo "What was created:"
echo "  • 4 groups: ETL, Monitoring, Deploys, Maintenance"
echo "  • 14 jobs with dependencies, event triggers, and output rules"
echo "  • 5 global variables (LAST_ETL_COUNT, DEPLOY_VERSION, ENV, API_HOST, ALERT_EMAIL)"
echo "  • Dependencies: extract→transform→load, staging→production, backup→log-rotate"
echo "  • Event triggers: etl-failure-alert, deploy-notify"
echo ""
echo "Try triggering a job:"
echo "  curl -X POST $URL/api/jobs/<id>/trigger -H '$AUTH'"
echo ""
echo "Or open the dashboard: $URL"
