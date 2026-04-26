# Layer 0 DNA 偏置扩展 + Surprise 整合 实施计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 把 `EvolutionPriors` 重新接入 crate；扩展 disruption-sensitivity 数据 + lock-weight 机制；在 `VibAdapter` 的 surprise 计算路径上让这些偏置真正影响行为，让 AI 对破坏/痛苦类声学模式产生加强的 surprise 反应（spec §3 + 方案 2 范围）。

**Architecture:**
1. `evolution_priors.rs` 重接入 `lib.rs`，从 orphan 状态恢复
2. 给 `EvolutionPriors` 加 disruption 数据字段 + `ModifyRateCaps` 子结构
3. 加纯函数 `compute_disruption_score()` 用于 TDD
4. `VibAdapter` 持有 `Arc<EvolutionPriors>`（默认值保持向后兼容），在 surprise 计算中应用敏感度权重和 disruption 倍增
5. 端到端测试：模拟 disruption 声学模式 → 验证 smoothed_surprise 被放大

**Tech Stack:** Rust 2021, Cargo, `bionic_v0` crate

**Scope（明确不在本 plan 内）：**
- 只覆盖 VIB（声学）路径；PHOT（视觉）的 disruption 信号 → 后续 plan
- 不动 `intention.rs`（也是 orphan，但属于另一个 plan 的范围）
- 不接 precision 路径（属于方案 3 范围）
- 不实装 spec §2 责任人模块（v0 deferred）

**Spec 引用：**
- `docs/superpowers/specs/2026-04-25-shenxing-design.md` §3（偏置实现策略）
- §3.3 给 planner 的留言（哪些感知模式映射到现有 vs 新增轴 / modify_rate 数值）
- §4.4 反向清单（细节守约 + 不情绪化）

**关键背景（实施前必读）：**
- 当前 `bionic_v0/src/lib.rs` **没有声明** `pub mod evolution_priors;`，所以 `EvolutionPriors` 类型对 crate 其他模块不可见。Task 1 的第一件事就是修这个。
- `bionic_v0/src/intention.rs` 也是 orphan，但**本 plan 不动它**（保持 scope 纯净）。
- `VibAdapter::new(vp)` 在 `src/bin/body.rs:63` 被调用——签名**不可改**，必须用新构造方法补充 priors 注入。
- 现有 `VibAdapter::process_frame` 的 surprise 计算在 `src/adapter.rs:225-234`：
  ```rust
  let mut inst_surprise = 0.0;
  for axis in 0..VIB_BANDS {
      if let Some(e) = vp.latest_err(axis) {
          inst_surprise += e.abs();
      }
  }
  ```
  本 plan 要修改这一段（Task 6）。

---

## Task 1：重接入 `evolution_priors` 到 crate

**目的：** 把 orphan 的 `evolution_priors.rs` 文件接进 `lib.rs`，让其他模块能 `use bionic_v0::evolution_priors::EvolutionPriors`。

**Files:**
- Modify: `bionic_v0/src/lib.rs`（在 `pub mod evolution_priors;` 加进 mod 声明列表）

- [ ] **Step 1.1：跑一次基线测试，确认起点干净**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --lib 2>&1 | tail -10
```

期望：所有现有 lib 测试通过（统计当前数量，记下来）。
注意：因为 `evolution_priors` 还是 orphan，它的 3 个 `#[cfg(test)]` 测试**不会**包含在内。

- [ ] **Step 1.2：在 `lib.rs` 中加入 `pub mod evolution_priors;`**

文件：`bionic_v0/src/lib.rs`
位置：在第 22 行 `pub mod viewport;` 之后（按字母顺序），或与其他 `pub mod` 一起。

加这一行：

```rust
pub mod evolution_priors;
```

- [ ] **Step 1.3：跑 cargo check，确认编译通过**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo check --lib 2>&1 | tail -5
```

期望：`Finished `dev` profile`，无 error，无 warning（除 dead_code 类的）。

- [ ] **Step 1.4：跑 evolution_priors 的现有测试**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --lib evolution_priors 2>&1 | tail -10
```

期望：3 个测试全部 PASS（`test_default_priors`, `test_vocal_converge_threshold`, `test_spatial_attention_center`）。

- [ ] **Step 1.5：跑一次完整测试，确认没有把别的测试搞坏**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test 2>&1 | tail -20
```

期望：所有测试 PASS。如果有失败，**停下来检查**——重接入 orphan 文件不应该引入任何回归。

- [ ] **Step 1.6：提交**

```bash
git add bionic_v0/src/lib.rs
git commit -m "refactor(bionic_v0): re-integrate evolution_priors into crate

evolution_priors.rs existed as a source file but was not declared in
lib.rs, making it orphan code (uncompiled, untested by lib test runs).
Adding pub mod re-integrates it so EvolutionPriors becomes accessible
to other modules. Required prerequisite for the Layer 0 偏置 wiring
work (spec §3, plan 2026-04-26-layer0-dna-bias-surprise-wiring)."
```

---

## Task 2：给 `EvolutionPriors` 加 disruption-sensitivity 数据字段

**目的：** 加 spec §3.2 里列的 disruption 感知敏感度（破坏/痛苦/割裂等）作为新 DNA 项。

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`（加字段、加 default、加测试）

**关键设计：** 默认值的目标是 **新字段在不被显式启用前不改变现有行为**——只要 surprise 计算路径还没读这些字段（Task 6 才读），添加字段本身就不会破坏任何现有测试。

- [ ] **Step 2.1：写失败测试 — 默认值应该满足 spec §3.2 的"对破坏类信号天然敏感"要求**

