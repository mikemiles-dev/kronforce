#!/bin/sh
set -e

DB_PATH="${KRONFORCE_DB:-/data/kronforce.db}"

# Restore from replica if the database doesn't exist
if [ ! -f "$DB_PATH" ] && [ -n "$LITESTREAM_REPLICA_URL" ]; then
    echo "Restoring database from replica..."
    litestream restore -if-replica-exists -config /etc/litestream.yml "$DB_PATH" || true
fi

if [ -n "$LITESTREAM_REPLICA_URL" ]; then
    # Start Litestream as the supervisor — it runs Kronforce as a subprocess
    # and continuously replicates WAL changes to S3.
    # On SIGTERM, Litestream stops replication and forwards the signal to Kronforce.
    echo "Starting with Litestream replication to $LITESTREAM_REPLICA_URL"
    exec litestream replicate -config /etc/litestream.yml -exec "kronforce"
else
    # No replication configured — run Kronforce directly
    echo "Starting without replication (set LITESTREAM_REPLICA_URL to enable)"
    exec kronforce
fi
