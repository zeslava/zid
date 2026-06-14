#!/bin/sh
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
. "$SCRIPT_DIR/.env"

JAIL="${1:-zid_pg}"

# Parse DATABASE_URL: postgres://user:pass@host:port/db
_url="${DATABASE_URL#postgres://}"
DB_USER="${_url%%:*}"
_url="${_url#*:}"
DB_PASS="${_url%%@*}"
_url="${_url#*@}"
_url="${_url#*/}"
DB_NAME="${_url%%\?*}"

doas dail exec "$JAIL" psql -U postgres -c "CREATE USER $DB_USER WITH PASSWORD '$DB_PASS';" 2>/dev/null || echo "User $DB_USER already exists"
doas dail exec "$JAIL" psql -U postgres -c "CREATE DATABASE $DB_NAME OWNER $DB_USER;" 2>/dev/null || echo "Database $DB_NAME already exists"

echo "Done."
