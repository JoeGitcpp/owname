#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PLIST_NAME="com.owname.body"
PLIST_DIR="$HOME/Library/LaunchAgents"
PLIST_PATH="$PLIST_DIR/${PLIST_NAME}.plist"
TMP_PLIST="$(mktemp)"

cat > "$TMP_PLIST" << PLISTEOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN"
  "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>${PLIST_NAME}</string>
    <key>ProgramArguments</key>
    <array>
        <string>${ROOT}/scripts/start_life.sh</string>
        <string>${ROOT}/runtime_life</string>
        <string>50</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
    <key>StandardOutPath</key>
    <string>${ROOT}/runtime_life/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>${ROOT}/runtime_life/stderr.log</string>
</dict>
</plist>
PLISTEOF

plutil -lint "$TMP_PLIST"

mkdir -p "$PLIST_DIR"
cp "$TMP_PLIST" "$PLIST_PATH"
launchctl unload "$PLIST_PATH" 2>/dev/null || true
launchctl load "$PLIST_PATH" 2>/dev/null || true
rm -f "$TMP_PLIST"

echo "Installed $PLIST_PATH"
echo "Body will auto-start on boot and restart on crash."
echo ""
echo "Manual control:"
echo "  stop:  launchctl unload $PLIST_PATH"
echo "  start: launchctl load $PLIST_PATH"
echo "  status: launchctl list | grep $PLIST_NAME"
