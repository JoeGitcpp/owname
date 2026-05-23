# Live Life v0 Layer 1 — Live Human Nursery 闭环实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 补完真实硬件验证，走通 Live Human Nursery Phase 0-3，拿到 communication_candidate = 1。

**Architecture:** 已有代码完整（RealCameraFrameProvider + VibAdapter + vocal output 均已实现）。本 plan 聚焦：编译验证、macOS 权限、启动脚本、live session 执行与证据记录。不新增 body 本体模块。

**Tech Stack:** Rust, nokhwa (AVFoundation), cpal (CoreAudio), existing bionic_v0 body/observer/surface binaries

---

## 文件变更范围

| 文件 | 变更 |
|------|------|
| `scripts/start_nursery.sh` | 新增一键启动脚本 |
| `scripts/stop_nursery.sh` | 新增停止脚本 |
| `docs/superpowers/notes/2026-05-24-nursery-m0-evidence.md` | M0 证据记录 |
| `docs/superpowers/notes/2026-05-24-nursery-m1-evidence.md` | M1 证据记录 |

不修改任何 `bionic_v0/src/` 下的文件。不改 `AGENTS.md`、`consciousness-stream-architecture.md`、`tier0-base-elements.md`。

---

## Task 1: M0 — 编译验证与硬件冒烟

**Files:**
- Create: `scripts/start_nursery.sh`
- Create: `scripts/stop_nursery.sh`
- Create: `docs/superpowers/notes/2026-05-24-nursery-m0-evidence.md`

### Step 1: 全量测试确保基线干净

- [ ] 运行全量测试确认所有现有测试通过。

```bash
cargo test --manifest-path bionic_v0/Cargo.toml 2>&1 | tail -5
```

Expected: `test result: ok. 0 failed` (全部通过).

### Step 2: Release build 验证

- [ ] Release 编译确保 nokhwa/cpal 链接正确。

```bash
cargo build --release --manifest-path bionic_v0/Cargo.toml 2>&1 | tail -5
```

Expected: `Finished` with no errors.

### Step 3: 摄像头权限说明与验证

- [ ] 确认 macOS 摄像头权限。在 macOS 上，首次调用 AVFoundation 摄像头时系统会弹出权限对话框。如果之前被拒绝，需要用户到「系统设置 → 隐私与安全性 → 摄像头」中手动开启终端（Terminal.app 或使用的终端模拟器）的摄像头权限。

无需代码变更——body.rs 已有权限失败时自动 fallback 到 Stub + phot_health=FAULT 的机制。

验证方法：启动 body 后检查 observer 输出中的 `phot_health` 字段。

### Step 4: 麦克风权限说明

- [ ] 同摄像头——macOS 会弹出麦克风权限对话框。cpal 在打开默认输入设备时触发。权限被拒时 VIB 通道无数据流入，observer 的 `ambient_vib.event_energy` 持续为 0。

### Step 5: 编写启动脚本

- [ ] 创建 `scripts/start_nursery.sh`：

```bash
#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MANIFEST="$ROOT/bionic_v0/Cargo.toml"
RUNTIME_DIR="$ROOT/runtime_nursery"
mkdir -p "$RUNTIME_DIR"

echo "=== Live Human Nursery v0 ==="
echo "runtime: $RUNTIME_DIR"
echo ""

# Terminal 1: body (silicon-life process)
echo "[1/3] Starting body..."
cargo run --release --manifest-path "$MANIFEST" --bin body -- "$RUNTIME_DIR" &
BODY_PID=$!
echo "  body pid=$BODY_PID"

sleep 2

# Terminal 2: observer (numeric snapshot viewer)
echo "[2/3] Starting observer..."
cargo run --release --manifest-path "$MANIFEST" --bin observer -- "$RUNTIME_DIR" 20 &
OBSERVER_PID=$!
echo "  observer pid=$OBSERVER_PID"

sleep 1

# Terminal 3: surface (pixel expression viewer)
echo "[3/3] Starting surface..."
cargo run --release --manifest-path "$MANIFEST" --bin observer_surface -- "$RUNTIME_DIR" &
SURFACE_PID=$!
echo "  surface pid=$SURFACE_PID"

echo ""
echo "All processes running. Press Enter to stop..."
read -r

kill $BODY_PID $OBSERVER_PID $SURFACE_PID 2>/dev/null || true
wait $BODY_PID $OBSERVER_PID $SURFACE_PID 2>/dev/null || true
echo "Nursery stopped."
```

```bash
chmod +x scripts/start_nursery.sh
```

### Step 6: 编写停止脚本

- [ ] 创建 `scripts/stop_nursery.sh`：

