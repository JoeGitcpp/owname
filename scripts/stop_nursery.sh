#!/usr/bin/env bash
set -euo pipefail

echo "Stopping nursery processes..."
pkill -f "bionic_v0.*bin/body" 2>/dev/null || true
pkill -f "bionic_v0.*bin/observer" 2>/dev/null || true
pkill -f "bionic_v0.*bin/observer_surface" 2>/dev/null || true
sleep 1
echo "Done. Remaining bionic_v0 processes:"
pgrep -fl "bionic_v0" || echo "  (none)"
