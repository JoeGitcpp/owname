# Learned Bias / Snapshot 分离 — 执行方案

> 日期：2026-05-28
> 依据：docs/superpowers/specs/2026-05-28-learned-bias-snapshot-boundary-design.md
> 状态：待执行

---

## 执行摘要

将 `VocalReflexState` 中的 `familiarity` 和 `suppression_strength` 迁移到 `LearnedBias`，同时用 predict-err-update 循环替代一刀切回音阻断。

- 改动 5 个文件
- 预计 ~150 行代码变更
- 不改变 body 的启动方式或对外接口
- 需要重新编译（`cargo build --release`）

---

## 执行步骤

### Step 1 · 扩展 LearnedBias

**文件**：`bionic_v0/src/learned_bias.rs`

**新增字段**：

```rust
pub struct LearnedBiasSummary {
    // ... 现有字段保持不变 ...

    /// 发声熟悉度。VPC 低层 predictor 在 closure 事件中缓慢积累。
    /// 高 = 对发声后果有稳定预期，反射抑制增强。
    /// 范围 [0.0, 1.0]，初始值 0.0（新生儿）。
    pub vocal_familiarity: f64,

    /// 发声抑制强度。VPC 低层 predictor 在"安静+观察到闭环"中积累。
    /// 高 = 更倾向于安静，需要更高的内部压力才触发反射。
    /// 范围 [0.0, 1.0]，初始值 0.0（新生儿）。
    pub vocal_suppression: f64,
}
```

**变更**：
- `Default::default()` 中新增字段初始化为 0.0
- `write_to()` 序列化新增两个 f64
- `read_from()` 反序列化新增两个 f64
- `summary()` 不变（自动包含）
- 文件格式版本号递增

**验证**：learned_bias.bin 文件大小增加 16 字节（2 × f64）

---

### Step 2 · 精简 VocalReflexState

**文件**：`bionic_v0/src/vocal_reflex.rs`

**删除字段**：
```rust
// 删除这两个字段：
// pub familiarity: f64,            → 迁移到 LearnedBias.vocal_familiarity
// pub suppression_strength: f64,   → 迁移到 LearnedBias.vocal_suppression
```

**修改 `tick_drive` 方法签名和实现**：

之前：
```rust
pub fn tick_drive(&mut self, now_us: u64, drive: VocalReflexDrive) -> Option<VocalReflexBurst>
```

之后（通过参数接收 learned_bias 值）：
```rust
pub fn tick_drive(
    &mut self,
    now_us: u64,
    drive: VocalReflexDrive,
    vocal_familiarity: f64,
    vocal_suppression: f64,
) -> Option<VocalReflexBurst>
```

内部计算改为：
```rust
// 原来：let regulating_pressure = self.familiarity * 0.35 + self.suppression_strength * 0.45;
// 改为：
let regulating_pressure = vocal_familiarity * 0.35 + vocal_suppression * 0.45;
```

**修改 `tick_drive` 调用点**：

**文件**：`bionic_v0/src/viewport.rs` — `tick_vocal_reflex_from_pressure`

```rust
pub fn tick_vocal_reflex_from_pressure(&self, now_us: u64) {
    // ... 前面不变 ...

    let bias = self.read_learned_bias();
    let burst = self.vocal_reflex.write().unwrap().tick_drive(
        now_us, drive,
        bias.vocal_familiarity,    // 新增
        bias.vocal_suppression,    // 新增
    );
    // ... 后面不变 ...
}
```

---

### Step 3 · 修改 observe_closure

**文件**：`bionic_v0/src/vocal_reflex.rs`

**修改 `observe_closure` 方法**：

```rust
/// 返回 learned_bias 需要应用的增量
pub fn observe_closure(
    &mut self,
    now_us: u64,
    closure_support: f64,
    vocalized: bool,
) -> (f64, f64) {  // 返回 (delta_familiarity, delta_suppression)
    let closure = closure_support.clamp(0.0, 1.0);
    let mut delta_fam = 0.0;
    let mut delta_sup = 0.0;

    if vocalized {
        delta_fam = closure * 0.01;           // 从 0.10 降到 0.01
        self.fast_gain = (self.fast_gain * (1.0 - closure * 0.30)).clamp(0.0, 1.0);
    } else {
        self.quiet_regulation_successes = self.quiet_regulation_successes.saturating_add(1);
        delta_sup = closure * 0.01;            // 从 0.18 降到 0.01
        delta_fam = closure * 0.01;            // 从 0.13 降到 0.01
        self.fast_gain = (self.fast_gain * (1.0 - closure * 0.45)).clamp(0.0, 1.0);
    }

    self.reflex_pressure = (self.reflex_pressure * (1.0 - closure * 0.35)).clamp(0.0, 1.0);
    self.last_update_t_us = now_us;

    (delta_fam, delta_sup)
}
```

**修改 `observe_closure` 调用点**：

**文件**：`bionic_v0/src/viewport.rs` — 对应的调用位置

```rust
// 原来调用 observe_closure 之后不管返回值
// 改为：
let (delta_fam, delta_sup) = reflex.observe_closure(now_us, closure_support, vocalized);
if delta_fam > 0.0 || delta_sup > 0.0 {
    self.apply_learned_bias_delta(delta_fam, delta_sup);
}
```

