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

# Create groups
echo "Creating groups..."
GROUPS=$(python3 -c "import json; d=json.load(open('$SEED_FILE')); [print(g) for g in d['groups']]" 2>/dev/null || jq -r '.groups[]' "$SEED_FILE")
for group in $GROUPS; do
    resp=$(curl -sf -X POST "$URL/api/jobs/groups" -H "$AUTH" -H "$CT" -d "{\"name\": \"$group\"}" 2>&1) && echo "  ✓ Group: $group" || echo "  ⚠ Group: $group (may already exist)"
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

echo ""
echo "========================="
echo "Seed complete!"
echo ""
echo "What was created:"
echo "  • 4 groups: ETL, Monitoring, Deploys, Maintenance"
echo "  • 12 jobs across all groups with various schedules"
echo "  • 5 global variables (LAST_ETL_COUNT, DEPLOY_VERSION, ENV, API_HOST, ALERT_EMAIL)"
echo ""
echo "Try triggering a job:"
echo "  curl -X POST $URL/api/jobs/<id>/trigger -H '$AUTH'"
echo ""
echo "Or open the dashboard: $URL"