```bash
#!/usr/bin/env bash
set -euo pipefail

echo "Stopping nursery processes..."
pkill -f "bionic_v0.*bin/body" 2>/dev/null || true
pkill -f "bionic_v0.*bin/observer" 2>/dev/null || true
pkill -f "bionic_v0.*bin/observer_surface" 2>/dev/null || true
sleep 1
echo "Done. Remaining bionic_v0 processes:"
pgrep -fl "bionic_v0" || echo "  (none)"
```

```bash
chmod +x scripts/stop_nursery.sh
```

### Step 7: 硬件冒烟测试

- [ ] 启动 body 10 秒，检查 observer 输出中三路硬件状态。

```bash
# 启动 body 后台运行 10 秒
cargo run --release --manifest-path bionic_v0/Cargo.toml --bin body -- /tmp/runtime_smoke &
BODY_PID=$!
sleep 10
kill $BODY_PID 2>/dev/null || true
wait $BODY_PID 2>/dev/null || true

# 检查 observer 输出
cargo run --release --manifest-path bionic_v0/Cargo.toml --bin observer -- /tmp/runtime_smoke 1 2>&1
```

检查 observer 输出中的以下字段：
- `phot_health`: 应为 `1`（OK）或 `0`（FAULT——如果权限未授）
- `ambient_vib.event_energy`: 有环境声音时应 > 0
- `vocal_target.active_energy`: 可能为 0（无教学信号时正常）

### Step 8: M0 证据记录

- [ ] 创建 `docs/superpowers/notes/2026-05-24-nursery-m0-evidence.md`，记录：

```markdown
# Live Human Nursery M0 — Hardware Readiness Evidence

**Date:** 2026-05-24
**Machine:** MacBook Pro M1
**OS:** macOS [version]

## Build

- `cargo test`: [PASS/FAIL]
- `cargo build --release`: [PASS/FAIL]

## Hardware Status

| Channel | Status | Evidence |
|---------|--------|----------|
| Camera (PHOT) | [OK/FAULT] | phot_health=[value] |
| Microphone (VIB) | [OK/FAULT] | ambient_vib.event_energy=[value] |
| Speaker (Vocal) | [OK/FAULT] | vocal output [tested/not tested] |

## Notes

[Any observations]
```

### Step 9: Commit M0

- [ ] 提交 scripts 和 evidence。

```bash
git add scripts/start_nursery.sh scripts/stop_nursery.sh docs/superpowers/notes/2026-05-24-nursery-m0-evidence.md
git commit -m "feat: add nursery startup scripts and M0 hardware evidence"
```

---

## Task 2: M1 — Phase 0-1 生命迹象 + 声音轮廓

**Files:**
- Create: `docs/superpowers/notes/2026-05-24-nursery-m1-evidence.md`

### Step 1: 运行 nursery 预检 gate