文件：`bionic_v0/src/evolution_priors.rs` 的 `#[cfg(test)] mod tests` 内部。

加这个测试：

```rust
#[test]
fn test_disruption_sensitivity_defaults() {
    let priors = EvolutionPriors::default();

    // disruption_band_sensitivity should bias toward distress acoustic range
    // (band 2: 2000-5000Hz typically carries cries/screams)
    assert!(
        priors.disruption_band_sensitivity[2] > priors.disruption_band_sensitivity[0],
        "distress band (2) must be more sensitive than band 0"
    );

    // Default disruption multiplier > 1.0 (must amplify, not dampen)
    assert!(
        priors.disruption_surprise_multiplier > 1.0,
        "disruption_surprise_multiplier must amplify (>1.0) by default"
    );

    // Threshold in (0, 1) — sensible range for normalized err
    assert!(priors.disruption_err_threshold > 0.0);
    assert!(priors.disruption_err_threshold < 1.0);

    // Distress acoustic sensitivity > 1.0 (must amplify)
    assert!(priors.distress_acoustic_sensitivity > 1.0);
}
```

- [ ] **Step 2.2：跑测试，确认 FAIL**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests::test_disruption_sensitivity_defaults 2>&1 | tail -15
```

期望：FAIL（编译错误：no field named `disruption_band_sensitivity`）。

- [ ] **Step 2.3：在 `EvolutionPriors` 结构体里加字段**

在结构体定义末尾（`slot_growth_sensitivity` 之后）追加：

```rust
    // --- v2.1 Layer 0 disruption sensitivity (per spec §3.2) ---
    /// Per-band sensitivity boost for VIB disruption signatures.
    /// Index aligns with vib_band_0..3. Bias toward distress acoustic range
    /// (band 2: 2000-5000Hz typically carries cries/screams/破坏noise).
    pub disruption_band_sensitivity: [f64; VIB_BANDS],

    /// Magnitude of |err| (after band sensitivity weighting) above which
    /// the "disruption regime" is considered to have fired this tick.
    pub disruption_err_threshold: f64,

    /// Multiplier applied to surprise when the disruption regime fires.
    /// Must be >1.0 (amplify, never dampen).
    pub disruption_surprise_multiplier: f64,

    /// Extra acoustic sensitivity for distress signals (cries, screams,
    /// sustained high-band energy). Multiplies the band 2 contribution
    /// to disruption score.
    pub distress_acoustic_sensitivity: f64,
```

- [ ] **Step 2.4：在 `Default` 实现里加默认值**

在 `impl Default for EvolutionPriors` 内的字段列表末尾追加：

```rust
            // v2.1 Layer 0 disruption defaults
            // Bias toward bands 1-2 (500-5000Hz) where distress signals live
            disruption_band_sensitivity: [0.5, 1.0, 1.4, 0.8],
            disruption_err_threshold: 0.3,
            disruption_surprise_multiplier: 2.0,
            distress_acoustic_sensitivity: 1.5,
```

- [ ] **Step 2.5：跑 Step 2.1 的测试，确认 PASS**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests::test_disruption_sensitivity_defaults 2>&1 | tail -10
```

期望：PASS。

- [ ] **Step 2.6：跑全 evolution_priors 测试 + 全 lib 测试，确认没破坏现有测试**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --lib evolution_priors 2>&1 | tail -10
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test 2>&1 | tail -10
```

期望：全部 PASS。

- [ ] **Step 2.7：提交**

```bash
git add bionic_v0/src/evolution_priors.rs
git commit -m "feat(evolution_priors): add Layer 0 disruption sensitivity fields

Adds 4 new fields to EvolutionPriors per shenxing spec §3.2:
- disruption_band_sensitivity[VIB_BANDS]: per-band weighting for
  disruption signature, biased toward distress acoustic range (band 2)
- disruption_err_threshold: |err| threshold for entering disruption regime
- disruption_surprise_multiplier: amplifier for surprise on disruption
- distress_acoustic_sensitivity: extra weight for cries/screams band

Defaults chosen so that fields are dormant until Task 6 wires them
into VibAdapter — no behavioral change at this commit."
```

---

## Task 3：加 `ModifyRateCaps` 子结构 + `modify_rate_cap_for()` 方法（lock-weight 机制）

**目的：** spec §3.1 + §3.3 要求 Layer 0 偏置「修改权重锁到极低」。这个 task 加机制（一个值表示每次学习更新允许的最大 |delta|），但**不强制任何 caller 去用**——caller 可以自由忽略（YAGNI），但当未来加学习路径时这个机制可用。

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`

- [ ] **Step 3.1：写失败测试 — `modify_rate_cap_for` 应该对 Layer 0 axis 返回极低值，对普通 axis 返回正常值**

加这个测试：

