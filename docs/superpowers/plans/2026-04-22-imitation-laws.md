# Imitation Laws Implementation

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将模仿定律 (§5 of spec 2026-04-21) 物理化到已有 substrate 上：write-consequence curve similarity, binding-to-object resonance, delay-distribution feedback。不引入"模仿模块"。

**Architecture:** 三个机制全部映射到已有 substrate：pair 新增 curve_similarity 字段, binding strength → attention bias, delay distribution → probe timing adjustment。

**Tech Stack:** Rust, existing Viewport/action_consequence/language_binding/probe APIs, Cargo integration tests.

**Spec:** `docs/superpowers/specs/2026-04-21-action-consequence-loop-and-imitation-laws-design.md`

---

## File Structure

**Create:**
- `bionic_v0/tests/imitation_laws.rs`

**Modify:**
- `bionic_v0/src/action_consequence.rs` — curve similarity on pairs
- `bionic_v0/src/language_binding.rs` — binding → attention bias bridge
- `bionic_v0/src/probe.rs` — delay-distribution feedback to probe timing
- `bionic_v0/src/lib.rs` — imitation law constants
- `bionic_v0/src/snapshot.rs` — serialize new pair field

---

## Chunk 1: Write-Consequence Curve Similarity

### Task 1: Write failing tests

**Files:**
- Create: `bionic_v0/tests/imitation_laws.rs`

- [ ] **Step 1: 相似曲线产生高 similarity**

```rust
#[test]
fn similar_write_and_consequence_curves_yield_high_similarity() {
    let vp = Arc::new(Viewport::new());

    // Write epoch with magnitude 0.5
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.5,
        event_t_us: 1_000,
    });

    // Consequence window with similar magnitude 0.48
    vp.accumulate_ingress_change(1_500, 0.48);
    vp.try_auto_pair_latest_epoch_to_window(1_500);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1);
    // similarity should be high (magnitudes are close)
    assert!(
        st.pairs[0].curve_similarity > 0.8,
        "Similar magnitudes must yield high curve_similarity"
    );
}
```

- [ ] **Step 2: 不同曲线产生低 similarity**

```rust
#[test]
fn dissimilar_curves_yield_low_similarity() {
    let vp = Arc::new(Viewport::new());

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.9,
        event_t_us: 1_000,
    });

    vp.accumulate_ingress_change(1_500, 0.1);
    vp.try_auto_pair_latest_epoch_to_window(1_500);

    let st = vp.read_action_consequence();
    assert!(
        st.pairs[0].curve_similarity < 0.3,
        "Dissimilar magnitudes must yield low curve_similarity"
    );
}
```

### Task 2: Implement

**Files:**
- Modify: `bionic_v0/src/action_consequence.rs`

- [ ] **Step 1: 在 ActionConsequencePair 新增 curve_similarity 字段**

```rust
pub struct ActionConsequencePair {
    pub write_seq: u64,
    pub window_id: u64,
    pub delay_us: u64,
    pub status: ConsequencePairStatus,
    pub pair_t_us: u64,
    /// Numeric similarity between write epoch magnitude and
    /// consequence window magnitude. [0,1]. Engineering approximation
    /// of curve alignment — not success/failure semantics.
    pub curve_similarity: f64,
}
```

- [ ] **Step 2: 在 `try_auto_pair_latest_epoch_to_window` 和 `pair_action_consequence` 中计算 similarity**

```rust
// In try_auto_pair_latest_epoch_to_window, when recording the pair:
let write_mag = {
    let ao = self.action_output.read().unwrap();
    ao.write_epoch_history.iter()
        .find(|r| r.write_seq == write_seq)
        .map(|r| r.peak_magnitude)
        .unwrap_or(0.0)
};
let consequence_mag = window.magnitude;
let curve_similarity = 1.0 - (write_mag - consequence_mag).abs();

ac.pairs.push(ActionConsequencePair {
    write_seq,
    window_id,
    delay_us,
    status: ConsequencePairStatus::Paired,
    pair_t_us,
    curve_similarity: curve_similarity.clamp(0.0, 1.0),
});
```

- [ ] **Step 3: 更新 snapshot 序列化 (bump SNAPSHOT_VERSION)**

---

## Chunk 2: Binding-to-Object Resonance

### Task 3: Write failing tests

**Files:**
- Modify: `bionic_v0/tests/imitation_laws.rs`

- [ ] **Step 1: 高 binding strength 注入 attention bias**

