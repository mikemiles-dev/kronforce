#!/bin/sh
#
# Quick reseed: wipes the DB, restarts the demo, extracts the admin key
# from container logs, and runs the seed script.
#
# Usage (on the droplet):
#   cd ~/kronforce-platform/deploy && sh ~/kronforce/demo/reseed.sh
#
set -e

COMPOSE_DIR="$(pwd)"
SEED_SCRIPT="$(dirname "$0")/seed-demo.sh"

if [ ! -f "$SEED_SCRIPT" ]; then
    echo "Error: seed-demo.sh not found at $SEED_SCRIPT"
    echo "Run this from the deploy directory: cd ~/kronforce-platform/deploy && sh ~/kronforce/demo/reseed.sh"
    exit 1
fi

echo "=== Kronforce Demo Reseed ==="

# Wipe the DB
echo "Wiping database..."
docker compose exec demo rm -f /data/kronforce.db 2>/dev/null || true

# Restart to trigger fresh bootstrap
echo "Restarting demo container..."
docker compose restart demo
echo "Waiting for startup..."
sleep 10

# Wait for health
for i in $(seq 1 30); do
    if curl -sf http://localhost:8080/api/health > /dev/null 2>&1; then
        echo "Demo is healthy."
        break
    fi
    sleep 1
done

# Extract admin key from logs
echo "Extracting admin key from logs..."
KEY=$(docker compose logs demo 2>&1 | grep "kf_" | tail -1 | grep -oE 'kf_[A-Za-z0-9+/=]+' | tail -1)

if [ -z "$KEY" ]; then
    echo "Could not find admin key in logs. Trying .env..."
    KEY=$(cat .env 2>/dev/null | grep DEMO_ADMIN_KEY | cut -d= -f2-)
fi

if [ -z "$KEY" ]; then
    echo "Error: No admin key found. Check docker compose logs demo"
    exit 1
fi

echo "Using key: $(echo "$KEY" | cut -c1-15)..."

# Verify the key works
STATUS=$(curl -sf -o /dev/null -w '%{http_code}' "http://localhost:8080/api/jobs?per_page=1" -H "Authorization: Bearer $KEY" 2>/dev/null || echo "000")
if [ "$STATUS" != "200" ]; then
    echo "Error: Key returned HTTP $STATUS. Check logs:"
    echo "  docker compose logs demo --tail 20"
    exit 1
fi
echo "Key verified (HTTP 200)."

# Run seed
echo ""
echo "Running seed script..."
KRONFORCE_URL=http://localhost:8080 ADMIN_KEY="$KEY" sh "$SEED_SCRIPT"
