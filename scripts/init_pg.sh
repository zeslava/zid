#!/bin/sh
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
. "$SCRIPT_DIR/.env"

JAIL="${1:-zid_pg}"

# Parse DATABASE_URL: postgres://user:pass@host:port/db
: "${DATABASE_URL:?DATABASE_URL is not set in .env}"
_url="${DATABASE_URL#postgres://}"
DB_USER="${_url%%:*}"
_url="${_url#*:}"
DB_PASS="${_url%%@*}"
_url="${_url#*@}"
_url="${_url#*/}"
DB_NAME="${_url%%\?*}"
: "${DB_USER:?failed to parse user from DATABASE_URL}"
: "${DB_PASS:?failed to parse password from DATABASE_URL}"
: "${DB_NAME:?failed to parse database name from DATABASE_URL}"

doas dail exec "$JAIL" psql -U postgres -c "CREATE USER $DB_USER WITH PASSWORD '$DB_PASS';" 2>/dev/null || echo "User $DB_USER already exists"
doas dail exec "$JAIL" psql -U postgres -c "CREATE DATABASE $DB_NAME OWNER $DB_USER;" 2>/dev/null || echo "Database $DB_NAME already exists"

echo "Done."