```rust
#[test]
fn test_modify_rate_caps_layer_0_locked() {
    let priors = EvolutionPriors::default();

    // Layer 0 axes: extremely low cap (essentially immutable)
    let disruption_cap = priors.modify_rate_cap_for(PriorAxis::DisruptionBand);
    let distress_cap = priors.modify_rate_cap_for(PriorAxis::DistressAcoustic);
    assert!(disruption_cap <= 0.005, "disruption cap should be near-immutable, got {}", disruption_cap);
    assert!(distress_cap <= 0.005, "distress cap should be near-immutable, got {}", distress_cap);

    // Non-Layer-0 axes: moderate cap (learning still possible)
    let vib_band_cap = priors.modify_rate_cap_for(PriorAxis::VibBand);
    let vocal_cap = priors.modify_rate_cap_for(PriorAxis::VocalConverge);
    assert!(vib_band_cap > 0.01, "vib_band cap should allow learning, got {}", vib_band_cap);
    assert!(vocal_cap > 0.01, "vocal_converge cap should allow learning, got {}", vocal_cap);

    // Sanity: Layer 0 caps strictly less than non-Layer-0 caps
    assert!(disruption_cap < vib_band_cap);
    assert!(distress_cap < vocal_cap);
}

#[test]
fn test_clamp_modification_respects_cap() {
    let priors = EvolutionPriors::default();

    // Trying to apply a large delta to a locked axis: clamp to cap
    let requested = 0.5;
    let cap = priors.modify_rate_cap_for(PriorAxis::DisruptionBand);
    let applied = priors.clamp_modification(PriorAxis::DisruptionBand, requested);
    assert!(applied.abs() <= cap, "applied {} must respect cap {}", applied, cap);
    assert_eq!(applied.signum(), requested.signum());

    // Small delta within cap: passes through unchanged
    let small_delta = cap * 0.5;
    let applied_small = priors.clamp_modification(PriorAxis::DisruptionBand, small_delta);
    assert!((applied_small - small_delta).abs() < 1e-12);
}
```

- [ ] **Step 3.2：跑测试，确认 FAIL（编译失败：无 PriorAxis enum）**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests::test_modify_rate 2>&1 | tail -15
```

期望：FAIL，"cannot find type `PriorAxis`"。

- [ ] **Step 3.3：实现 `PriorAxis` enum + `ModifyRateCaps` + 方法**

在 `evolution_priors.rs` 的最上方（`use` 之后，`pub struct EvolutionPriors` 之前）加：

```rust
/// Identifies an axis in EvolutionPriors for modification-rate enforcement.
/// Numeric-only (no semantic labels in the AI's body logic).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PriorAxis {
    // Pre-existing axes (allow moderate learning)
    VibBand,
    Surprise,
    VocalConverge,
    SpatialAttention,
    TemporalWindow,
    SlotGrowth,
    // v2.1 Layer 0 axes (near-immutable)
    DisruptionBand,
    DisruptionThreshold,
    DisruptionMultiplier,
    DistressAcoustic,
}

/// Per-axis cap on the absolute value of any single learned delta.
/// Layer 0 axes get caps so low they are effectively immutable.
#[derive(Clone, Debug)]
pub struct ModifyRateCaps {
    pub vib_band: f64,
    pub surprise: f64,
    pub vocal_converge: f64,
    pub spatial_attention: f64,
    pub temporal_window: f64,
    pub slot_growth: f64,
    // Layer 0 — near-immutable
    pub disruption_band: f64,
    pub disruption_threshold: f64,
    pub disruption_multiplier: f64,
    pub distress_acoustic: f64,
}

impl Default for ModifyRateCaps {
    fn default() -> Self {
        Self {
            // Pre-existing axes: moderate learning rate cap
            vib_band: 0.05,
            surprise: 0.05,
            vocal_converge: 0.05,
            spatial_attention: 0.05,
            temporal_window: 0.05,
            slot_growth: 0.05,
            // Layer 0 — orders of magnitude lower (effectively locked)
            disruption_band: 0.001,
            disruption_threshold: 0.001,
            disruption_multiplier: 0.001,
            distress_acoustic: 0.001,
        }
    }
}
```

然后在 `EvolutionPriors` 结构体末尾追加新字段：

```rust
    /// Per-axis modification rate caps. Layer 0 axes are near-immutable.
    pub modify_rate_caps: ModifyRateCaps,
```

在 `impl Default for EvolutionPriors` 末尾追加：

```rust
            modify_rate_caps: ModifyRateCaps::default(),
```

在 `impl EvolutionPriors` 末尾追加方法：

```rust
    /// Look up the modify-rate cap for a given axis.
    pub fn modify_rate_cap_for(&self, axis: PriorAxis) -> f64 {
        match axis {
            PriorAxis::VibBand => self.modify_rate_caps.vib_band,
            PriorAxis::Surprise => self.modify_rate_caps.surprise,
            PriorAxis::VocalConverge => self.modify_rate_caps.vocal_converge,
            PriorAxis::SpatialAttention => self.modify_rate_caps.spatial_attention,
            PriorAxis::TemporalWindow => self.modify_rate_caps.temporal_window,
            PriorAxis::SlotGrowth => self.modify_rate_caps.slot_growth,
            PriorAxis::DisruptionBand => self.modify_rate_caps.disruption_band,
            PriorAxis::DisruptionThreshold => self.modify_rate_caps.disruption_threshold,
            PriorAxis::DisruptionMultiplier => self.modify_rate_caps.disruption_multiplier,
            PriorAxis::DistressAcoustic => self.modify_rate_caps.distress_acoustic,
        }
    }

    /// Clamp a requested delta to the modify-rate cap for the axis.
    /// Returns the clamped value; preserves sign.
    pub fn clamp_modification(&self, axis: PriorAxis, requested_delta: f64) -> f64 {
        let cap = self.modify_rate_cap_for(axis);
        let mag = requested_delta.abs().min(cap);
        requested_delta.signum() * mag
    }
```

- [ ] **Step 3.4：跑两个新测试 + 确认 PASS**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests::test_modify_rate 2>&1 | tail -15
```

期望：两个测试都 PASS。

- [ ] **Step 3.5：跑全 lib 测试，确认无回归**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test 2>&1 | tail -10
```

期望：全部 PASS。

- [ ] **Step 3.6：提交**

```bash
git add bionic_v0/src/evolution_priors.rs
git commit -m "feat(evolution_priors): add lock-weight (modify_rate_caps) mechanism

