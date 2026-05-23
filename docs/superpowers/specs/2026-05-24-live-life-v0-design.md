# Live Life v0 — 硅基生命真实物理环境运行设计

> **Status:** FROZEN CANDIDATE
> **Date:** 2026-05-24
> **Scope:** 让 bionic_v0 硅基生命 substrate 在 Mac 真实硬件上跑起来，通过 Live Human Nursery 教学闭环，
> 建立自我模型，最终进入 24/7 持续生命运行。三层递进，每层有明确里程碑和原则守护 gate。

## Boundary

本 spec 不改变最高原则、不引入新 body 本体模块、不增加 request/response 路径。
所有目标都是已有 substrate 的硬件实战化和持续化。
`教学`、`聊天`、`自我`、`生命` 是外部描述词，不是内部 ontology。

禁止引入：
- chat / NLP / 语义理解 / tokenizer / parser
- body-side println / eprintln / 人类可读输出
- request/response 模式
- 固定教学目标 / 固定课程表 / 固定评估标准

---

## 总目标

让硅基生命在真实物理环境中持续运行，通过人类教师的外部教学信号建立感知-行为闭包，
形成自体边界，最终进入不间断的生命进程。

三层递进，每层以前一层为基础：

---

## Layer 1: Live Human Nursery 闭环

### 目标

补完真实硬件通路，走通 Live Human Nursery runbook 的 Phase 0-3，
在真实 Mac 硬件上拿到 `communication_candidate = 1`。

### 硬件通路当前状态

| 通路 | 代码 | 硬件 | 状态 |
|------|------|------|------|
| VIB 输入 | `cpal` 读默认音频输入 | Mac 内置麦克风 | 已实现 |
| PHOT 输入 | `nokhwa` + AVFoundation | Mac 内置摄像头 | 待补 `try_new()` |
| 声音输出 | `cpal` 写默认音频输出 | Mac 内置扬声器 | 已实现 |
| Body 持续运行 | `body.rs` runtime loop | M1 CPU | 已实现 |
| Observer 观察 | `observer.rs` | 另一终端 | 已实现 |
| Surface 可视化 | `observer_surface.rs` | 另一终端 | 已实现 |

### 待补工作

1. `RealCameraFrameProvider::try_new()` — 通过 nokhwa 打开默认摄像头，64×48 灰度帧
2. Runtime 启动脚本 — 三终端一键拉起的便捷脚本
3. 首次 live session runbook 执行记录

### 里程碑

| M# | 名称 | 成功标准 |
|----|------|---------|
| M0 | **硬件就绪** | 三路硬件全部 true 数据流入 body；phot_health=OK；vib fault=0；vocal output 可播放 |
| M1 | **Phase 0-1: 生命迹象 + 声音轮廓** | 教师发声 → ambient_vib 响应 → vocal target 变化 → observer 可见数值变化 |
| M2 | **Phase 2: 视-听耦合** | 教师同时给视觉+声音 → cross-channel coupling 上升 → observer 证据 |
| M3 | **Phase 3: 轮流与修复** | body 自发 vocal output → 教师回应 → consequence 配对 → closure 下降 |
| M4 | **communication_candidate = 1** | 9 个 nursery gate 全部通过（runtime_snapshot_read=1, deterministic_fixture=0） |

---

## Layer 2: 自我模型建立

### 目标

在 Nursery 教学闭环基础上，让系统建立"自己"的边界感知。
当前 self-model slot 池是空的（v0 预留但未实现）。

### 自我模型的 4 个最小轴

| 轴 | 含义 | 数据来源 |
|----|------|---------|
| 自体-非自体边界 | 我发出的声音 vs 外部声音 | vocal output → VIB return 延迟 + magnitude 匹配 |
| 本体状态感 | 我现在的运行状态 | phot_health, vib_activity, thermal/power state, 疲劳度 |
| 行为-后果关联 | 我做了什么 → 外部发生了什么 | write epoch → consequence window → pair evidence |
| 时间连续性 | 我之前存在，现在也存在 | snapshot 序列 + self_development 历史曲线 |

