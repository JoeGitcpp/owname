# Learned Bias 扩展 + Vocal Reflex 重构 实施计划

> 日期：2026-05-28
> 依据：2026-05-28-learned-bias-snapshot-boundary-design.md
> 状态：待审批

---

## 目标

将 `VocalReflexState` 中的 `familiarity` 和 `suppression_strength` 从瞬时状态迁移到 `LearnedBias`，使其成为"死过一次也能保留"的底层偏置。

同步修复回音消除：用 predict-err-update 循环替代一刀切阻断。

---

## Step 1 · LearnedBias 扩展

**文件：`bionic_v0/src/learned_bias.rs`**

在 `LearnedBias` struct 中新增两个字段：

```rust
pub struct LearnedBias {
    // ... 现有字段不变 ...

    /// 发声熟悉度。高 = 对发声后果有稳定预期。
    /// VPC 低层 predictor 在 closure 事件中缓慢积累。
    /// 死后保留。
    pub vocal_familiarity: f64,        // [0.0, 1.0]

    /// 发声抑制强度。高 = 更倾向于安静。
    /// VPC 低层 predictor 在"安静 + 观察到闭环"事件中积累。
    /// 死后保留。
    pub vocal_suppression: f64,        // [0.0, 1.0]
}
```

**约束**（遵循原则文档约束 2）：
- 单次事件修改幅度 ≤ 0.01
- 初始值：`vocal_familiarity = 0.0`, `vocal_suppression = 0.0`（新生儿）

**序列化**：
- `learned_bias.bin` 写入时包含这两个字段
- `learned_bias.bin` 读取时解析这两个字段
- snapshot 不存（原则：learned_bias 独立于 snapshot）

---

## Step 2 · VocalReflexState 精简

**文件：`bionic_v0/src/vocal_reflex.rs`**

从 `VocalReflexState` 中移除 `familiarity` 和 `suppression_strength`：

```rust
pub struct VocalReflexState {
    pub reflex_pressure: f64,          // 留在 snapshot
    pub fast_gain: f64,                // 留在 snapshot
    // familiarity         → learned_bias (移除)
    // suppression_strength → learned_bias (移除)
    pub prior_drive: f64,
    pub burst_count: u32,              // 留在 snapshot
    pub bypass_count: u32,             // 留在 snapshot
    pub quiet_regulation_successes: u32, // 留在 snapshot
    pub last_update_t_us: u64,
}
```

**`tick_drive` 方法修改**：

原来：
```rust
let regulating_pressure = self.familiarity * 0.35 + self.suppression_strength * 0.45;
```

改为从 learned_bias 读取：
```rust
// 通过 viewport 读取 learned_bias（公理 8：所有组件读视口）
let bias = vp.read_learned_bias();
let regulating_pressure = bias.vocal_familiarity * 0.35 + bias.vocal_suppression * 0.45;
```

**`observe_closure` 方法修改**：

原来是更新 `self.familiarity` 和 `self.suppression_strength`，改为写 learned_bias：

```rust
pub fn observe_closure(&mut self, now_us: u64, closure_support: f64, vocalized: bool, vp: &Viewport) {
    let closure = closure_support.clamp(0.0, 1.0);
    let mut bias = vp.read_learned_bias().clone();

    if vocalized {
        // 发声后：熟悉度缓慢上升，快速增益下降
        bias.vocal_familiarity = (bias.vocal_familiarity + closure * 0.01).clamp(0.0, 1.0);
        // fast_gain 仍在 snapshot 里降
        self.fast_gain = (self.fast_gain * (1.0 - closure * 0.30)).clamp(0.0, 1.0);
    } else {
        // 安静 + 观察到闭环：抑制力上升
        self.quiet_regulation_successes = self.quiet_regulation_successes.saturating_add(1);
        bias.vocal_suppression = (bias.vocal_suppression + closure * 0.01).clamp(0.0, 1.0);
        bias.vocal_familiarity = (bias.vocal_familiarity + closure * 0.01).clamp(0.0, 1.0);
        self.fast_gain = (self.fast_gain * (1.0 - closure * 0.45)).clamp(0.0, 1.0);
    }

    vp.write_learned_bias(&bias);
    self.reflex_pressure = (self.reflex_pressure * (1.0 - closure * 0.35)).clamp(0.0, 1.0);
    self.last_update_t_us = now_us;
}
```

**关键变更**：每次 closure 事件对 learned_bias 的修改幅度从 0.10/0.18 降到 **0.01**（遵循约束 2，单次事件 ≤ 0.01）。

---

## Step 3 · 回音消除：precision 降级替代阻断

**文件：`bionic_v0/src/viewport.rs`** — `tick_vocal_target_from_vib` 方法

**当前逻辑**：
```rust
if self.has_recent_write_coupled_vib_caution(now_us) {
    return;  // 一刀切阻断：完全不反应
}
```