Adds PriorAxis enum + ModifyRateCaps struct + modify_rate_cap_for() +
clamp_modification(). Layer 0 axes (disruption_*, distress_acoustic)
get caps 50x lower than other axes (0.001 vs 0.05), making them
effectively immutable under any future learning path.

No callers wire learning yet — this commit lays the API per spec §3.1
('修改权重锁到极低'). When a learning path needs to modify a prior, it
must clamp through clamp_modification(axis, delta)."
```

---

## Task 4：加纯函数 `compute_disruption_score()`

**目的：** 把"判断当前 err 是否构成 disruption signature"抽成一个无副作用的纯函数，方便单测。VibAdapter 之后会调用它（Task 6）。

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`

- [ ] **Step 4.1：写失败测试 — disruption score 行为**

加这个测试到 `tests` mod：

```rust
#[test]
fn test_disruption_score_quiet_input_low() {
    let priors = EvolutionPriors::default();
    // All bands have small err — no disruption
    let err = [0.05, 0.05, 0.05, 0.05];
    let score = compute_disruption_score(&err, &priors);
    assert!(score < 0.5, "quiet err should yield low score, got {}", score);
}

#[test]
fn test_disruption_score_distress_band_dominant() {
    let priors = EvolutionPriors::default();
    // Large err in distress band (band 2: 2000-5000Hz)
    let err = [0.05, 0.1, 0.8, 0.1];
    let score = compute_disruption_score(&err, &priors);
    assert!(score > 1.0, "distress band err should produce high score, got {}", score);
}

#[test]
fn test_disruption_score_broadband_burst_high() {
    let priors = EvolutionPriors::default();
    // Sudden broadband large err (e.g. shattering / explosion signature)
    let err = [0.6, 0.7, 0.8, 0.5];
    let score = compute_disruption_score(&err, &priors);
    assert!(score > 1.5, "broadband burst should produce very high score, got {}", score);
}

#[test]
fn test_disruption_score_monotonic_in_err() {
    let priors = EvolutionPriors::default();
    let small = [0.1, 0.1, 0.1, 0.1];
    let medium = [0.3, 0.3, 0.3, 0.3];
    let large = [0.6, 0.6, 0.6, 0.6];
    let s_small = compute_disruption_score(&small, &priors);
    let s_medium = compute_disruption_score(&medium, &priors);
    let s_large = compute_disruption_score(&large, &priors);
    assert!(s_small < s_medium && s_medium < s_large,
        "disruption score must be monotonic in err magnitude: {} {} {}",
        s_small, s_medium, s_large);
}

#[test]
fn test_disruption_score_distress_sensitivity_amplifies_band_2() {
    let mut priors = EvolutionPriors::default();
    let err = [0.05, 0.05, 0.5, 0.05]; // only band 2 has signal

    let baseline = compute_disruption_score(&err, &priors);
    priors.distress_acoustic_sensitivity = 3.0; // crank it up
    let amplified = compute_disruption_score(&err, &priors);

    assert!(amplified > baseline,
        "distress_acoustic_sensitivity bump must amplify band-2-dominant score: baseline {} vs amplified {}",
        baseline, amplified);
}
```

- [ ] **Step 4.2：跑测试，确认 FAIL（compute_disruption_score not found）**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests::test_disruption_score 2>&1 | tail -15
```

期望：FAIL，"cannot find function `compute_disruption_score`"。

- [ ] **Step 4.3：实现 `compute_disruption_score`**

在 `impl EvolutionPriors` **之后**（顶层函数）加：

```rust
/// Compute a numeric disruption score from per-band prediction error.
///
/// Combines:
/// - per-band sensitivity weighting (priors.disruption_band_sensitivity)
/// - distress acoustic boost on band 2 (priors.distress_acoustic_sensitivity)
/// - sum of weighted |err| across bands
///
/// Higher score = more "破坏/痛苦/割裂" signature in the input.
/// Score is unbounded above (caller compares against thresholds).
pub fn compute_disruption_score(err: &[f64; VIB_BANDS], priors: &EvolutionPriors) -> f64 {
    let mut score = 0.0;
    for axis in 0..VIB_BANDS {
        let weighted = err[axis].abs() * priors.disruption_band_sensitivity[axis];
        // Distress acoustic boost on band 2 (2000-5000Hz: cries/screams)
        let with_distress_boost = if axis == 2 {
            weighted * priors.distress_acoustic_sensitivity
        } else {
            weighted
        };
        score += with_distress_boost;
    }
    score
}
```

- [ ] **Step 4.4：跑 disruption_score 测试，确认全部 PASS**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests::test_disruption_score 2>&1 | tail -15
```

期望：5 个测试全 PASS。如果 Step 4.1 的某个常数（`< 0.5`, `> 1.0` 等）不匹配你选的默认值，调整测试断言而不是改默认值——除非默认值本身明显违反 spec §3。

- [ ] **Step 4.5：跑全 lib 测试**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test 2>&1 | tail -10
```

期望：全部 PASS。

- [ ] **Step 4.6：提交**

```bash
git add bionic_v0/src/evolution_priors.rs
git commit -m "feat(evolution_priors): add compute_disruption_score pure function

Maps a per-band err array to a numeric disruption score by combining
band sensitivity weighting + distress acoustic boost on band 2. Higher
score = more disruption signature (sudden broadband err, distress
acoustic patterns, etc).

