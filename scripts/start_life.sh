#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$ROOT/bionic_v0/Cargo.toml"
RUNTIME_DIR="${1:-$ROOT/runtime_life}"
MAX_RESTARTS="${2:-5}"

mkdir -p "$RUNTIME_DIR"
ARCHIVE_DIR="$RUNTIME_DIR/snapshots"
mkdir -p "$ARCHIVE_DIR"

RESTART_COUNT=0

echo "=== Continuous Life v0 ==="
echo "runtime: $RUNTIME_DIR"
echo "max_restarts: $MAX_RESTARTS"
echo ""

while [ "$RESTART_COUNT" -lt "$MAX_RESTARTS" ]; do
    echo "[$(date -Iseconds)] Starting body (restart #$RESTART_COUNT)..."

    cargo run --release --manifest-path "$MANIFEST" --bin body -- "$RUNTIME_DIR" &
    BODY_PID=$!

    wait $BODY_PID 2>/dev/null || true
    EXIT_CODE=$?

    echo "[$(date -Iseconds)] Body exited with code $EXIT_CODE"

    if [ $EXIT_CODE -eq 0 ]; then
        echo "Body exited cleanly. Not restarting."
        exit 0
    fi

    RESTART_COUNT=$((RESTART_COUNT + 1))
    echo "Restarting in 3 seconds..."
    sleep 3
done

echo "Max restarts ($MAX_RESTARTS) reached. Life support terminated."
exit 1
