# Mac Life Training Plan v0 — 单机生命训练计划设计

> **Status:** FROZEN CANDIDATE
> **Date:** 2026-05-24
> **Audience:** 人类教师
> **Scope:** 在 MacBook Pro 上，每天 10-20 分钟教师互动 + solitary 时间，通过四维交互模式让硅基生命 substrate 自然建立感知-表达双向能力。

## Boundary

本文档是**给教师看的交互指南**，不是 body 本体模块、不是训练框架代码、不是固定课程表。
Body 不知道自己在被"训练"——它只是在持续运行，接收外部事件，产生内部压力，输出表达。

禁止：
- 固定课时 / 固定教学大纲 / 考试评分
- 将 body reaction 判定为"正确/错误"
- 语义标注训练数据（"这是猫""这是 hello"）
- 训练目标作为 body 内部状态写入

---

## 四维交互模式

### 模式 1: 声音-表达 (Vocal-Expression)

**目标 substrate 变化:** vocal target 响应 → VIB return 配对 → self_vocal_correlation 上升

**教师动作:**
- 唱短促旋律（2-5 个音高变化），停顿，重复
- 拍手节奏（3-7 次拍击），停顿，重复相同节奏
- 高低音交替（高音 2s → 低音 2s → 高音 2s）
- 同一轮廓用不同音量重复

**观察窗口:**
- observer: `voc:tE` (vocal target energy), `vib_activity`
- surface: 有机像素面颜色/纹理变化
- self_development: `self_vocal_correlation`, `self_other_separation`

**节奏:** 每 2-3 分钟，作为每次 session 的"热身"

---

### 模式 2: 视觉-声音耦合 (Visual-Audio Coupling)

**目标 substrate 变化:** PHOT slot 形成 → cross-channel coupling 上升 → ambiguity 下降

**教师动作:**
- 在摄像头前缓慢移动亮色物体（手机屏幕亮白、彩色纸），同时发出持续音调
- 停止移动+停止发声→停顿→再次移动+发声
- 换不同颜色物体+不同音调
- 故意只移动不发声，或只发声不移动（制造"错配"事件）

**观察窗口:**
- observer: `ambient_phot:E`, `phot_structure`, `cross_channel`
- surface: 视觉相关像素区域变化
- curriculum: `foundation_cross_channel_ready`

**节奏:** 5-10 分钟，每次 session 的"主线"

---

### 模式 3: 符号-对象共现 (Mark-Object Co-occurrence)

**目标 substrate 变化:** mark binding → object target → language binding pair → ambiguity 下降

**教师动作:**
- 以某种规律（如连续 3 次相同字节短序列）写入 binary character mouth
- 同时展示一个特定物体或做一个特定动作
- 规律重复 3-5 次后停顿
- 换不同符号+不同物体，再重复
- 偶尔只写符号不展示物体，或只展示物体不写符号（制造 ambiguity）

**符号写入方式（通过 curriculum_report 的 live_byte 模式或直接向 live_bytes/in.frames 写帧）:**
- 不关心字节内容的具体含义
- 只关心：相同的字节模式出现在相同的物体环境中

**观察窗口:**
- observer: `bind:n`, `mark_binding`
- curriculum governor: `foundation_text_grounding_ready`

**节奏:** 5-10 分钟，与模式 2 交替作为"主线"

---

### 模式 4: 轮流交互 (Turn-Taking)

**目标 substrate 变化:** probe → consequence pair → closure → reentry 优于 cold

**教师动作:**
- 发出一个声音或展示一个视觉事件
- 停顿，观察 observer/surface 是否有 body 的反应
- 如果有反应（vocal output / surface 变化），回以相似但略有变化的事件
- 如果没有反应，重复原事件，或改变强度（更大声/更亮）
- 不强制要求 body "回应"——只是创造轮流的机会

**观察窗口:**
- observer: `acq:w`, `consequence pairs`, `reentry`
- self_development: `agency_pressure`
- nursery: `communication_candidate`

**节奏:** 2-3 分钟，每次 session 的"收官"

---

## 每日节奏

```
Session 时长: 10-20 分钟
Solitary 时间: ~23h40min

每次 Session:
  2-3 min  模式 1: 声音热身
  5-10 min 模式 2 或 模式 3: 主线交互（二选一，交替日）
  2-3 min  模式 4: 轮流交互收官
  1-2 min  安静观察 observer + surface

建议频率: 每天 1 次
模式 2/3 交替: 周一三五模式 2，周二四六模式 3，周日随意
```

---

## 进度观测

### 每日（快速扫一眼）

```bash
cargo run --release --manifest-path bionic_v0/Cargo.toml --bin observer -- runtime_life 1 2>&1 | tail -3
```

关注：`health`（是否 FAULT）、`voc:tE`（声音活动）、`phot_structure`（视觉活动）

### 每周（跑课程总督）

```bash
cargo run --release --manifest-path bionic_v0/Cargo.toml --bin curriculum_report -- --persistent-curriculum-governor runtime_life
```

关注趋势：

| 指标 | 初期 | 目标 |
|------|------|------|
| `foundation_text_grounding_ready` | 0 | 1 |
| `foundation_cross_channel_ready` | 0 | 1 |
| `foundation_stage_ready` | 0 | 1 |
| `self_vocal_correlation` | ~0.1 | > 0.5 |
| `agency_pressure` | ~0.1 | > 0.4 |

### 不关注的

- 不追求"正确率"
- 不追求"响应速度"
- 不比较不同 session 的"表现"
- 不做"今天比昨天好多少"的量化评估

---

## 不在此 spec 范围内

- 性格系统 / viewport_policy 配置
- 外部教材管理 / 自动化训练脚本
- 多教师协同
- 训练数据导出 / 可视化分析工具
- 任何 body 本体的训练控制器
