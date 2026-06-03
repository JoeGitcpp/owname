#!/usr/bin/env bash
# server_setup.sh — one-time setup on server-joe.
# Run this AFTER:
#   1. aborn.live is in Cloudflare (NS changed at Alibaba Cloud, propagated)
#   2. CF Access is configured for aborn.live (Zero Trust → Applications)
#   3. You've run: bash scripts/deploy_aborn.sh   (syncs files to server)
#
# Then SSH in and run:   bash ~/aborn/scripts/server_setup.sh
# ─────────────────────────────────────────────────────────────────────────────

set -euo pipefail
DEPLOY=/home/joe/aborn

echo "=== 1/5  Install Rust (if not present) ==="
if ! command -v rustc &>/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  source "$HOME/.cargo/env"
fi
rustc --version

echo "=== 2/5  Compile body binary on server ==="
cd "$DEPLOY/bionic_v0_src"
source "$HOME/.cargo/env" 2>/dev/null || true
# Install system deps for nokhwa (camera, only used in hardware mode) and cpal (audio)
apt-get install -y libudev-dev libv4l-dev libclang-dev pkg-config libasound2-dev 2>/dev/null || true
cargo build --release --bin body
cp target/release/body "$DEPLOY/body"
chmod 755 "$DEPLOY/body"
echo "  body binary: $(ls -lh $DEPLOY/body | awk '{print $5}')"

echo "=== 3/5  Install gateway deps ==="
cd "$DEPLOY/gateway"
npm install --omit=dev

echo "=== 4/5  Add aborn.live to CF tunnel ==="
CF_CONFIG=/etc/cloudflared/config.yml
if grep -q "aborn.live" "$CF_CONFIG"; then
  echo "  aborn.live already in tunnel config"
else
  # Insert before the catch-all 404 line
  sed -i '/- service: http_status:404/i\  - hostname: aborn.live\n    service: http://127.0.0.1:3900' "$CF_CONFIG"
  echo "  Added aborn.live → localhost:3900"
fi
cat "$CF_CONFIG"

echo "=== 5/5  Create systemd service for aborn-gateway ==="
cat > /etc/systemd/system/aborn-gateway.service << 'UNIT'
[Unit]
Description=aborn gateway
After=network.target

[Service]
Type=simple
User=joe
WorkingDirectory=/home/joe/aborn/gateway
Environment=ABORN_PORT=3900
Environment=ABORN_BODY_BIN=/home/joe/aborn/body
Environment=ABORN_STATIC=/home/joe/aborn/site/dist
Environment=ABORN_SESSIONS=/tmp/aborn-sessions
Environment=ABORN_MAX=8
Environment=ABORN_PERIP=2
Environment=ABORN_RATE=5
Environment=ABORN_IDLE_MIN=5
Environment=ABORN_MAX_MIN=30
ExecStart=/usr/bin/node server.mjs
Restart=always
RestartSec=5
# No stdout logging of content — only errors to journal
StandardOutput=null
StandardError=journal

[Install]
WantedBy=multi-user.target
UNIT

systemctl daemon-reload
systemctl enable aborn-gateway
systemctl restart aborn-gateway
systemctl reload cloudflared 2>/dev/null || systemctl restart cloudflared

echo ""
echo "=== Setup complete ==="
echo "  gateway: systemctl status aborn-gateway"
echo "  logs:    journalctl -u aborn-gateway -f"
echo "  tunnel:  cloudflared tunnel info hermes-server"
echo ""
echo "  Test locally first:"
echo "    curl -s http://127.0.0.1:3900 | head -3"
echo ""
echo "  Once DNS propagates, visit https://aborn.live"