Pure function — no side effects, easy to unit-test. VibAdapter will
call this from process_frame() in the next task to amplify surprise
when disruption fires."
```

---

## Task 5：让 `VibAdapter` 持有 `Arc<EvolutionPriors>`

**目的：** 给 VibAdapter 注入 priors 引用，**不破坏现有 `VibAdapter::new(vp)` 签名**（body.rs 在用它）。新增 `with_priors()` 构造方法，default 路径自动使用 `EvolutionPriors::default()`。

**Files:**
- Modify: `bionic_v0/src/adapter.rs`（VibAdapter struct + impl）

- [ ] **Step 5.1：写失败测试 — VibAdapter 应能携带 priors**

新建文件：`bionic_v0/tests/vib_adapter_priors.rs`

```rust
//! Verify VibAdapter can be constructed with a custom EvolutionPriors
//! and that it stores the reference.

use bionic_v0::adapter::VibAdapter;
use bionic_v0::evolution_priors::EvolutionPriors;
use bionic_v0::viewport::Viewport;
use std::sync::Arc;

#[test]
fn test_vib_adapter_with_priors_stores_arc() {
    let vp = Arc::new(Viewport::new());
    let mut priors = EvolutionPriors::default();
    priors.disruption_surprise_multiplier = 9.99; // distinguishable sentinel

    let priors_arc = Arc::new(priors);
    let adapter = VibAdapter::with_priors(&vp, priors_arc.clone());

    // Adapter should expose its priors for inspection (test interface)
    assert_eq!(adapter.priors().disruption_surprise_multiplier, 9.99);
}

#[test]
fn test_vib_adapter_default_uses_default_priors() {
    let vp = Arc::new(Viewport::new());
    let adapter = VibAdapter::new(&vp);
    let default_mult = EvolutionPriors::default().disruption_surprise_multiplier;
    assert_eq!(adapter.priors().disruption_surprise_multiplier, default_mult);
}
```

- [ ] **Step 5.2：跑测试，确认 FAIL**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vib_adapter_priors 2>&1 | tail -15
```

期望：FAIL，"no method named `with_priors`" 或 "no method named `priors`"。

- [ ] **Step 5.3：在 `VibAdapter` struct 加字段，加 `with_priors` + `priors` 方法**

文件：`bionic_v0/src/adapter.rs`

(a) 在文件顶部加 import：

定位到 `use crate::vocal_target::...`（line 27 附近），在它后面加：

```rust
use crate::evolution_priors::EvolutionPriors;
```

> 用 `crate::` 路径（同 crate 内部模块），不是 `bionic_v0::`。

(b) 修改 `VibAdapter` struct（line 181 附近），在最后一个字段后追加：

```rust
    priors: Arc<EvolutionPriors>,
```

(c) 修改 `VibAdapter::new`（line 194-217 附近）——它有两个分支（match 的 Ok / Err），两个分支都要在 struct literal 里加 `priors: Arc::new(EvolutionPriors::default()),`。

```rust
impl VibAdapter {
    pub fn new(vp: &Arc<Viewport>) -> Self {
        Self::with_priors(vp, Arc::new(EvolutionPriors::default()))
    }

    pub fn with_priors(vp: &Arc<Viewport>, priors: Arc<EvolutionPriors>) -> Self {
        let fault = Arc::new(AtomicBool::new(false));
        match crate::vib::VibSource::try_new_parts(fault.clone()) {
            Ok((rx, stream)) => Self {
                rx: Some(rx),
                _stream: Some(stream),
                fault,
                smoothed_energy: 0.0,
                smoothed_surprise: 0.0,
                last_pattern_hash: None,
                priors,
            },
            Err(_) => {
                vp.set_hearing_fault(true);
                Self {
                    rx: None,
                    _stream: None,
                    fault,
                    smoothed_energy: 0.0,
                    smoothed_surprise: 0.0,
                    last_pattern_hash: None,
                    priors,
                }
            }
        }
    }

    /// Test-only accessor for the priors reference.
    #[cfg(any(test, debug_assertions))]
    pub fn priors(&self) -> &EvolutionPriors {
        &self.priors
    }
    // ... existing methods continue ...
```

- [ ] **Step 5.4：跑测试，确认 PASS**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test vib_adapter_priors 2>&1 | tail -10
```

期望：两个测试都 PASS。

- [ ] **Step 5.5：跑全测试套件，确认 body.rs 等下游没破**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test 2>&1 | tail -15
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo build --bin body 2>&1 | tail -5
```

期望：所有测试 PASS；body 二进制编译成功。

- [ ] **Step 5.6：提交**

```bash
git add bionic_v0/src/adapter.rs bionic_v0/tests/vib_adapter_priors.rs
git commit -m "feat(adapter): VibAdapter holds Arc<EvolutionPriors>

Adds VibAdapter::with_priors(vp, priors) constructor + priors() accessor.
The pre-existing new(vp) signature is preserved (body.rs:63 unchanged) —
it now delegates to with_priors() with EvolutionPriors::default().

No behavioral change yet: priors are stored but not read in
process_frame(). Wiring into surprise calculation comes in next task."
```

---

## Task 6：在 `VibAdapter::process_frame` 把 priors 接到 surprise 计算

**目的：** 修改 `process_frame` 的 surprise 累加逻辑，让它使用 priors：
1. 每个 axis 的 `|err|` 先乘以 `disruption_band_sensitivity[axis]`
2. 整帧计算 `compute_disruption_score`；如果超过 `disruption_err_threshold`，乘以 `disruption_surprise_multiplier`
3. 默认参数下，对于平凡输入应该和现有行为接近（避免回归）

**Files:**
- Modify: `bionic_v0/src/adapter.rs`（process_frame 方法）

