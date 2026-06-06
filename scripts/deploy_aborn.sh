#!/usr/bin/env bash
# deploy_aborn.sh — deploy to server-joe: build site, sync, rebuild body, restart.
# Run from your Mac after making changes.
# Usage: bash scripts/deploy_aborn.sh
#
# Syncs site + gateway + bionic_v0 source, then rebuilds the Rust `body` binary
# ON the server (incremental --release) and restarts the gateway. For first-time
# server setup, run server_setup.sh on the server.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REMOTE=server-joe
DEPLOY_DIR=/home/joe/aborn

echo "=== 1/4  Build site ==="
cd "$ROOT/site"
npm run build

echo "=== 2/4  Sync to server ==="
ssh $REMOTE "mkdir -p $DEPLOY_DIR/site/dist $DEPLOY_DIR/gateway $DEPLOY_DIR/scripts"

# Static site
rsync -az --delete "$ROOT/site/dist/"   "$REMOTE:$DEPLOY_DIR/site/dist/"
# Gateway — sync all .mjs modules (exclude test files) + package manifests
rsync -az \
  --exclude 'node_modules/' \
  --exclude '*.test.mjs' \
  "$ROOT/gateway/" "$REMOTE:$DEPLOY_DIR/gateway/"
# Bionic source (for server-side compile)
rsync -az --delete \
  --exclude target --exclude .git \
  "$ROOT/bionic_v0/" "$REMOTE:$DEPLOY_DIR/bionic_v0_src/"
# Setup script
rsync -az "$ROOT/scripts/server_setup.sh" "$REMOTE:$DEPLOY_DIR/scripts/"

echo "=== 3/4  Rebuild body on server (cargo --release, incremental) ==="
# deploy syncs SOURCE only; the Rust `body` must be recompiled on the server or
# the gateway keeps spawning the OLD binary. cargo lives in ~/.cargo/bin, which a
# non-interactive SSH shell does NOT have on PATH — source ~/.cargo/env first
# (otherwise: `cargo: command not found`, exit 127).
ssh $REMOTE "set -e; . \$HOME/.cargo/env; cd $DEPLOY_DIR/bionic_v0_src && cargo build --release --bin body && cp -f target/release/body $DEPLOY_DIR/body && echo 'body rebuilt'"

echo "=== 4/4  Restart gateway ==="
# Restarting a system service needs root; a plain `systemctl restart` over SSH is
# denied by polkit ("interactive authentication required"). `sudo -n` works
# (passwordless sudo is configured). Separate the is-active check from the
# restart so a restart failure is not misreported as "not installed".
ssh $REMOTE "mkdir -p $DEPLOY_DIR/user-memories; if systemctl is-active --quiet aborn-gateway; then sudo -n systemctl restart aborn-gateway && echo 'gateway restarted (active)' || echo 'RESTART FAILED — run on server: sudo systemctl restart aborn-gateway'; else echo 'aborn-gateway not installed — run: sudo bash $DEPLOY_DIR/scripts/server_setup.sh'; fi"

echo ""
echo "Deployed: site + gateway synced, body rebuilt, gateway restarted."
echo "First-time server setup only: sudo bash ~/aborn/scripts/server_setup.sh"