- [ ] 在 live session 前运行确定性 gate。

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test live_io_preflight
cargo test --manifest-path bionic_v0/Cargo.toml --test auditory_visual
cargo test --manifest-path bionic_v0/Cargo.toml --test surface_projection --test surface_render
cargo test --manifest-path bionic_v0/Cargo.toml --test body_no_human_output
cargo test --manifest-path bionic_v0/Cargo.toml --test live_human_nursery
```

Expected: ALL PASS.

### Step 2: 启动 nursery

- [ ] 运行启动脚本。

```bash
./scripts/start_nursery.sh
```

### Step 3: Phase 0 — Life Signs

- [ ] 执行 runbook Phase 0 操作：
  1. 在麦克风附近发出短促声音
  2. 在摄像头前移动一个简单物体或亮色块
  3. 观察 observer 输出和 surface 窗口

- [ ] 在 observer 输出中确认：
  - PHOT health 可见
  - VIB energy 或 windows 响应声音
  - snapshot 序列号递增
  - surface 非空白（当 pressure 存在时）

### Step 4: Phase 1 — Vocal Contours

- [ ] 执行 runbook Phase 1 操作：
  1. 发出高音然后低音的声音轮廓
  2. 发出短促然后悠长的声音轮廓
  3. 重复一个双脉冲轮廓
  4. 暂停，然后再次重复相同的轮廓

- [ ] 在 observer 输出中确认：
  - vocal target energy 在外部声音后变化
  - vocal organ 可能有输出尝试
  - imitation pressure 随 gap 变化

### Step 5: 运行 nursery evidence report

- [ ] 用 curriculum_report 检查 nursery evidence。

```bash
cargo run --release --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --live-human-nursery runtime_nursery
```

检查输出中的 `communication_candidate` 和 `teacher_phase_count`。

### Step 6: M1 证据记录

- [ ] 创建 `docs/superpowers/notes/2026-05-24-nursery-m1-evidence.md`，记录 Phase 0-1 观察到的数值证据。

### Step 7: Commit M1

```bash
git add docs/superpowers/notes/2026-05-24-nursery-m1-evidence.md
git commit -m "docs: record nursery M1 phase 0-1 life signs and vocal contour evidence"
```

---

## Task 3: M2-M4 — Phase 2-3 与 communication_candidate

**Files:**
- Create: `docs/superpowers/notes/2026-05-24-nursery-m2-m4-evidence.md`

M2-M4 是在多次 live session 中逐步达成的，不是一次性执行。以下步骤在每次 session 后重复。

### Step 1: Phase 2 — Visual-Audio Coupling（每次 session）

- [ ] 执行 runbook Phase 2 操作：
  1. 将一个可见的明亮/移动标记与一个声音轮廓配对
  2. 将一个静止或暗淡标记与不同的声音轮廓配对
  3. 故意错配一个视觉事件和声音

- [ ] 在 observer/surface 中确认：
  - PHOT slots 和 spatial regions 出现
  - 对齐的 VIB/PHOT activity 提高 coupling evidence
  - 错配提高 drift 或 ambiguity 而非虚假确定

### Step 2: Phase 3 — Turn-Taking And Repair（多次 session 后）

- [ ] 执行 runbook Phase 3 操作：
  1. 行动，暂停，等待 body 输出或 surface pressure 变化
  2. 如果反应弱则重复
  3. 通过改变 timing 或 contour 进行修正

- [ ] 在 observer/surface 中确认：
  - body 输出时出现 action-output epochs
  - consequence windows 仅在合法时配对
  - closure 降低 pressure
  - reentry 恢复 traces 优于 cold exposure

### Step 3: communication_candidate gate 检查

- [ ] 当所有 Phase 0-3 数值证据可见时，运行：

```bash
cargo run --release --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --live-human-nursery runtime_nursery
```

确认输出包含：
```
deterministic_fixture=0
runtime_snapshot_read=1
communication_candidate=1
```

### Step 4: M2-M4 证据记录

- [ ] 创建 `docs/superpowers/notes/2026-05-24-nursery-m2-m4-evidence.md`，记录每次 session 的数值证据。

### Step 5: Commit

```bash
git add docs/superpowers/notes/2026-05-24-nursery-m2-m4-evidence.md
git commit -m "docs: record nursery M2-M4 evidence and communication candidate gate"
```

---

## Task 4: 原则守护 Gate 验证

在每个 milestone 完成后执行。

### Step 1: G1 — 无人类语言污染

- [ ] 扫描 body 源码确认无语义污染。

```bash
grep -r "println\|eprintln\|format!" bionic_v0/src/ --include="*.rs" | grep -v "bin/\|tests/\|experimental/" || echo "G1 PASS: no body-side println"
```

Expected: 仅在 `bin/` 和 `tests/` 中出现（observer 工具链）。

### Step 2: G2 — 观察与身体分离

- [ ] 确认 body stdout 为空。

```bash
cargo run --release --manifest-path bionic_v0/Cargo.toml --bin body -- /tmp/runtime_gate 2>&1 &
BODY_PID=$!
sleep 5
kill $BODY_PID 2>/dev/null || true
wait $BODY_PID 2>/dev/null || true
# Body process stdout should be empty
```

### Step 3: G3 — 数值 substrate 完整

- [ ] 确认 observer 能从 snapshot 重建完整画面。

```bash
cargo run --release --manifest-path bionic_v0/Cargo.toml --bin observer -- /tmp/runtime_gate 1 2>&1 | head -40
```

确认输出为纯数值字段，无语义标签。

### Step 4: G4 — 持续运行不死机

- [ ] 运行中 body 进程不 panic/退出/OOM。

```bash
# 检查 body 进程存活
pgrep -fl "bin/body" || echo "G4 WARNING: body not running"
```

### Step 5: G5 — 不可逆原则

- [ ] diff 检查无新增禁止模式。

```bash
git diff main..HEAD -- bionic_v0/src/ | grep -i "chat\|nlp\|tokeniz\|semantic\|concept_id\|symbolizer\|request.response" || echo "G5 PASS: no forbidden patterns"
```

---

## 不在此 plan 范围内

- Layer 2（自我模型建立）— 依赖 Layer 1 M4 完成后
- Layer 3（持续生命运行）— 依赖 Layer 2 完成后
- 性格系统 / viewport_policy
- 机械臂
- 伦理监测第二/三层
- 任何 `AGENTS.md` / `consciousness-stream-architecture.md` / `tier0-base-elements.md` 的修改
