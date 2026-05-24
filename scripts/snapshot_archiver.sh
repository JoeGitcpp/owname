#!/usr/bin/env bash
set -euo pipefail

RUNTIME_DIR="${1:-runtime_nursery}"
ARCHIVE_DIR="$RUNTIME_DIR/snapshots"
SNAPSHOT_FILE="$RUNTIME_DIR/snapshot.bin"
INTERVAL_SEC="${2:-3600}"

mkdir -p "$ARCHIVE_DIR"

echo "snapshot_archiver start dir=$RUNTIME_DIR interval=${INTERVAL_SEC}s"

while true; do
    if [ -f "$SNAPSHOT_FILE" ]; then
        TS=$(date +%Y-%m-%d/%H%M)
        DEST="$ARCHIVE_DIR/$TS.bin"
        mkdir -p "$(dirname "$DEST")"
        cp "$SNAPSHOT_FILE" "$DEST"
        echo "$(date -Iseconds) archived $(wc -c < "$SNAPSHOT_FILE") bytes -> $TS.bin"
    fi
    sleep "$INTERVAL_SEC"
done
