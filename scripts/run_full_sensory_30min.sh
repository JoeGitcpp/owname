#!/usr/bin/env bash
# 全感官 30 分钟跑 —— 耳(麦克风/BlackHole)+ 眼(摄像头)+ 嘴(发声)+ surface
#
# 必须在【你授权过摄像头的终端】里直接运行(Terminal/iTerm),
# 这样 body 子进程才继承摄像头权限,眼睛才能真睁开。
#
# 用法:  bash scripts/run_full_sensory_30min.sh
# 停止:  bash scripts/run_full_sensory_30min.sh --stop

set -uo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
BIN="$ROOT/bionic_v0/target/release"
RT="$ROOT/runtime_full30"
TRACE="$RT/life_trace.csv"

if [[ "${1:-}" == "--stop" ]]; then
  pkill -f "release/body $RT" 2>/dev/null
  pkill -f nursery_mentor.py 2>/dev/null
  pkill -f sample_full30 2>/dev/null
  echo "stopped."
  exit 0
fi

rm -rf "$RT"; mkdir -p "$RT"
echo "=========================================="
echo " 全感官 30 分钟跑 (耳+眼+嘴+surface)"
echo " 运行目录: $RT"
echo "=========================================="

# 1) sandbox_runner —— 给字节闭环喂种子(可选,但完整管线该有)
if [[ -x "$BIN/sandbox_runner" ]]; then
  nohup "$BIN/sandbox_runner" "$RT" > "$RT/sandbox.log" 2>&1 &
  echo "  sandbox_runner pid=$!"
  sleep 0.5
fi

# 2) body —— 本体(摄像头由本进程权限决定;在授权终端里跑→眼睛真睁开)
nohup "$BIN/body" "$RT" 1800 > "$RT/body.log" 2>&1 &
BODY_PID=$!
echo "  body pid=$BODY_PID  (跑 1800s = 30min)"
sleep 1

# 3) nursery_mentor —— 用 TTS 喂结构化声音教学(经 BlackHole 回环进麦克风=耳朵)
if [[ -f "$ROOT/scripts/nursery_mentor.py" ]]; then
  nohup python3 "$ROOT/scripts/nursery_mentor.py" --corpus > "$RT/mentor.log" 2>&1 &
  echo "  nursery_mentor pid=$!  (TTS 教学喂声音)"
fi

# 4) 采样器 —— 每 60s 解码生命信号到 CSV(嘴/眼/耳/维生/自我/责任人/纠结)
echo "t_min,reserve,anxiety,pain,vital_state,self_coh,agency,n_obj,top_fam,top_recog,phot_health,hearing_fault,vocal_breath" > "$TRACE"
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
) > "$RT/sample_full30.log" 2>&1 &
echo "  sampler pid=$!  → 轨迹写到 $TRACE"

echo ""
echo "30 分钟后看轨迹:  column -s, -t < $TRACE"
echo "看 surface(外化窗口):  $BIN/observer_surface $RT/snapshot.bin"
echo "停止:  bash scripts/run_full_sensory_30min.sh --stop"
