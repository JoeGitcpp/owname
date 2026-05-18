# Tri-Modal Primitive Bias Curriculum v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build an external curriculum/report path that proves one live object slot can repeatedly carry VIB, PHOT, and byte-mark disturbances together, hold the learned tendency, and re-enter stronger than a cold baseline.

**Architecture:** The body substrate remains unchanged in shape: VIB/PHOT are written through existing slot channels, byte marks enter through the existing disturbance and binding path, and the observer-side curriculum collects numeric evidence. No new internal ontology, naming gate, content category, or request/response layer is introduced.

**Tech Stack:** Rust crate `bionic_v0`, existing `Viewport`, `curriculum_report` binary, Rust integration tests.

---

## Boundary Notes

- "Byte mark" is an external disturbance carrier and engineering index. It is not an internal object name, meaning unit, concept allocator, or token ontology.
- "Primitive bias" means a measurable learned/reused routing tendency in the substrate after repeated co-occurrence. It does not define what the object really is.
- The curriculum is observer-side. It may print report lines. The `body` process remains silent and untouched.
- VIB and PHOT are base-element disturbances. The course may coordinate them externally, but it must not add fixed world categories or programmer-written object ranges.

## Files

- Modify: `bionic_v0/src/curriculum.rs`
  - Add `TriModalPrimitiveBiasEvidence`.
  - Add `run_tri_modal_primitive_bias_curriculum`.
  - Add a small private helper that drives one live slot with coordinated VIB, PHOT, spatial, and byte-mark observations.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--tri-modal-primitive`.
  - Emit numeric evidence lines only.
- Create: `bionic_v0/tests/primitive_bias_curriculum.rs`
  - TDD tests for hold, re-entry, cold baseline, and forbidden architecture terms.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Add CLI output coverage for `--tri-modal-primitive`.

## Task 1: Curriculum Evidence API

**Files:**
- Test: `bionic_v0/tests/primitive_bias_curriculum.rs`
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Write the failing test**

```rust
use bionic_v0::curriculum::{
    run_tri_modal_primitive_bias_curriculum, CurriculumConfig,
};

#[test]
fn tri_modal_primitive_curriculum_holds_and_reenters_against_cold_baseline() {
    let evidence =
        run_tri_modal_primitive_bias_curriculum(CurriculumConfig::default()).expect("course runs");

    assert_eq!(evidence.train_episodes, 4);
    assert_eq!(evidence.reentry_episodes, 2);
    assert!(evidence.vib_observations >= evidence.train_episodes + evidence.reentry_episodes);
    assert!(evidence.phot_worldline_updates >= evidence.train_episodes);
    assert!(evidence.mark_recurrence >= evidence.train_episodes);
    assert!(evidence.train_binding_strength > 0.5);
    assert!(evidence.hold_binding_strength_ratio >= 0.80);
    assert!(evidence.reentry_persistent_strength > evidence.reentry_cold_strength);
    assert!(evidence.final_ambiguity <= evidence.train_ambiguity + 0.25);
}
```

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test primitive_bias_curriculum tri_modal_primitive_curriculum_holds_and_reenters_against_cold_baseline`

Expected: FAIL because `run_tri_modal_primitive_bias_curriculum` does not exist.

- [ ] **Step 3: Implement minimum evidence path**

Add a curriculum function that:

1. Allocates one live slot.
2. Runs four training episodes.
3. Each episode writes VIB/PHOT through `seed_attended_slot`, writes PHOT spatial movement through `write_slot_spatial`, observes one stable byte mark, and aligns the latest mark to the live object slot.
4. Measures recurrence, spatial worldline updates, binding strength, ambiguity, and hold ratio.
5. Runs two re-entry episodes on the persistent viewport.
6. Runs the same two re-entry episodes on a fresh cold viewport.
7. Returns numeric evidence.

- [ ] **Step 4: Verify GREEN**

Run the same test. Expected: PASS.

## Task 2: Observer Report Output

**Files:**
- Test: `bionic_v0/tests/curriculum_report.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Write the failing report test**

Add a test that runs:

```rust
Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
    .arg("--tri-modal-primitive")
```

Expected stdout contains:

```text
mode=tri_modal_primitive
train_episodes=4
vib_observations=
phot_worldline_updates=
mark_recurrence=
hold_binding_strength_ratio=
reentry_persistent_strength=
reentry_cold_strength=
final_ambiguity=
```

- [ ] **Step 2: Verify RED**

Run: `cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report tri_modal_primitive_report_emits_numeric_stage_evidence`

Expected: FAIL because the flag is ignored or the output is missing.

- [ ] **Step 3: Implement the report flag**

Parse `--tri-modal-primitive`, call `run_tri_modal_primitive_bias_curriculum`, and print numeric observer-side report lines.

- [ ] **Step 4: Verify GREEN**

Run the same report test. Expected: PASS.

## Task 3: Architecture Hygiene

**Files:**
- Test: `bionic_v0/tests/primitive_bias_curriculum.rs`

- [ ] **Step 1: Write the failing-or-passing hygiene test**

Scan the new curriculum/report path for rejected architecture terms:

```rust
for forbidden in [
    "semantic_readiness",
    "SymbolizerState",
    "concept_id",
    "token_id",
    "word_id",
    "object_name",
    "text_understanding",
    "name_gate",
    "eprintln!",
] {
    assert!(!src.contains(forbidden));
}
```

- [ ] **Step 2: Verify**

Run: `cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test primitive_bias_curriculum`

Expected: PASS after implementation.

## Task 4: Full Verification

**Files:**
- All touched files.

- [ ] **Step 1: Run focused tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test primitive_bias_curriculum
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report
```

- [ ] **Step 2: Run full crate tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

- [ ] **Step 3: Check whitespace**

```bash
git diff --check
```

- [ ] **Step 4: Capture handoff evidence**

Run:

```bash
/Users/joe/unnamed/bionic_v0/target/debug/curriculum_report --tri-modal-primitive
```

Record the numeric report in the final answer.
