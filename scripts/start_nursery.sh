#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$ROOT/bionic_v0/Cargo.toml"
RUNTIME_DIR="$ROOT/runtime_nursery"
mkdir -p "$RUNTIME_DIR"

echo "=== Live Human Nursery v0 ==="
echo "runtime: $RUNTIME_DIR"
echo ""

# Terminal 1: body (silicon-life process)
echo "[1/3] Starting body..."
cargo run --release --manifest-path "$MANIFEST" --bin body -- "$RUNTIME_DIR" &
BODY_PID=$!
echo "  body pid=$BODY_PID"

sleep 2

# Terminal 2: observer (numeric snapshot viewer)
echo "[2/3] Starting observer..."
cargo run --release --manifest-path "$MANIFEST" --bin observer -- "$RUNTIME_DIR" 20 &
OBSERVER_PID=$!
echo "  observer pid=$OBSERVER_PID"

sleep 1

# Terminal 3: surface (pixel expression viewer)
echo "[3/3] Starting surface..."
cargo run --release --manifest-path "$MANIFEST" --bin observer_surface -- "$RUNTIME_DIR" &
SURFACE_PID=$!
echo "  surface pid=$SURFACE_PID"

echo ""
echo "All processes running. Press Enter to stop..."
read -r

kill $BODY_PID $OBSERVER_PID $SURFACE_PID 2>/dev/null || true
wait $BODY_PID $OBSERVER_PID $SURFACE_PID 2>/dev/null || true
echo "Nursery stopped."
