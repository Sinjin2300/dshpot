#!/usr/bin/env bash
set -euo pipefail

DATA_DIR="${DATA_DIR:-/data}"
DB="${DATA_DIR}/honeypot.db"
HOST_KEY="${DATA_DIR}/honeypot_host_key.pem"

if [ ! -f "$DB" ]; then
    echo "No database found at $DB, running init..."
    dshpot init --db "$DB" --host-key "$HOST_KEY"
fi

exec dshpot serve \
    --db "$DB" \
    --host-key "$HOST_KEY" \
    "$@"
