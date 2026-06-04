#!/usr/bin/env bash
# deploy_aborn.sh — sync files to server-joe.
# Run from your Mac after making changes.
# Usage: bash scripts/deploy_aborn.sh
#
# This only syncs files. For first-time server setup, run server_setup.sh on
# the server after this.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REMOTE=server-joe
DEPLOY_DIR=/home/joe/aborn

echo "=== 1/3  Build site ==="
cd "$ROOT/site"
npm run build

echo "=== 2/3  Sync to server ==="
ssh $REMOTE "mkdir -p $DEPLOY_DIR/site/dist $DEPLOY_DIR/gateway $DEPLOY_DIR/scripts"

# Static site
rsync -az --delete "$ROOT/site/dist/"   "$REMOTE:$DEPLOY_DIR/site/dist/"
# Gateway (security + server)
rsync -az           "$ROOT/gateway/server.mjs" \
                    "$ROOT/gateway/security.mjs" \
                    "$ROOT/gateway/auth.mjs" \
                    "$ROOT/gateway/training_pool.mjs" \
                    "$ROOT/gateway/package.json" \
                    "$REMOTE:$DEPLOY_DIR/gateway/"
# Bionic source (for server-side compile)
rsync -az --delete \
  --exclude target --exclude .git \
  "$ROOT/bionic_v0/" "$REMOTE:$DEPLOY_DIR/bionic_v0_src/"
# Setup script
rsync -az "$ROOT/scripts/server_setup.sh" "$REMOTE:$DEPLOY_DIR/scripts/"

echo "=== 3/3  Restart gateway (if running) ==="
ssh $REMOTE "mkdir -p $DEPLOY_DIR/user-memories && systemctl is-active aborn-gateway 2>/dev/null && systemctl restart aborn-gateway && echo 'gateway restarted' || echo 'gateway not yet installed — run server_setup.sh on server'"

echo ""
echo "Files synced. If first deploy, SSH in and run:"
echo "  sudo bash ~/aborn/scripts/server_setup.sh"