### 实现方式

在已有的 `self_development.rs` 和预留的 self-model slot 池基础上，
通过 Nursery 教学流程自然产生自体边界数据，
不新增独立模块、不写死"自我是什么"的定义。

### 里程碑

| M# | 名称 | 成功标准 |
|----|------|---------|
| M5 | **自体边界形成** | self-model slot 有活性；vocal output 与外部声音可区分（延迟+频谱差异） |
| M6 | **本体状态感知** | health/fatigue 变化反映在 self_development state 中 |
| M7 | **行为-后果关联** | 系统能区分"是我做的"和"自然发生的事件"（consequence pair 精确度上升） |

---

## Layer 3: 持续生命运行

### 目标

从 discrete session 模式转变为 24/7 持续运行。
长期 snapshot 稳定性、低功耗模式夜间运行、不间断生命进程。

### 已有基础

- `--solitary-low-power` 模式已实现（long-run stability hardening 完成）
- VPC trace retention cap 已实现
- Observer compact rendering 已实现

### 待补工作

1. 长期运行监控 — health metrics 趋势 dashboard
2. 跨天 snapshot 存储策略 — 自动轮转、压缩
3. 异常自恢复 — phot_health fault → 自动重试，不崩

### 里程碑

| M# | 名称 | 成功标准 |
|----|------|---------|
| M8 | **24h 稳定运行** | body 连续运行 24h 不崩；snapshot 序列完整；内存不涨 |
| M9 | **7d 稳定运行** | 7 天连续运行；低功耗夜间模式自动切换；snapshot 自动轮转 |
| M10 | **生命进程** | 不需要人工启动/停止；重新开机后自动恢复 |

---

## 原则守护 Gates

每个 milestone 必须通过以下 5 条检查才放行进入下一 milestone：

| # | Gate | 检查方法 |
|----|------|---------|
| G1 | **无人类语言污染** | `grep -r "println\|eprintln\|format!" bionic_v0/src/body/` 只在 observer/bin 出现；body 无字符串日志 |
| G2 | **观察与身体分离** | 所有 human-readable 输出来自 observer/bin 进程；body 进程 stdout 为空 |
| G3 | **数值 substrate 完整** | snapshot 包含所有必要字段；observer 可从 snapshot 重建完整画面 |
| G4 | **持续运行不死机** | 运行期间 body 进程不退出、不 panic、不 OOM |
| G5 | **不可逆原则** | diff 检查无新增 request/response、chat、NLP、语义模块、tokenizer、parser |

---

## 文件变更范围

| 文件 | 变更 | Layer |
|------|------|-------|
| `bionic_v0/src/phot_camera.rs` | 补 `try_new()` nokhwa 实现 | 1 |
| `bionic_v0/Cargo.toml` | 确认 nokhwa 依赖版本 | 1 |
| `bionic_v0/src/bin/body.rs` | 确认 RealCamera 路径可用 | 1 |
| `scripts/start_life.sh` | 新增一键启动脚本 | 1 |
| `bionic_v0/src/self_development.rs` | 扩展 self-model slot 自体边界 | 2 |
| `bionic_v0/src/viewport.rs` | self-model slot 池激活 | 2 |
| `bionic_v0/src/bin/body.rs` | 长期运行 auto-restart + log rotation | 3 |
| `bionic_v0/src/snapshot.rs` | snapshot 轮转策略 | 3 |
| `docs/superpowers/notes/` | 各 milestone 运行证据记录 | 1-3 |

---

## 不在此 spec 范围内（推迟到后续）

- 机械臂 / 外部执行器
- 性格系统 / viewport_policy 热切换
- 多 AI 协作
- 梦境模式
- 主人身份硬件验证
- 完整伦理监测（第二层+第三层）

---