- [ ] **Step 6.1：写端到端失败测试 — 模拟 disruption 输入应放大 surprise**

新建文件：`bionic_v0/tests/disruption_surprise_amplification.rs`

```rust
//! End-to-end test: disruption-pattern err in VibAdapter should produce
//! amplified surprise compared to baseline (low-err) input under the same
//! priors and same processing path.
//!
//! Strategy: directly inject controlled err into the viewport's predictor
//! buffers, then call process_frame, then read smoothed_surprise via
//! the ambient channel. Compare baseline vs disruption frames.

use bionic_v0::adapter::VibAdapter;
use bionic_v0::evolution_priors::EvolutionPriors;
use bionic_v0::viewport::Viewport;
use bionic_v0::VIB_BANDS;
use std::sync::Arc;

/// Helper: drive a single-band synthetic err pattern through the viewport
/// and adapter, returning the smoothed_surprise observed in ambient.
fn smoothed_surprise_for_pattern(
    bands_pattern: [f64; VIB_BANDS],
    priors: Arc<EvolutionPriors>,
    n_ticks: u32,
) -> f64 {
    let vp = Arc::new(Viewport::new());
    let mut adapter = VibAdapter::with_priors(&vp, priors);

    // Drive n_ticks frames with the same band pattern. The viewport's
    // predictor will adapt and produce err that follows the pattern.
    for tick in 0..n_ticks {
        let t_us = (tick as u64) * 10_000;
        adapter.process_frame_for_test(&vp, t_us, &bands_pattern);
    }

    let ambient = vp.read_ambient();
    ambient.surprise
}

#[test]
fn test_disruption_pattern_amplifies_surprise_vs_baseline() {
    let priors = Arc::new(EvolutionPriors::default());

    // Baseline: low-energy quiet pattern
    let baseline = smoothed_surprise_for_pattern(
        [0.05, 0.05, 0.05, 0.05],
        priors.clone(),
        50,
    );

    // Disruption: distress band (2) dominant
    let disruption = smoothed_surprise_for_pattern(
        [0.05, 0.1, 0.8, 0.1],
        priors.clone(),
        50,
    );

    assert!(
        disruption > baseline * 1.5,
        "disruption pattern should amplify surprise >1.5x: baseline={}, disruption={}",
        baseline, disruption
    );
}

#[test]
fn test_low_disruption_multiplier_means_smaller_amplification() {
    let mut priors_low = EvolutionPriors::default();
    priors_low.disruption_surprise_multiplier = 1.0; // No amplification

    let mut priors_high = EvolutionPriors::default();
    priors_high.disruption_surprise_multiplier = 5.0; // Strong amplification

    let pattern = [0.05, 0.1, 0.8, 0.1];

    let surprise_low = smoothed_surprise_for_pattern(pattern, Arc::new(priors_low), 50);
    let surprise_high = smoothed_surprise_for_pattern(pattern, Arc::new(priors_high), 50);

    assert!(
        surprise_high > surprise_low,
        "higher disruption_surprise_multiplier should produce higher surprise: low={}, high={}",
        surprise_low, surprise_high
    );
}
```

> **注意**：这个测试调用了 `process_frame_for_test`——一个**还不存在的** test-only 方法。Step 6.3 会加它。这是因为 `process_frame` 当前是 `fn`（私有），并且依赖 `cpal::Stream` 等运行时基础设施。我们需要一个绕过 mic 输入的测试入口。

- [ ] **Step 6.2：跑测试确认 FAIL**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test disruption_surprise_amplification 2>&1 | tail -15
```

期望：FAIL，"no method named `process_frame_for_test`"。

- [ ] **Step 6.3：在 `VibAdapter` 加 test-only 入口 + 修改 `process_frame` 应用 priors**

(a) 把 `process_frame` 现有 surprise 累加段（`src/adapter.rs:225-234`）改写为：

```rust
        let inst_energy: f64 = bands.iter().map(|b| b * b).sum();

        // v2.1 Layer 0: collect per-band err, apply disruption sensitivity
        let mut per_band_err = [0.0f64; VIB_BANDS];
        for axis in 0..VIB_BANDS {
            if let Some(e) = vp.latest_err(axis) {
                per_band_err[axis] = e;
            }
        }

        // Sensitivity-weighted base surprise (uses disruption_band_sensitivity
        // as a multiplier — for v0 single tuning knob shared with disruption
        // signature; revisit if a separate "always-on band weight" is needed)
        let mut inst_surprise = 0.0;
        for axis in 0..VIB_BANDS {
            inst_surprise += per_band_err[axis].abs()
                * self.priors.disruption_band_sensitivity[axis];
        }

        // Disruption signature check: if score crosses threshold, amplify
        let disruption_score = crate::evolution_priors::compute_disruption_score(
            &per_band_err, &self.priors,
        );
        if disruption_score > self.priors.disruption_err_threshold {
            inst_surprise *= self.priors.disruption_surprise_multiplier;
        }

        let alpha = 0.1;
        self.smoothed_energy = (1.0 - alpha) * self.smoothed_energy + alpha * inst_energy;
        self.smoothed_surprise = (1.0 - alpha) * self.smoothed_surprise + alpha * inst_surprise;
```

(b) 在 `impl VibAdapter` 内部加 test-only 入口（位置：与现有 `priors()` 方法相邻）：

```rust
    /// Test-only entry point for process_frame that bypasses the mic stream.
    /// Exposes the same logic to integration tests.
    #[cfg(any(test, debug_assertions))]
    pub fn process_frame_for_test(
        &mut self,
        vp: &Arc<Viewport>,
        t_us: u64,
        bands: &[f64; VIB_BANDS],
    ) {
        self.process_frame(vp, t_us, bands);
    }
