# Language Binding Runtime Path

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 language_binding substrate 接入 live body 运行路径，使外部图案检测能实时生成 binding event 并更新 binding trace。

**Architecture:** 在 adapter 层从 VIB/PHOT 数据中提取 pattern hash，生成 BindingEvent 并通过 Viewport API 写入 binding trace。新增低频 ticker 定期刷新。binding 只读对象层，不写对象层。

**Tech Stack:** Rust, existing Viewport/language_binding APIs, Cargo integration tests.

---

## File Structure

**Create:**
- `bionic_v0/tests/language_binding_runtime.rs`

**Modify:**
- `bionic_v0/src/language_binding.rs` — 新增 Viewport impl methods
- `bionic_v0/src/viewport.rs` — 挂载 language_binding state (如尚未挂载)
- `bionic_v0/src/adapter.rs` — VIB/PHOT adapter 中生成 pattern events
- `bionic_v0/src/bin/body.rs` — 新增 binding refresh ticker
- `bionic_v0/src/snapshot.rs` — 序列化 language binding state (如未实现)
- `bionic_v0/src/lib.rs` — 新增 binding 相关常量

---

## Chunk 1: Viewport API for Language Binding

### Task 1: Write failing tests

**Files:**
- Create: `bionic_v0/tests/language_binding_runtime.rs`

- [ ] **Step 1: Binding event updates trace**

```rust
#[test]
fn binding_event_creates_and_updates_trace() {
    let vp = Arc::new(Viewport::new());

    let event = BindingEvent {
        pattern_kind: ExternalPatternKind::AudiblePattern,
        pattern_hash: 0xDEAD_BEEF,
        target: BindingTargetRef::ObjectSlot { slot_id: 0 },
        feedback: BindingFeedback::Support,
        evidence_ref: 1,
        event_t_us: 1_000,
    };

    vp.apply_binding_event_from_adapter(event);

    let st = vp.read_language_binding();
    assert_eq!(st.n_bindings, 1);
    assert_eq!(st.traces[0].pattern_hash, 0xDEAD_BEEF);
    assert_eq!(st.traces[0].status, BindingStatus::Candidate);
}
```

- [ ] **Step 2: Repeated support reinforces**

```rust
#[test]
fn repeated_support_reinforces_binding() {
    let vp = Arc::new(Viewport::new());

    let make_event = |t: u64| BindingEvent {
        pattern_kind: ExternalPatternKind::AudiblePattern,
        pattern_hash: 0xDEAD_BEEF,
        target: BindingTargetRef::ObjectSlot { slot_id: 0 },
        feedback: BindingFeedback::Support,
        evidence_ref: t,
        event_t_us: t,
    };

    vp.apply_binding_event_from_adapter(make_event(1_000));
    vp.apply_binding_event_from_adapter(make_event(2_000));
    vp.apply_binding_event_from_adapter(make_event(3_000));

    let st = vp.read_language_binding();
    assert_eq!(st.n_bindings, 1);
    assert_eq!(st.traces[0].status, BindingStatus::Reinforced);
    assert!(st.traces[0].strength.value > 0.5);
}
```

- [ ] **Step 3: Contradict weakens**

```rust
#[test]
fn contradict_weakens_binding() {
    let vp = Arc::new(Viewport::new());

    vp.apply_binding_event_from_adapter(BindingEvent {
        pattern_kind: ExternalPatternKind::AudiblePattern,
        pattern_hash: 0xDEAD_BEEF,
        target: BindingTargetRef::ObjectSlot { slot_id: 0 },
        feedback: BindingFeedback::Support,
        evidence_ref: 1,
        event_t_us: 1_000,
    });

    vp.apply_binding_event_from_adapter(BindingEvent {
        pattern_kind: ExternalPatternKind::AudiblePattern,
        pattern_hash: 0xDEAD_BEEF,
        target: BindingTargetRef::ObjectSlot { slot_id: 0 },
        feedback: BindingFeedback::Contradict,
        evidence_ref: 2,
        event_t_us: 2_000,
    });

    let st = vp.read_language_binding();
    assert_eq!(st.traces[0].status, BindingStatus::Weakened);
}
```

### Task 2: Implement Viewport API

**Files:**
- Modify: `bionic_v0/src/language_binding.rs`

- [ ] **Step 1: 在 language_binding.rs 末尾添加 Viewport impl**

```rust
use std::sync::Arc;
use crate::viewport::Viewport;

impl Viewport {
    pub fn apply_binding_event_from_adapter(&self, event: BindingEvent) {
        let mut st = self.language_binding.write().unwrap();
        st.apply_binding_event(event);
    }

    pub fn read_language_binding(&self) -> LanguageBindingState {
        self.language_binding.read().unwrap().clone()
    }
}
```

- [ ] **Step 2: 确认 viewport.rs 中已挂载 `language_binding: RwLock<LanguageBindingState>`**

如未挂载，在 Viewport struct 和 new() 中添加初始化。

- [ ] **Step 3: Run tests → GREEN**

---

## Chunk 2: Pattern Detection from Adapters

### Task 3: VIB → AudiblePattern

**Files:**
- Modify: `bionic_v0/src/vib.rs`

- [ ] **Step 1: 在 VIB adapter 的事件处理路径中，当 band pattern 变化显著时生成 AudiblePattern binding event**

```rust
// 在 VIB event processing 中:
let pattern_hash = compute_vib_pattern_hash(&bands); // simple hash of 4-band values
if pattern_hash != self.last_vib_pattern_hash {
    vp.apply_binding_event_from_adapter(BindingEvent {
        pattern_kind: ExternalPatternKind::AudiblePattern,
        pattern_hash,
        target: BindingTargetRef::ObjectSlot { slot_id: claimed_slot },
        feedback: BindingFeedback::Support,
        evidence_ref: event_t_us,
        event_t_us,
    });
    self.last_vib_pattern_hash = pattern_hash;
}
```

### Task 4: PHOT → VisualGlyph

**Files:**
- Modify: `bionic_v0/src/phot_camera.rs`

- [ ] **Step 1: 在 PHOT motion region 处理中，当 region pattern 变化时生成 VisualGlyph binding event**

类似 VIB 模式，用 region 的空间分布计算 hash。

---

## Chunk 3: Body Integration + Snapshot

### Task 5: Binding refresh ticker

**Files:**
- Modify: `bionic_v0/src/bin/body.rs`

- [ ] **Step 1: 新增 5 Hz ticker 定期刷新 binding state**

```rust
let vp_bind = vp.clone();
let h_binding = spawn_ticker(5, stop.clone(), move |_| {
    // Periodic binding state maintenance (decay weak bindings, etc.)
    vp_bind.tick_binding_maintenance(now_us());
});
```

### Task 6: Snapshot serialization

**Files:**
- Modify: `bionic_v0/src/snapshot.rs`

- [ ] **Step 1: 确认 snapshot 已序列化 language_binding state**

检查 `write_snapshot` 和 `read_snapshot` 中是否有 language_binding 的读写。如无，添加序列化逻辑。

- [ ] **Step 2: 如添加新字段，bump SNAPSHOT_VERSION**

---

## Verification Checklist

- [ ] `cargo test` 全量通过
- [ ] body 运行后 snapshot 中 `language_binding.n_bindings >= 1`
- [ ] binding trace 只绑定到已有 slot，不创建新对象
- [ ] 无 eprintln!/println! 引入 body 代码
- [ ] 无 concept/semantic/target 新字段