```rust
#[test]
fn strong_binding_injects_attention_bias() {
    let vp = Arc::new(Viewport::new());

    // Create a strong binding to slot 0
    for i in 0..5 {
        vp.apply_binding_event_from_adapter(BindingEvent {
            pattern_kind: ExternalPatternKind::AudiblePattern,
            pattern_hash: 0xBEEF,
            target: BindingTargetRef::ObjectSlot { slot_id: 0 },
            feedback: BindingFeedback::Support,
            evidence_ref: i as u64,
            event_t_us: 1_000 + i as u64 * 1_000,
        });
    }

    // Run binding resonance tick
    vp.tick_binding_resonance(10_000);

    let attn = vp.read_global_attention();
    // Slot 0 should have received an attention bias
    assert!(
        attn.slot_biases[0] > 0.0,
        "Strong binding must inject positive attention bias"
    );
}
```

### Task 4: Implement

**Files:**
- Modify: `bionic_v0/src/language_binding.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Add constants**

```rust
/// Minimum binding strength to trigger attention bias injection.
pub const BINDING_RESONANCE_THRESHOLD: f64 = 0.5;

/// Alpha for binding → attention bias injection.
pub const BINDING_ATTN_ALPHA: f64 = 0.05;
```

- [ ] **Step 2: Add `tick_binding_resonance`**

```rust
pub fn tick_binding_resonance(&self, now_us: u64) {
    let bindings = self.language_binding.read().unwrap();
    for trace in bindings.traces.iter() {
        if trace.strength.value < BINDING_RESONANCE_THRESHOLD {
            continue;
        }
        if let BindingTargetRef::ObjectSlot { slot_id } = trace.target {
            let slot_id = slot_id as usize;
            let mut attn = self.global_attention.write().unwrap();
            if slot_id < attn.slot_biases.len() {
                let bias = trace.strength.value * BINDING_ATTN_ALPHA;
                attn.slot_biases[slot_id] = (attn.slot_biases[slot_id] + bias)
                    .min(ATTN_PRECISION_BIAS_MAX);
            }
        }
    }
}
```

- [ ] **Step 3: 在 body.rs 新增 3 Hz ticker**

---

## Chunk 3: Delay-Distribution Feedback

### Task 5: Write failing tests

**Files:**
- Modify: `bionic_v0/tests/imitation_laws.rs`

- [ ] **Step 1: 典型延迟影响 probe timing**

```rust
#[test]
fn typical_delay_from_pairs_adjusts_probe_timing() {
    let vp = Arc::new(Viewport::new());

    // Create several pairs with similar delays (~500ms)
    for i in 0..5 {
        let t_epoch = i as u64 * 2_000_000;
        vp.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Begin,
            magnitude: 0.3,
            event_t_us: t_epoch,
        });
        vp.accumulate_ingress_change(t_epoch + 500_000, 0.3);
        vp.try_auto_pair_latest_epoch_to_window(t_epoch + 500_000);
    }

    // Run delay distribution feedback
    let typical = vp.compute_typical_consequence_delay();
    // Should converge around 500ms
    assert!(
        typical > 400_000 && typical < 600_000,
        "Typical delay should be around 500ms, got {}", typical
    );
}
```

### Task 6: Implement

**Files:**
- Modify: `bionic_v0/src/probe.rs`
- Modify: `bionic_v0/src/action_consequence.rs`

- [ ] **Step 1: Add `compute_typical_consequence_delay` to Viewport**

```rust
pub fn compute_typical_consequence_delay(&self) -> u64 {
    let ac = self.action_consequence.read().unwrap();
    if ac.pairs.is_empty() { return 0; }

    // Simple median of recent pair delays (last 16)
    let recent: Vec<u64> = ac.pairs.iter()
        .rev()
        .take(16)
        .map(|p| p.delay_us)
        .collect();

    if recent.is_empty() { return 0; }

    let mut sorted = recent.clone();
    sorted.sort();
    sorted[sorted.len() / 2]
}
```

- [ ] **Step 2: 在 probe.rs 的 tick_probe_output 中使用典型延迟调整 probe 间隔**

```rust
// In tick_probe_output, after computing the event:
let typical_delay = vp.compute_typical_consequence_delay();
if typical_delay > 0 {
    // Use typical delay as a timing reference for next probe
    // This creates resonance: the system probes at the rhythm
    // that has historically produced consequences.
    // Engineering approximation — not a fixed timing law.
    let delay_factor = (typical_delay as f64 / 1_000_000.0).clamp(0.1, 5.0);
    // Adjust magnitude by delay factor
    mag *= (1.0 / delay_factor);
}
```

---

## Verification Checklist

- [ ] `cargo test` 全量通过
- [ ] curve_similarity 在 pair 中正确计算
- [ ] 高 binding strength 产生 attention bias
- [ ] 典型延迟从 pair 分布中提取
- [ ] 无 eprintln!/println! 引入 body 代码
- [ ] 无 imitation/模仿/success 语义字段
- [ ] SNAPSHOT_VERSION 正确 bump
