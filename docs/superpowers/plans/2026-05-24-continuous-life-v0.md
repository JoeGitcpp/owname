# Continuous Life v0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 24/7 continuous body runtime with auto snapshot archiving, crash recovery, and macOS launchd auto-start.

**Architecture:** All changes in `scripts/` — shell scripts only. Zero Rust code changes. Snapshot archiver runs as a cron-like observer-side loop. Crash recovery wraps the body binary. launchd provides OS-level auto-start.

**Tech Stack:** bash, macOS launchd, existing bionic_v0 binaries.

---

## Task 1: Snapshot Archiver

**Files:** Create `scripts/snapshot_archiver.sh`

- [ ] **Step 1: Create script**

```bash
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
```

- [ ] **Step 2: Test — start body, archiver, verify**

```bash
chmod +x scripts/snapshot_archiver.sh

# Start body in background
RUNTIME_DIR="$TMPDIR/runtime_continuous_test"
mkdir -p "$RUNTIME_DIR"
cargo run --release --manifest-path bionic_v0/Cargo.toml --bin body -- "$RUNTIME_DIR" &
BODY_PID=$!
sleep 3

# Start archiver with 5s interval for testing
./scripts/snapshot_archiver.sh "$RUNTIME_DIR" 5 &
ARCHIVER_PID=$!
sleep 12

# Verify archives exist
ls -la "$RUNTIME_DIR/snapshots/"*/ ||
    echo "No archives yet — check if body is writing snapshots"

kill $ARCHIVER_PID $BODY_PID 2>/dev/null || true
```

Expected: Archive files appear under `$RUNTIME_DIR/snapshots/YYYY-MM-DD/HHMM.bin`.

- [ ] **Step 3: Commit**

```bash
git add scripts/snapshot_archiver.sh
git commit -m "feat: add snapshot archiver for continuous runtime history"
```

---

## Task 2: Crash Recovery Wrapper

**Files:** Create `scripts/start_life.sh`

- [ ] **Step 1: Create script**

```bash
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

    # Wait for body to exit
    wait $BODY_PID 2>/dev/null || true
    EXIT_CODE=$?

    echo "[$(date -Iseconds)] Body exited with code $EXIT_CODE"

    if [ $EXIT_CODE -eq 0 ]; then
        echo "Body exited cleanly (stopped by signal). Not restarting."
        exit 0
    fi

    RESTART_COUNT=$((RESTART_COUNT + 1))
    echo "Restarting in 3 seconds..."
    sleep 3
done

echo "Max restarts ($MAX_RESTARTS) reached. Life support terminated."
exit 1
```

- [ ] **Step 2: Test — kill body, verify restart**

```bash
chmod +x scripts/start_life.sh

# Start in background
RUNTIME_DIR="$TMPDIR/runtime_life_test"
./scripts/start_life.sh "$RUNTIME_DIR" 5 &
LIFE_PID=$!
sleep 4

# Kill body (not the wrapper)
BODY_PID=$(pgrep -f "bin/body" | head -1)
kill -9 $BODY_PID 2>/dev/null || true
sleep 6

# Check if body restarted (new PID)
NEW_BODY_PID=$(pgrep -f "bin/body" | head -1)
echo "Old PID: $BODY_PID, New PID: $NEW_BODY_PID"
[ "$NEW_BODY_PID" != "$BODY_PID" ] && echo "TEST PASS: body restarted" || echo "TEST FAIL: no restart"

kill $LIFE_PID 2>/dev/null || true
pkill -f "bin/body" 2>/dev/null || true
```

Expected: body restarted with a new PID after kill.

- [ ] **Step 3: Commit**

```bash
git add scripts/start_life.sh
git commit -m "feat: add crash recovery wrapper for continuous life runtime"
```

---

## Task 3: macOS Launchd Auto-Start

**Files:** Create `scripts/install_launchd.sh`

- [ ] **Step 1: Create install script**

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PLIST_NAME="com.owname.body"
PLIST_PATH="$HOME/Library/LaunchAgents/${PLIST_NAME}.plist"

cat > /tmp/${PLIST_NAME}.plist << PLISTEOF
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

mkdir -p "$HOME/Library/LaunchAgents"
cp /tmp/${PLIST_NAME}.plist "$PLIST_PATH"
launchctl unload "$PLIST_PATH" 2>/dev/null || true
launchctl load "$PLIST_PATH"

echo "Installed $PLIST_PATH"
echo "Body will auto-start on boot and restart on crash."
echo ""
echo "Manual control:"
echo "  stop:  launchctl unload $PLIST_PATH"
echo "  start: launchctl load $PLIST_PATH"
echo "  status: launchctl list | grep $PLIST_NAME"
```

- [ ] **Step 2: Verify plist syntax**

```bash
chmod +x scripts/install_launchd.sh
plutil -lint /tmp/com.owname.body.plist
```

Expected: `OK`.

- [ ] **Step 3: Commit**

```bash
git add scripts/install_launchd.sh
git commit -m "feat: add macOS launchd auto-start installer"
```

---

## Task 4: 24h Verification + Principle Gates

- [ ] **Step 1: Run all existing tests**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml 2>&1 | grep "test result:"
```

- [ ] **Step 2: G1 — Zero Rust changes**

```bash
git diff HEAD~3..HEAD --stat | grep "bionic_v0/src/" && echo "G1 WARNING" || echo "G1 PASS"
```

- [ ] **Step 3: G2 — Scripts are observer-side**

```bash
for script in scripts/*.sh; do
    grep -i "println\|body.write\|view.*write" "$script" && echo "G2 FAIL in $script" || true
done
echo "G2 PASS"
```

- [ ] **Step 4: Commit verification**

```bash
git add -A && git commit -m "chore: verify continuous life infrastructure"
```
