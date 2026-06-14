#!/bin/sh
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
. "$SCRIPT_DIR/.env"

JAIL="${1:-zid_pg}"

doas dail exec "$JAIL" psql -U postgres -c "CREATE USER $POSTGRES_USER WITH PASSWORD '$POSTGRES_PASSWORD';" 2>/dev/null || echo "User $POSTGRES_USER already exists"
doas dail exec "$JAIL" psql -U postgres -c "CREATE DATABASE $POSTGRES_DB OWNER $POSTGRES_USER;" 2>/dev/null || echo "Database $POSTGRES_DB already exists"

echo "Done."