**新增方法**：
```rust
fn apply_learned_bias_delta(&self, delta_fam: f64, delta_sup: f64) {
    let mut bias = self.learned_bias.write().unwrap();
    bias.vocal_familiarity = (bias.vocal_familiarity + delta_fam).clamp(0.0, 1.0);
    bias.vocal_suppression = (bias.vocal_suppression + delta_sup).clamp(0.0, 1.0);
}
```

---

### Step 4 · 回音消除：阻断改为 precision 降级

**文件**：`bionic_v0/src/viewport.rs`

**修改 `tick_vocal_target_from_vib`**：

```rust
pub fn tick_vocal_target_from_vib(&self, now_us: u64) {
    let ambient = self.read_ambient();
    let vib = ambient.vib;
    if !vib.candidate_flag || vib.event_energy < VOCAL_TARGET_INGRESS_MIN_ENERGY {
        return;
    }
    if now_us.saturating_sub(vib.last_event_t_us) > QUIET_WINDOW_US {
        return;
    }

    // ── 替换 has_recent_write_coupled_vib_caution ──
    let self_match = self.compute_vib_self_match();
    if self_match > 0.7 {
        // 大概率是自己的回声
        let reduced_energy = vib.event_energy * (1.0 - self_match).max(0.1);
        if reduced_energy < VOCAL_TARGET_INGRESS_MIN_ENERGY {
            return;  // 纯回声，不设 target
        }
        // 异常回声（变了/被反射了）→ 仍然追踪但降权
    }
    // ── 替换结束 ──

    // ... 后面不变：读 bands、计算 spectral_center 等 ...
}
```

**新增 `compute_vib_self_match` 方法**：

```rust
/// 计算当前 VIB 信号和 vocal_organ 输出预测的匹配度
/// 返回值 [0.0, 1.0]：
///   1.0 = VIB 信号完全来自自己发声
///   0.0 = VIB 信号完全来自外部
fn compute_vib_self_match(&self) -> f64 {
    let organ = self.read_vocal_organ();
    if organ.phase != VocalOrganPhase::Active || organ.breath_drive < 0.01 {
        return 0.0;  // 没在发声，不可能是回声
    }

    let now_us = self.clock_us.load(Ordering::Relaxed);
    let last_write = self.action_output.read().unwrap().last_write_t_us;
    let since_last_write = now_us.saturating_sub(last_write);

    if since_last_write > VOCAL_TARGET_WRITE_COUPLED_GUARD_US {
        return 0.0;  // 太久没发声
    }

    let ambient = self.read_ambient();
    let vib = ambient.vib;

    // 匹配度 = f(时序接近度, 能量比)
    let time_closeness =
        1.0 - (since_last_write as f64 / VOCAL_TARGET_WRITE_COUPLED_GUARD_US as f64);
    let energy_match =
        (vib.event_energy / organ.breath_drive.max(0.01)).clamp(0.0, 1.0).min(1.0);

    (time_closeness * 0.6 + energy_match * 0.4).clamp(0.0, 1.0)
}
```

---

### Step 5 · Snapshot 序列化更新

**文件**：`bionic_v0/src/snapshot.rs`

**VocalReflexState 序列化**（`write_vocal_reflex` / `read_vocal_reflex`）：
- 移除 `familiarity` 的写入和读取
- 移除 `suppression_strength` 的写入和读取
- 保持其他字段不变

**LearnedBiasSummary 序列化**（如果 snapshot 中包含 learned_bias summary）：
- 确保 `vocal_familiarity` 和 `vocal_suppression` 正确读写

---

## 影响范围汇总

| 文件 | 改动行数 | 改动类型 |
|------|---------|---------|
| `src/learned_bias.rs` | ~30 行 | 新增 2 字段 + 序列化 |
| `src/vocal_reflex.rs` | ~40 行 | 删 2 字段 + tick_drive 改签名 + observe_closure 改返回值 |
| `src/viewport.rs` | ~60 行 | 新增方法 + 改调用点 |
| `src/snapshot.rs` | ~15 行 | 序列化更新 |
| `tests/` | ~50 行 | 单元测试 |

---

## 验证清单

### 编译验证
- [ ] `cargo build --release` 成功
- [ ] `cargo test` 全部通过

### 行为验证
- [ ] Body 启动后发声行为与改动前一致（新生儿状态，familiarity=0, suppression=0）
- [ ] Body 运行 1 小时后 familiarity 和 suppression 值 > 0.05
- [ ] Body 运行 10 小时后，在安静环境下的自发发声频率明显降低
- [ ] 外部声音（人说话）能正常触发 vocal_target（不因回音消除被阻断）

### 持久化验证
- [ ] 重启 body（保留 learned_bias.bin）→ 发声行为与重启前一致
- [ ] 删除 learned_bias.bin 后重启 → 发声行为回到新生儿
- [ ] learned_bias.bin 大小增加 16 字节

### 回音消除验证
- [ ] Body 发声时无人说话 → vocal_target 不触发
- [ ] Body 发声时有人说话 → vocal_target 正常触发
- [ ] Body 连续发声 1 分钟 → 不陷入自激循环

---

## 回滚方案

如果改动后行为异常：

1. `git checkout -- bionic_v0/src/learned_bias.rs bionic_v0/src/vocal_reflex.rs bionic_v0/src/viewport.rs bionic_v0/src/snapshot.rs`
2. `cargo build --release`
3. 重新启动 body

learned_bias.bin 新增的 16 字节会被旧代码忽略（读取时跳过未知字段），无需特殊处理。
