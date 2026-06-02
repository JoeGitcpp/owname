#!/usr/bin/env bash
# 静默训练 30 分钟 —— 眼(摄像头)+ 耳(环境声/麦克风)+ 嘴(自发咿呀)+ surface
# 不加语音教学(无 nursery_mentor TTS)。让它自己安静地活、自己长。
#
# 必须在【你授权过摄像头的终端】里直接运行,眼睛才能真睁开。
# 用法:  bash scripts/run_silent_30min.sh
# 停止:  bash scripts/run_silent_30min.sh --stop

set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/bionic_v0/target/release"
RT="$ROOT/runtime_silent30"
TRACE="$RT/life_trace.csv"

if [[ "${1:-}" == "--stop" ]]; then
  pkill -f "release/body $RT" 2>/dev/null
  pkill -f sandbox_runner 2>/dev/null
  pkill -f sample_silent30 2>/dev/null
  echo "stopped."
  exit 0
fi

rm -rf "$RT"; mkdir -p "$RT"
echo "=========================================="
echo " 静默训练 30 分钟 (眼+耳+嘴+surface,无教学)"
echo " 运行目录: $RT"
echo "=========================================="

# 1) sandbox_runner —— 字节闭环种子(完整管线该有)
if [[ -x "$BIN/sandbox_runner" ]]; then
  nohup "$BIN/sandbox_runner" "$RT" > "$RT/sandbox.log" 2>&1 &
  echo "  sandbox_runner pid=$!"
  sleep 0.5
fi

# 2) body —— 本体(在授权终端里跑 → 眼睛真睁开)。NO nursery: 静默。
nohup "$BIN/body" "$RT" 1800 > "$RT/body.log" 2>&1 &
BODY_PID=$!
echo "  body pid=$BODY_PID  (跑 1800s = 30min, 静默)"
sleep 1

# 3) 采样器 —— 每 60s 解码生命信号(看自我慢慢长、责任人慢慢突出)
echo "t_min,reserve,supply,drain,anxiety,pain,vital_state,self_coh,agency,n_obj,top_fam,top_recog,phot_health,hearing_fault,vocal_breath" > "$TRACE"
START=$(date +%s)
(
  for i in $(seq 1 31); do
    while [ $(( $(date +%s) - START )) -lt $(( i * 60 )) ]; do
      kill -0 "$BODY_PID" 2>/dev/null || exit 0
      sleep 2
    done
    kill -0 "$BODY_PID" 2>/dev/null || exit 0
    tmin=$(( ($(date +%s) - START) / 60 ))
    python3 "$ROOT/scripts/decode_full30.py" "$RT" "$tmin" >> "$TRACE"
  done
  echo "DONE" >> "$TRACE"
) > "$RT/sample_silent30.log" 2>&1 &
echo "  sampler pid=$!  → 轨迹: $TRACE"

echo ""
echo "30 分钟后看轨迹:  column -s, -t < $TRACE"
echo "看 surface:       $BIN/observer_surface $RT/snapshot.bin"
echo "停止:  bash scripts/run_silent_30min.sh --stop"