```

> **注意**：`process_frame` 当前是私有方法。`process_frame_for_test` 在**同一个 `impl` 块**里调用它，所以可见性 OK，不需要加 pub。如果你把它移到别的 impl 块或别的文件，记得改可见性。

- [ ] **Step 6.4：跑端到端测试**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test disruption_surprise_amplification 2>&1 | tail -15
```

期望：两个测试 PASS。如果 surprise 数值不符合断言（例如 disruption pattern 没产生 1.5x 放大），**先打印实际值再调整**：

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test --test disruption_surprise_amplification -- --nocapture 2>&1 | tail -20
```

可能需要调整：
- `disruption_err_threshold` 默认值（可能 0.3 太高或太低）
- 测试中的 `n_ticks`（让 smoothing 收敛到稳态）
- 或者测试断言的倍数（1.5x 可能太严格，可调到 1.2x）

调整原则：**优先调测试断言匹配实际行为；只有当实际行为明显违反 spec §3 时才调默认值。** 调整后回到 Step 6.4 重跑。

- [ ] **Step 6.5：跑全测试套件，确认无回归**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test 2>&1 | tail -20
```

**重要：可能会有现有 adapter 相关测试因为 surprise 行为变了而 FAIL**。这是预期的——新默认值改变了 surprise 计算。

如果现有测试失败：
- 先看失败的测试断言什么；如果是 "smoothed_surprise == X"，把它改为新的实际值
- 如果是行为类断言（"surprise should increase when X"），通常仍应通过
- 任何修改都应该在 commit message 里说明：「surprise baseline shifted due to Layer 0 priors integration; expected values updated」

如果某个回归你**确定不应该发生**（例如 surprise 在静默输入下也变了），停下来检查 priors 默认值是否需要调整。

- [ ] **Step 6.6：提交**

```bash
git add bionic_v0/src/adapter.rs bionic_v0/tests/disruption_surprise_amplification.rs
# 如果 Step 6.5 修了任何现有测试，把它们也 add 进来
git commit -m "feat(adapter): wire EvolutionPriors disruption sensitivity into VIB surprise

VibAdapter::process_frame now reads from self.priors:
1. Per-band err is weighted by disruption_band_sensitivity before summing
2. compute_disruption_score is checked against disruption_err_threshold;
   on overshoot, surprise is multiplied by disruption_surprise_multiplier

End-to-end test (tests/disruption_surprise_amplification.rs) verifies
distress-band-dominant patterns produce >1.5x amplified smoothed_surprise
vs baseline quiet input.

Spec §3 + §4.4: 'AI must be sensitive to physical-harm perception
patterns'. This is the v0 实现 of that for VIB modality. PHOT modality
remains TODO (separate plan)."
```

---

## Task 7：写文档脚注，记录已落地范围 + 后续触发点

**目的：** 让未来读 spec §3 的人立刻知道哪些已经做了、哪些还在 TODO。

**Files:**
- Modify: `docs/superpowers/specs/2026-04-25-shenxing-design.md`（在 §3 末尾加实现状态脚注）

- [ ] **Step 7.1：在 §3 末尾加实现状态章节**

定位到 `## 4. 元原则：补充人类缺陷，而非模仿人类` 之前的位置（即 §3 末尾），加：

```markdown
### 3.5 实现状态（更新于 2026-04-26）

| 子项 | 状态 | 实现位置 |
|---|---|---|
| EvolutionPriors 接入 crate | ✅ 已落地 | `bionic_v0/src/lib.rs` |
| disruption-sensitivity 数据字段（disruption_band / threshold / multiplier / distress_acoustic）| ✅ 已落地 | `bionic_v0/src/evolution_priors.rs` |
| ModifyRateCaps（lock-weight 机制）+ clamp_modification API | ✅ 已落地 | `bionic_v0/src/evolution_priors.rs` |
| compute_disruption_score 纯函数 | ✅ 已落地 | `bionic_v0/src/evolution_priors.rs` |
| VibAdapter 持有 EvolutionPriors 并应用到 surprise 计算 | ✅ 已落地 | `bionic_v0/src/adapter.rs` |
| 端到端测试：disruption pattern → 放大的 surprise | ✅ 已落地 | `bionic_v0/tests/disruption_surprise_amplification.rs` |
| **PHOT（视觉）通道 disruption 信号** | ❌ TODO | 后续 plan |
| **precision 路径整合（让 AI 不仅惊吓，还专注盯死破坏）**| ❌ TODO（方案 3 范围）| 后续 plan |
| **学习路径实际调用 clamp_modification** | ❌ TODO（API 已铺，无 caller）| 触发：任何引入 EvolutionPriors mutation 的 PR |

参考：`docs/superpowers/plans/2026-04-26-layer0-dna-bias-surprise-wiring.md`
```

- [ ] **Step 7.2：提交文档更新**

```bash
git add docs/superpowers/specs/2026-04-25-shenxing-design.md
git commit -m "docs(shenxing): add §3.5 implementation status — Layer 0 VIB wiring landed

Records what plan 2026-04-26-layer0-dna-bias-surprise-wiring delivered:
- EvolutionPriors re-integrated into crate
- disruption sensitivity fields + ModifyRateCaps API + compute_disruption_score
- VibAdapter applies priors to surprise calculation
- End-to-end test verifies amplification

Explicitly TODOs:
- PHOT modality disruption wiring
- precision path integration (方案 3 范围)
- Learning path callers for clamp_modification"
```

