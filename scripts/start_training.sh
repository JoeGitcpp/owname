#!/usr/bin/env bash
# start_training.sh — 启动完整训练管线
#   sandbox_runner + body + nursery_mentor (TTS 训练)
#
# Usage:
#   ./scripts/start_training.sh              # 需要 API key
#   ./scripts/start_training.sh --corpus     # 本地语料，不需要 API
#   ./scripts/start_training.sh --dry-run    # 内置短语，不需要 API
#   ./scripts/start_training.sh --stop       # 停止所有组件

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/bionic_v0/target/release"
RUNTIME="$ROOT/runtime"
LOGS="$ROOT/logs"
mkdir -p "$LOGS" "$RUNTIME/live_bytes"

MODE="${1:-}"

# ── Stop ──
if [[ "$MODE" == "--stop" ]]; then
    echo "Stopping all training processes..."
    killall -9 body sandbox_runner 2>/dev/null || true
    pkill -f nursery_mentor.py 2>/dev/null || true
    echo "Stopped."
    exit 0
fi

# ── Start ──
echo "========================================="
echo "  Nursery Training Pipeline"
echo "========================================="
echo ""

# Clean corrupt snapshot (keep learned_bias)
if [[ -f "$RUNTIME/snapshot.bin" ]]; then
    echo "[0/3] Removing stale snapshot.bin (keeping learned_bias.bin)..."
    rm -f "$RUNTIME/snapshot.bin" "$RUNTIME/snapshot.bin.tmp"
fi

# 1. Sandbox Runner (generates seed frames for body's bytecode loop)
echo "[1/3] Starting sandbox_runner..."
nohup "$BIN/sandbox_runner" "$RUNTIME" > "$LOGS/sandbox.log" 2>&1 &
SANDBOX_PID=$!
echo "  sandbox_runner pid=$SANDBOX_PID"
sleep 0.5

# 2. Body (the silicon life process, nursery mode for voice training)
echo "[2/3] Starting body (nursery-bias mode)..."
nohup "$BIN/body" "$RUNTIME" --nursery-bias > "$LOGS/body.log" 2>&1 &
BODY_PID=$!
echo "  body pid=$BODY_PID"
sleep 1

# 3. Nursery Mentor (TTS training)
echo "[3/3] Starting nursery_mentor..."
MENTOR_ARGS=""
if [[ "$MODE" == "--dry-run" ]]; then
    MENTOR_ARGS="--dry-run"
elif [[ "$MODE" == "--corpus" ]]; then
    MENTOR_ARGS="--corpus"
fi
nohup python3 "$ROOT/scripts/nursery_mentor.py" $MENTOR_ARGS > "$LOGS/mentor.log" 2>&1 &
MENTOR_PID=$!
echo "  nursery_mentor pid=$MENTOR_PID"
echo "  mode: ${MODE:-api}"

echo ""
echo "All components running."
echo "  sandbox: logs/sandbox.log"
echo "  body:    logs/body.log"
echo "  mentor:  logs/mentor.log"
echo ""
echo "Stop: ./scripts/start_training.sh --stop"
echo ""

# Save PIDs for later stop
echo "$SANDBOX_PID $BODY_PID $MENTOR_PID" > "$RUNTIME/training_pids.txt"

wait
