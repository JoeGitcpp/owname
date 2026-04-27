# Close Action-Consequence Loop in Live Body

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将跨模态 ingress 变化接入 action_consequence substrate，使 consequence window 不再仅由 PHOT 路径创建，而是由所有模态的感知变化共同驱动，形成完整的 write → consequence → pair 闭环。

**Architecture:** 在 adapter 层接入 `accumulate_ingress_change`，在 viewport 新增统一 ingress accumulation ticker，替换现有零散的 consequence window 创建路径。

**Tech Stack:** Rust, existing Viewport/action_consequence/adapter APIs, Cargo integration tests.

---

## File Structure

**Create:**
- `bionic_v0/tests/ingress_consequence_loop.rs`

**Modify:**
- `bionic_v0/src/adapter.rs` — VIB/PHOT adapter 中接入 consequence window
- `bionic_v0/src/vib.rs` — VIB 事件能量 → accumulate_ingress_change
- `bionic_v0/src/phot_camera.rs` — 改用 begin/extend/close 生命周期 API
- `bionic_v0/src/action_consequence.rs` — 新增 `tick_all_ingress_accumulation`
- `bionic_v0/src/lib.rs` — 新增 `INGRESS_ACCUMULATION_THRESHOLD` 常量
- `bionic_v0/src/bin/body.rs` — 新增统一 ingress accumulation ticker

---

## Chunk 1: VIB Ingress → Consequence Window

### Task 1: Write failing tests

**Files:**
- Create: `bionic_v0/tests/ingress_consequence_loop.rs`

- [ ] **Step 1: VIB 高能量事件打开 consequence window**

```rust
#[test]
fn vib_high_energy_opens_consequence_window() {
    let vp = Arc::new(Viewport::new());
    // 直接写入高能量 VIB 数据
    let mut vib = vp.vib_state.write().unwrap();
    vib.bands[0] = 0.8;
    vib.bands[1] = 0.6;
    vib.energy = 0.5;
    vib.last_event_t_us = 1_000;

    // 触发 VIB ingress accumulation
    tick_vib_ingress_consequence(&vp, 1_000);

    let st = vp.read_action_consequence();
    assert!(st.n_windows >= 1, "VIB high energy must open a consequence window");
}
```

- [ ] **Step 2: VIB 低能量不打开 window**

```rust
#[test]
fn vib_low_energy_no_consequence_window() {
    let vp = Arc::new(Viewport::new());
    let mut vib = vp.vib_state.write().unwrap();
    vib.energy = 0.0001; // well below threshold
    vib.last_event_t_us = 1_000;

    tick_vib_ingress_consequence(&vp, 1_000);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_windows, 0, "VIB low energy must not open a window");
}
```

- [ ] **Step 3: Run tests → RED**

### Task 2: Implement VIB ingress consequence

**Files:**
- Modify: `bionic_v0/src/action_consequence.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Add constant to lib.rs**

```rust
/// Engineering approximation: VIB event energy above this triggers
/// consequence window accumulation. Substrate knob, not theoretical bound.
pub const INGRESS_ACCUMULATION_THRESHOLD: f64 = 0.01;
```

- [ ] **Step 2: Add `tick_vib_ingress_consequence` to action_consequence.rs**

```rust
pub fn tick_vib_ingress_consequence(vp: &Arc<Viewport>, t: u64) {
    let vib = vp.vib_state.read().unwrap();
    if vib.energy > INGRESS_ACCUMULATION_THRESHOLD {
        vp.accumulate_ingress_change(t, vib.energy.min(1.0));
    }
}
```

- [ ] **Step 3: Run tests → GREEN**

---

## Chunk 2: PHOT Lifecycle Migration

### Task 3: Migrate PHOT to begin/extend/close API

**Files:**
- Modify: `bionic_v0/src/phot_camera.rs`
- Modify: `bionic_v0/tests/real_phot_adapter.rs` (update assertions)

- [ ] **Step 1: 在 phot_camera.rs 中，将 `push_action_consequence_window` 替换为 begin/extend/close 模式**

当前 PHOT 路径直接调用 `push_action_consequence_window`，改为：
- 首次检测到 motion → `begin_action_consequence_window`
- 持续 motion → `extend_action_consequence_window`
- motion quiescence → `close_action_consequence_window`

- [ ] **Step 2: 运行现有 PHOT 测试确认无回归**

---

## Chunk 3: Unified Ingress Accumulation Ticker

### Task 4: Add unified ticker

**Files:**
- Modify: `bionic_v0/src/action_consequence.rs`
- Modify: `bionic_v0/src/bin/body.rs`

- [ ] **Step 1: 新增 `tick_all_ingress_accumulation`**

```rust
pub fn tick_all_ingress_accumulation(vp: &Arc<Viewport>, now_t_us: u64, quiescence_us: u64) {
    // Close quiescent windows across all modalities
    vp.tick_ingress_accumulation(now_t_us, quiescence_us);
    // Attempt auto-pairing for any newly closed windows
    vp.try_auto_pair_latest_epoch_to_window(now_t_us);
}
```

- [ ] **Step 2: 在 body.rs 新增 10 Hz ticker 调用此函数**

```rust
let vp_ingress = vp.clone();
let h_ingress = spawn_ticker(10, stop.clone(), move |_| {
    tick_all_ingress_accumulation(&vp_ingress, now_us(), PHOT_CONSEQUENCE_QUIESCENCE_US);
});
```

- [ ] **Step 3: 全量测试验证**

---

## Verification Checklist

- [ ] `cargo test` 全量通过
- [ ] body 运行 5 秒后 snapshot 中 `action_consequence.n_windows >= 1` 且 `n_pairs >= 1`
- [ ] VIB 和 PHOT 路径都能独立打开 consequence window
- [ ] 无 eprintln!/println! 引入 body 代码
- [ ] 无 target/meaning/semantic 新字段