---

## Task 8：最终验收

- [ ] **Step 8.1：跑完整测试套件**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo test 2>&1 | tail -20
```

期望：全部 PASS。记录新增测试数（Task 2-6 共加了大约 9 个新测试）。

- [ ] **Step 8.2：构建所有 binary，确认下游没破**

```bash
cd /Users/joe/Documents/unnamed/bionic_v0 && cargo build --bins 2>&1 | tail -10
```

期望：body / observer / observer_surface 三个 binary 都编译成功。

- [ ] **Step 8.3：跑 git log 看本 plan 的提交链**

```bash
cd /Users/joe/Documents/unnamed && git log --oneline | head -10
```

期望：能看到 7 个本 plan 的 commit（Task 1-7 各一个），加 Task 6 可能的额外 fixup commit。

- [ ] **Step 8.4：在本 plan 文档末尾加完工章节**

在文件末尾追加：

```markdown
---

## 完工记录

- **完工日期**：[填写]
- **新增测试数**：[填写]
- **影响行数**：参见 commit `<起始 sha>..<终止 sha>` 的 git diff --stat
- **未触及但相关的事**：
  - PHOT modality 的 disruption 信号（视觉破坏 / 不对称破坏检测）→ 单独 plan
  - precision 路径整合（让 disruption 不仅放大 surprise，也提高 attention）→ 方案 3 plan
  - intention.rs 的 orphan 状态 → 后续 plan
  - Layer 0 偏置在学习路径中实际被 clamp_modification 守护 → 当任何 mutation 路径引入时触发
```

提交：

```bash
git add docs/superpowers/plans/2026-04-26-layer0-dna-bias-surprise-wiring.md
git commit -m "docs(plan): mark layer0-dna-bias-surprise-wiring complete"
```

---

## 设计决策备忘（不在任务里，给执行者参考）

1. **为什么 disruption_band_sensitivity 默认 [0.5, 1.0, 1.4, 0.8]？**
   - Band 0（<500Hz）= 低频背景声音，权重最低
   - Band 1（500-2000Hz）= 人声范围，正常权重
   - Band 2（2000-5000Hz）= 痛苦/尖叫范围，权重最高
   - Band 3（>5000Hz）= 高频环境噪声，中等权重
   - 这与 spec §3.2 "对痛苦信号敏感"一致

2. **为什么 lock-weight 用"修改速率上限"而不是"完全锁定"？**
   - spec §3.1 原话："修改权重锁到极低"——意思是低，不是零
   - 完全锁定会让任何未来学习路径必须走特殊 if/else，违反 §10 "偏置优先于规则"
   - 用 cap 机制：API 一致，未来学习路径只需 clamp 即可，无需感知 Layer 0 概念

3. **为什么不动 intention.rs 的 orphan？**
   - 本 plan scope 严格限定在 Layer 0；intention 属于 Layer 1（内驱引擎）
   - 一次 plan 处理多件事会让 commit 链条复杂、回滚困难
   - intention 重接入应另开 plan

4. **为什么 process_frame 改动那么"机械"——只是乘法，没有任何 if 复杂逻辑？**
   - spec §10 公约 1：「不写规则文档让 AI 查阅」
   - 偏置应该是数据驱动的（乘以一个权重），不应该是 if/else 规则
   - 这一改动是直接用乘数表达"对破坏更敏感"，没有任何"if disruption then 特殊 path"

5. **如果 Task 6.4 的端到端测试一直 FAIL 怎么办？**
   - 先用 `--nocapture` 看实际数值
   - 检查是否 viewport 的 predictor 在 50 ticks 后还没收敛（可以试 100 / 200 ticks）
   - 检查 `latest_err` 在测试场景下是否真的有非零值（可能需要预先 push 一些 baseline 观测）
   - 极端情况：如果 viewport predictor 行为不可控，可以重写测试为"用 mock viewport，直接调用 compute_disruption_score 验证乘数关系"——退化为单测，不再是端到端，但仍能验证核心契约

---

## 完工记录

- **完工日期**：2026-04-26
- **新增测试数**：10（evolution_priors: 8 新测试；vib_adapter_priors: 2 测试；disruption_surprise_amplification: 2 测试；共 12 新增断言，其中 evolution_priors 原有 3 测试保留）
- **总测试数**：257 passed, 1 failed（`vocal_body_contract` — pre-existing failure, unrelated to this plan）
- **影响行数**：+564 / -2（6 files）
- **Commit SHA 范围**：`d04ba14..2e4a1ec`（7 commits）
  - `40c2ab7` Task 1: re-integrate evolution_priors into crate
  - `a953797` Task 2: add disruption sensitivity fields
  - `391254b` Task 3: add lock-weight (modify_rate_caps) mechanism
  - `9143f4f` Task 4: add compute_disruption_score pure function
  - `2047203` Task 5: VibAdapter holds Arc<EvolutionPriors>
  - `4a426a1` Task 6: wire disruption sensitivity into VIB surprise
  - `2e4a1ec` Task 7: add §3.5 implementation status
- **未触及但相关的事**：
  - PHOT modality 的 disruption 信号（视觉破坏 / 不对称破坏检测）→ 单独 plan
  - precision 路径整合（让 AI 不仅惊吓，还专注盯死破坏）→ 方案 3 plan
  - intention.rs 的 orphan 状态 → 后续 plan
  - Layer 0 偏置在学习路径中实际被 clamp_modification 守护 → 当任何 mutation 路径引入时触发
  - `vocal_body_contract` 测试失败（pre-existing, verified against base SHA d04ba14）
