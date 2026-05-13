# Pattern Expression Binding Events Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let real substrate evidence form language binding events from external pattern expression traces and existing object/relation/time targets, without allowing adapters to fabricate feedback or reintroducing naming ontology.

**Architecture:** Keep `language_binding.rs` as a trace-only binding substrate. Add a small Viewport bridge that converts an existing action-consequence pair into a `BindingEvent` only when the referenced target is live and the pair carries strong curve evidence. Ambiguous or missing pair evidence produces no event, preserving uncertainty instead of inventing a name or concept.

**Tech Stack:** Rust, existing `Viewport`, `language_binding`, `action_output`, `action_consequence`, Cargo integration tests.

---

## File Structure

- Modify: `bionic_v0/src/language_binding.rs`
  - Owns the substrate-only binding-event formation bridge from existing pair evidence.
- Modify: `bionic_v0/src/lib.rs`
  - Adds engineering thresholds for pair evidence classification with boundary comments.
- Modify: `bionic_v0/tests/language_binding_runtime.rs`
  - Adds TDD coverage for pair-gated event formation, live target gating, ambiguity preservation, and feedback weakening.

## Chunk 1: Pair-Gated Binding Event Formation

### Task 1: Missing Pair Evidence Must Not Create A Binding

**Files:**
- Modify: `bionic_v0/tests/language_binding_runtime.rs`
- Modify: `bionic_v0/src/language_binding.rs`

- [ ] **Step 1: Write the failing test**

Add a test proving the new bridge API exists and refuses to form a binding from a missing pair:

```rust
#[test]
fn missing_pair_evidence_does_not_form_binding_event() {
    let vp = Arc::new(Viewport::new());

    let formed = vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xCAFE,
        BindingTargetRef::ObjectSlot { slot_id: 0 },
        1,
        1,
    );

    assert!(!formed);
    assert_eq!(vp.read_language_binding().n_bindings, 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test language_binding_runtime missing_pair_evidence_does_not_form_binding_event -- --nocapture
```

Expected: FAIL because `try_form_binding_event_from_pair` does not exist yet.

- [ ] **Step 3: Write minimal implementation**

In `language_binding.rs`, add an `impl Viewport` method that returns `false` when it cannot find a matching paired `(write_seq, window_id)` in `action_consequence`.

- [ ] **Step 4: Run test to verify it passes**

Run the same focused test. Expected: PASS.

### Task 2: Strong Pair Evidence Creates A Candidate Binding For A Live Target

**Files:**
- Modify: `bionic_v0/tests/language_binding_runtime.rs`
- Modify: `bionic_v0/src/language_binding.rs`
- Modify: `bionic_v0/src/lib.rs`

- [ ] **Step 1: Write the failing test**

Add helpers to allocate a live slot and produce a high-similarity pair, then assert a candidate trace forms:

```rust
fn claim_vib_slot(vp: &Arc<Viewport>, t: u64) -> usize {
    let ev = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t,
        event_energy: 0.5,
        surprise: 0.2,
        spatial: None,
        nomination_seq_hint: 1,
    };
    vp.claim_slot_for_event(&ev).slot_id().expect("slot")
}

fn paired_write_and_window(vp: &Arc<Viewport>, t: u64, write_mag: f64, window_mag: f64) -> (u64, u64) {
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: write_mag,
        event_t_us: t,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    let window_id = vp.accumulate_ingress_change(t + 100, window_mag);
    assert!(vp.try_auto_pair_latest_epoch_to_window(t + 100));
    (write_seq, window_id)
}

#[test]
fn strong_pair_evidence_forms_candidate_binding_for_live_slot() {
    let vp = Arc::new(Viewport::new());
    let slot_id = claim_vib_slot(&vp, 1_000);
    let (write_seq, window_id) = paired_write_and_window(&vp, 2_000, 0.7, 0.68);

    let formed = vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xA11CE,
        BindingTargetRef::ObjectSlot { slot_id: slot_id as u8 },
        write_seq,
        window_id,
    );

    let bindings = vp.read_language_binding();
    assert!(formed);
    assert_eq!(bindings.n_bindings, 1);
    assert_eq!(bindings.traces[0].status, BindingStatus::Candidate);
    assert_eq!(bindings.traces[0].pattern_hash, 0xA11CE);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test language_binding_runtime strong_pair_evidence_forms_candidate_binding_for_live_slot -- --nocapture
```

Expected: FAIL because the bridge still only returns `false`.

- [ ] **Step 3: Write minimal implementation**

Add `BINDING_PAIR_ALIGN_MIN` and `BINDING_PAIR_DEVIATE_MAX` constants in `lib.rs` with comments stating they are engineering approximations over curve evidence, not semantic thresholds. In the bridge, only create `BindingFeedback::Align` when `curve_similarity >= BINDING_PAIR_ALIGN_MIN` and the target is live.

- [ ] **Step 4: Run focused test**

Run the same focused test. Expected: PASS.

