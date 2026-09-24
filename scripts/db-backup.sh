#!/usr/bin/env bash
# Online SQLite backup for tester / public deployments.
#
# Usage: ./scripts/db-backup.sh [db_path] [backup_dir]
#   db_path    default: $DATABASE_URL sqlite path, else knowledgeable.db
#   backup_dir default: ./backups
# Env: KEEP (default 7) — how many newest backups to retain.
#
# Uses `sqlite3 .backup`, which is safe against a live server (WAL-aware,
# no file copy races). Verifies the copy with `integrity_check` and fails
# loudly otherwise. Never prints secrets; takes no credentials.
set -euo pipefail

resolve_db() {
    if [ "${1:-}" != "" ]; then
        printf '%s' "$1"
        return
    fi
    local url="${DATABASE_URL:-}"
    if [ "$url" = "" ] && [ -f .env ]; then
        url="$(grep -E '^DATABASE_URL=' .env | tail -1 | cut -d= -f2-)"
    fi
    case "$url" in
        sqlite:*) printf '%s' "${url#sqlite:}" ;;
        *) printf 'knowledgeable.db' ;;
    esac
}

DB="$(resolve_db "${1:-}")"
DIR="${2:-./backups}"
KEEP="${KEEP:-7}"

[ -f "$DB" ] || { echo "db-backup: database not found: $DB" >&2; exit 1; }
command -v sqlite3 >/dev/null || { echo "db-backup: missing sqlite3" >&2; exit 1; }
mkdir -p "$DIR"

TS="$(date +%Y%m%d-%H%M%S)"
DEST="$DIR/knowledgeable-$TS.db"
# Same-second reruns must not silently overwrite: wait for a fresh stamp.
while [ -e "$DEST" ]; do
    sleep 1
    TS="$(date +%Y%m%d-%H%M%S)"
    DEST="$DIR/knowledgeable-$TS.db"
done

sqlite3 "$DB" ".backup '$DEST'"
CHECK="$(sqlite3 "$DEST" 'PRAGMA integrity_check;')"
[ "$CHECK" = "ok" ] || { echo "db-backup: integrity_check failed on $DEST" >&2; exit 1; }

# Prune to the newest $KEEP backups.
ls -1t "$DIR"/knowledgeable-*.db 2>/dev/null | tail -n "+$((KEEP + 1))" | xargs -r rm -f

echo "backup ok: $DEST"
