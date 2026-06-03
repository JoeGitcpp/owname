#!/usr/bin/env bash
# deploy_aborn.sh — deploy body binary + gateway + site to server-joe
# Usage:  bash scripts/deploy_aborn.sh
# Requires: ssh server-joe works; Rust installed locally; Node on server.

set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
REMOTE=server-joe
DEPLOY_DIR=/home/joe/aborn

echo "=== 1/5  Build Rust release (local, Linux x86_64 cross-compile) ==="
cd "$ROOT/bionic_v0"
# If cross-compiler available use it; otherwise build natively on server instead.
if rustup target list --installed | grep -q x86_64-unknown-linux-musl; then
  cargo build --release --bin body --target x86_64-unknown-linux-musl
  BODY_BIN="target/x86_64-unknown-linux-musl/release/body"
else
  echo "  NOTE: cross-compiler not found. Will compile on server instead."
  BODY_BIN=""
fi

echo "=== 2/5  Build site ==="
cd "$ROOT/site"
npm run build

echo "=== 3/5  Sync files to server ==="
ssh $REMOTE "mkdir -p $DEPLOY_DIR/site/dist $DEPLOY_DIR/gateway"

# Site static files
rsync -az --delete "$ROOT/site/dist/" "$REMOTE:$DEPLOY_DIR/site/dist/"

# Gateway
rsync -az "$ROOT/gateway/server.mjs" "$ROOT/gateway/package.json" "$REMOTE:$DEPLOY_DIR/gateway/"

# Body binary (or source if no cross-compile)
if [ -n "$BODY_BIN" ]; then
  rsync -az "$ROOT/bionic_v0/$BODY_BIN" "$REMOTE:$DEPLOY_DIR/body"
  ssh $REMOTE "chmod +x $DEPLOY_DIR/body"
else
  echo "  Syncing source for server-side build..."
  rsync -az --delete \
    --exclude target --exclude .git \
    "$ROOT/bionic_v0/" "$REMOTE:$DEPLOY_DIR/bionic_v0_src/"
fi

echo "=== 4/5  Install gateway deps on server ==="
ssh $REMOTE "cd $DEPLOY_DIR/gateway && npm install --omit=dev 2>/dev/null"

echo "=== 5/5  Deploy instructions ==="
cat <<'INSTRUCTIONS'

On the server, run:
  # (first time only, if body binary was not cross-compiled)
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source ~/.cargo/env
  cd ~/aborn/bionic_v0_src && cargo build --release --bin body
  cp target/release/body ~/aborn/body

  # Start gateway
  cd ~/aborn/gateway
  ABORN_BODY_BIN=~/aborn/body \
  ABORN_STATIC=~/aborn/site/dist \
  ABORN_PORT=3900 \
  node server.mjs &

  # Add to CF tunnel (edit /etc/cloudflared/config.yml, add before the 404 line):
  #   - hostname: aborn.live
  #     service: http://localhost:3900
  # Then: sudo systemctl reload cloudflared

INSTRUCTIONS
echo "Done. Check the server instructions above."