### Task 3: Inactive Target And Ambiguous Pair Evidence Preserve Uncertainty

**Files:**
- Modify: `bionic_v0/tests/language_binding_runtime.rs`
- Modify: `bionic_v0/src/language_binding.rs`

- [ ] **Step 1: Write failing tests**

Add two tests:

```rust
#[test]
fn pair_evidence_does_not_bind_inactive_slot_target() {
    let vp = Arc::new(Viewport::new());
    let (write_seq, window_id) = paired_write_and_window(&vp, 2_000, 0.7, 0.68);

    let formed = vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xBEEF,
        BindingTargetRef::ObjectSlot { slot_id: 0 },
        write_seq,
        window_id,
    );

    assert!(!formed);
    assert_eq!(vp.read_language_binding().n_bindings, 0);
}

#[test]
fn ambiguous_pair_similarity_does_not_force_binding_feedback() {
    let vp = Arc::new(Viewport::new());
    let slot_id = claim_vib_slot(&vp, 1_000);
    let (write_seq, window_id) = paired_write_and_window(&vp, 2_000, 0.6, 0.25);

    let formed = vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xC0FFEE,
        BindingTargetRef::ObjectSlot { slot_id: slot_id as u8 },
        write_seq,
        window_id,
    );

    assert!(!formed);
    assert_eq!(vp.read_language_binding().n_bindings, 0);
}
```

- [ ] **Step 2: Run tests to verify at least one fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test language_binding_runtime pair_evidence_does_not_bind_inactive_slot_target ambiguous_pair_similarity_does_not_force_binding_feedback -- --nocapture
```

Expected: FAIL until target liveness and ambiguity gating exist.

- [ ] **Step 3: Write minimal implementation**

Add target liveness checks:

- `ObjectSlot` target is live only when the slot id exists and `read_slot(slot_id).active` is true.
- `RelationEdge` target is live only when the edge id exists and the edge is active.
- `TimeWindow` target is live when `start_t_us < end_t_us`.

Return `false` for mid-band similarity between `BINDING_PAIR_DEVIATE_MAX` and `BINDING_PAIR_ALIGN_MIN`.

- [ ] **Step 4: Run focused tests**

Run the same focused test command. Expected: PASS.

### Task 4: Low Pair Evidence Weakens Existing Binding Instead Of Creating Ontology

**Files:**
- Modify: `bionic_v0/tests/language_binding_runtime.rs`
- Modify: `bionic_v0/src/language_binding.rs`

- [ ] **Step 1: Write failing test**

Add a test that first reinforces a trace, then uses low curve evidence to weaken it:

```rust
#[test]
fn low_pair_evidence_weakens_existing_binding_trace() {
    let vp = Arc::new(Viewport::new());
    let slot_id = claim_vib_slot(&vp, 1_000);
    let target = BindingTargetRef::ObjectSlot { slot_id: slot_id as u8 };

    let (good_write_seq, good_window_id) = paired_write_and_window(&vp, 2_000, 0.7, 0.69);
    assert!(vp.try_form_binding_event_from_pair(PatternKind::SymbolSeq, 0xDAD, target, good_write_seq, good_window_id));
    let (good_write_seq_2, good_window_id_2) = paired_write_and_window(&vp, 3_000, 0.7, 0.69);
    assert!(vp.try_form_binding_event_from_pair(PatternKind::SymbolSeq, 0xDAD, target, good_write_seq_2, good_window_id_2));
    let before = vp.read_language_binding().traces[0].strength.value;

    let (bad_write_seq, bad_window_id) = paired_write_and_window(&vp, 4_000, 0.9, 0.05);
    assert!(vp.try_form_binding_event_from_pair(PatternKind::SymbolSeq, 0xDAD, target, bad_write_seq, bad_window_id));

    let after = vp.read_language_binding();
    assert_eq!(after.n_bindings, 1);
    assert_eq!(after.traces[0].status, BindingStatus::Weakened);
    assert!(after.traces[0].strength.value < before);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test language_binding_runtime low_pair_evidence_weakens_existing_binding_trace -- --nocapture
```

Expected: FAIL until low-similarity evidence emits `BindingFeedback::Deviate`.

- [ ] **Step 3: Write minimal implementation**

In the bridge, map `curve_similarity <= BINDING_PAIR_DEVIATE_MAX` to `BindingFeedback::Deviate`. This should only adjust an existing trace because `LanguageBindingState::apply_binding_event` already refuses to create new traces from deviate-only evidence.

- [ ] **Step 4: Run focused test**

Run the same focused test. Expected: PASS.

## Chunk 2: Verification

### Task 5: Run Language Binding And Guardrail Suites

**Files:**
- Test only unless failures require fixes.

- [ ] **Step 1: Run focused language binding suites**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test language_binding --test language_binding_runtime -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run action consequence and cleanup guardrails**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test action_consequence_loop --test cleanup_hygiene -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run full suite**

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: PASS.