**新逻辑**：
```rust
// 计算 VIB 信号和 vocal_organ 输出预测的匹配度
let self_match = self.compute_vib_self_match();
// self_match: f64, [0.0, 1.0]
// 1.0 = VIB 信号完全来自自己发声（回声）
// 0.0 = VIB 信号完全来自外部

if self_match > 0.7 {
    // 大概率是自己的回声 → 降精度，不完全阻断
    // 但保留突变检测：如果回声异常 → 仍可升级
    let reduced_energy = vib.event_energy * (1.0 - self_match).max(0.1);
    if reduced_energy < VOCAL_TARGET_INGRESS_MIN_ENERGY {
        // 回声太弱，不值得追踪 → 不设 target
        // 但可以触发 closure（"我说的话被听到了"）
        return;
    }
    // 回声异常（变了/被挡住了）→ 仍然追踪，但精度降低
}

// 外部声音或异常回声 → 正常 vocal_target 流程
self.observe_vocal_target(now_us, ...);
```

**新增方法**：`compute_vib_self_match()`

```rust
fn compute_vib_self_match(&self) -> f64 {
    let organ = self.read_vocal_organ();
    if organ.phase != VocalOrganPhase::Active || organ.breath_drive < 0.01 {
        return 0.0;  // 没在发声 → 不可能是回声
    }

    let ambient = self.read_ambient();
    let vib = ambient.vib;

    // 简化的匹配：比对 VIB 能量的时序和 vocal_organ 的活动时序
    // 时间窗口：近 300ms 内（VOCAL_TARGET_WRITE_COUPLED_GUARD_US）
    let now_us = self.clock_us.load(Ordering::Relaxed);
    let since_last_write = now_us.saturating_sub(
        self.action_output.read().unwrap().last_write_t_us
    );

    if since_last_write > VOCAL_TARGET_WRITE_COUPLED_GUARD_US {
        return 0.0;  // 太久没发声，不可能是回声
    }

    // 匹配度 = f(时序接近度, 能量一致性)
    // 时序越近 + 能量越接近 → 越可能是回声
    let time_closeness = 1.0 - (since_last_write as f64 / VOCAL_TARGET_WRITE_COUPLED_GUARD_US as f64);
    let energy_match = (vib.event_energy / organ.breath_drive.max(0.01)).clamp(0.0, 1.0).min(1.0);

    (time_closeness * 0.6 + energy_match * 0.4).clamp(0.0, 1.0)
}
```

---

## Step 4 · Snapshot 序列化更新

**文件：`bionic_v0/src/snapshot.rs`**

**VocalReflexState 序列化**：移除 `familiarity` 和 `suppression_strength` 的读写。

**LearnedBias 序列化**：确保 `vocal_familiarity` 和 `vocal_suppression` 在 `learned_bias.bin` 中读写。

**文件：`bionic_v0/src/learned_bias.rs`**

`LearnedBias::write_to()` 和 `LearnedBias::read_from()` 增加新字段。

---

## Step 5 · Viewport 接口更新

**文件：`bionic_v0/src/viewport.rs`**

新增/修改方法：

```rust
// 读取 learned_bias（用于 reflex tick_drive）
pub fn read_learned_bias(&self) -> LearnedBias;

// 写入 learned_bias（用于 reflex observe_closure）
pub fn write_learned_bias(&self, bias: &LearnedBias);

// 计算 VIB 自匹配（用于 vocal_target 回音消除）
fn compute_vib_self_match(&self) -> f64;
```

---

## 影响范围

| 文件 | 改动类型 |
|------|---------|
| `src/learned_bias.rs` | 扩展字段 + 序列化 |
| `src/vocal_reflex.rs` | 移除字段 + tick_drive/observe_closure 改读 learned_bias |
| `src/viewport.rs` | 新增 compute_vib_self_match + 接口 + tick_vocal_target 改为 precision 降级 |
| `src/snapshot.rs` | VocalReflexState 序列化移除旧字段 |
| `src/bin/body.rs` | 无需改动（接口兼容） |

---

## 验证方法

1. **单元测试**：`VocalReflexState` 新行为验证
   - `tick_drive` 从 `learned_bias` 读取 regulating_pressure
   - `observe_closure` 以 0.01 幅度写入 `learned_bias`
   - 100 次 closure 后 `vocal_suppression` 应达到 ~0.63（1 - (1-0.01)^100 ≈ 0.634）

2. **集成测试**：重启后行为连续性
   - 运行 body 2 小时 → 记录 reflex 行为
   - 重启 body（不删 learned_bias.bin）→ 确认行为基线一致
   - 重启 body（删掉 learned_bias.bin）→ 确认行为回到新生儿

3. **回音消除测试**：
   - Body 发声时外部有人说话 → 确认 vocal_target 触发
   - Body 发声时无人说话 → 确认 vocal_target 不触发
   - Body 连续发声时 → 确认不陷入无限自激

---

## 不在此计划内的

- 多源 VIB（前端信号处理，独立项目）
- ViewportPolicy 情绪锁定（未来性格系统）
- VocalMemory 的持久化裁决（待分析）
- VocalImitationState 的持久化裁决（待分析）
