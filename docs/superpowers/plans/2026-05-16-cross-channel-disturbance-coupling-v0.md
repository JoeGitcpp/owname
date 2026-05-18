# Cross-Channel Disturbance Coupling v0 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents are explicitly requested) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build observer-side evidence that text mark disturbance can couple with vib and phot disturbance through the same existing substrate targets, survive conflict/correction, and reenter better than cold baseline.

**Architecture:** Extend only `bionic_v0` curriculum, report, test, and evidence docs. Reuse `Viewport`, object slots, relation edges, time windows, mark disturbance, and existing `language_binding` pattern kinds. Add no semantic bridge, naming gate, parser, tokenizer, vocabulary, visual labeler, voice labeler, identity layer, or body-side output.

**Tech Stack:** Rust crate `bionic_v0`, Cargo integration tests, existing `curriculum_report` observer binary, Superpowers TDD/verification workflow.

---

## Guardrails

Read first:

1. `/Users/joe/unnamed/AGENTS.md`
2. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-cross-channel-disturbance-coupling-v0-design.md`
3. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-text-disturbance-open-stream-v3-design.md`
4. `/Users/joe/unnamed/docs/superpowers/specs/2026-04-20-language-binding-layer-design.md`

Hard bans:

- no body-side `println!` or `eprintln!`;
- no `semantic_readiness`, `symbolizer`, `concept_id`, token ids, word ids,
  object names, text-understanding fields, naming gates, meaning DB, parser,
  tokenizer, dictionary, image labeler, voice labeler, identity, owner, ethics,
  or personality;
- no module that translates one carrier into another;
- no fixed world categories beyond synthetic curriculum disturbance schedules.

## Files

- Create: `bionic_v0/tests/cross_channel_disturbance_coupling.rs`
  - Stage acceptance test and source hygiene test.
- Modify: `bionic_v0/src/curriculum.rs`
  - Add `CrossChannelDisturbanceCouplingEvidence`.
  - Add `run_cross_channel_disturbance_coupling_curriculum`.
  - Add private helpers for non-symbol pattern binding strength and shared
    target coupling scores.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--cross-channel-disturbance-coupling`.
  - Emit compact numeric observer evidence.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Add CLI report coverage.
- Create after verification:
  - `docs/superpowers/notes/2026-05-16-cross-channel-disturbance-coupling-v0-evidence.md`

## Acceptance Target

The implementation is accepted when:

- `stream_events >= 18`
- `text_marks >= 4`
- `vib_observations >= 4`
- `phot_worldline_updates >= 4`
- `object_targets >= 3`
- `relation_targets >= 2`
- `time_window_targets >= 2`
- `text_binding_strength > 0.5`
- `vib_slot_confidence > 0.0`
- `phot_spatial_confidence > 0.0`
- `text_vib_coupling_strength > 0.0`
- `text_phot_coupling_strength > 0.0`
- `vib_phot_coupling_strength > 0.0`
- `conflict_coupling_drop > 0.0`
- `wrong_coupling_strength <= 0.01`
- `correction_recovery_ratio >= 0.50`
- `reentry_persistent_coupling > reentry_cold_coupling`

## Task 1: RED Test For Cross-Channel Evidence

**Files:**

- Create: `bionic_v0/tests/cross_channel_disturbance_coupling.rs`

- [ ] **Step 1: Write failing behavior test**

```rust
use bionic_v0::curriculum::{
    run_cross_channel_disturbance_coupling_curriculum, CurriculumConfig,
};

#[test]
fn cross_channel_disturbance_coupling_proves_shared_targets_conflict_recovery_and_reentry() {
    let evidence = run_cross_channel_disturbance_coupling_curriculum(CurriculumConfig::default())
        .expect("cross-channel disturbance coupling runs");

    assert!(evidence.stream_events >= 18);
    assert!(evidence.text_marks >= 4);
    assert!(evidence.vib_observations >= 4);
    assert!(evidence.phot_worldline_updates >= 4);
    assert!(evidence.object_targets >= 3);
    assert!(evidence.relation_targets >= 2);
    assert!(evidence.time_window_targets >= 2);
    assert!(evidence.text_binding_strength > 0.5);
    assert!(evidence.vib_slot_confidence > 0.0);
    assert!(evidence.phot_spatial_confidence > 0.0);
    assert!(evidence.text_vib_coupling_strength > 0.0);
    assert!(evidence.text_phot_coupling_strength > 0.0);
    assert!(evidence.vib_phot_coupling_strength > 0.0);
    assert!(evidence.conflict_coupling_drop > 0.0);
    assert!(evidence.wrong_coupling_strength <= 0.01);
    assert!(evidence.correction_recovery_ratio >= 0.50);
    assert!(evidence.reentry_persistent_coupling > evidence.reentry_cold_coupling);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test cross_channel_disturbance_coupling cross_channel_disturbance_coupling_proves_shared_targets_conflict_recovery_and_reentry
```

Expected: compile failure because the new curriculum function does not exist.

## Task 2: RED/Guard Test For Highest-Principle Hygiene

**Files:**

- Modify: `bionic_v0/tests/cross_channel_disturbance_coupling.rs`

- [ ] **Step 1: Add source hygiene test**

Scan only production/observer sources, not the test file itself:

```rust
#[test]
fn cross_channel_disturbance_coupling_sources_do_not_introduce_rejected_ontology() {
    let sources = [
        include_str!("../src/mark_disturbance.rs"),
        include_str!("../src/language_binding.rs"),
        include_str!("../src/curriculum.rs"),
        include_str!("../src/bin/curriculum_report.rs"),
    ];
    let forbidden = [
        "semantic_readiness",
        "SymbolizerState",
        "concept_id",
        "token_id",
        "word_id",
        "object_name",
        "text_understanding",
        "name_gate",
        "meaning_db",
        "dictionary",
        "parser",
        "tokenizer",
        "image_label",
        "voice_label",
        "multimodal_semantic",
        "identity",
        "owner",
        "ethics",
        "personality",
        "eprintln!",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "source contains rejected cross-channel term: {term}"
            );
        }
    }
}
```

- [ ] **Step 2: Verify guard**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test cross_channel_disturbance_coupling cross_channel_disturbance_coupling_sources_do_not_introduce_rejected_ontology
```

Expected: pass unless existing source already contains a rejected production
term.

## Task 3: Implement Minimal Curriculum Evidence

**Files:**

- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Add evidence struct**

Add `CrossChannelDisturbanceCouplingEvidence` near the other evidence structs.

- [ ] **Step 2: Add runner**

Implement `run_cross_channel_disturbance_coupling_curriculum` by:

1. allocating existing text grounding targets;
2. binding four supported marks to object/relation/time targets;
3. applying vib and phot binding events to the same targets;
4. measuring coupling as the minimum compatible strength across carriers;
5. weakening one existing phot trace as conflict;
6. measuring wrong target leakage without creating durable wrong binding;
7. correcting the weakened trace;
8. running persistent and cold reentry comparison with the same bounded event
   budget.

- [ ] **Step 3: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test cross_channel_disturbance_coupling
```

Expected: both tests pass.

## Task 4: Report Coverage

**Files:**

- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Add failing report test**

Add test expecting:

- `mode=cross_channel_disturbance_coupling`
- `stream_events=`
- `text_marks=`
- `vib_observations=`
- `phot_worldline_updates=`
- `text_vib_coupling_strength=`
- `text_phot_coupling_strength=`
- `vib_phot_coupling_strength=`
- `conflict_coupling_drop=`
- `correction_recovery_ratio=`
- `reentry_persistent_coupling=`
- `reentry_cold_coupling=`

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report cross_channel_disturbance_coupling_report_emits_numeric_stage_evidence
```

Expected: fail because the flag is not wired.

- [ ] **Step 3: Implement report mode**

Add the flag to `parse_args`, call the new curriculum runner, and print only
observer-side numeric evidence.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report cross_channel_disturbance_coupling_report_emits_numeric_stage_evidence
```

Expected: pass.

## Task 5: Evidence And Verification

**Files:**

- Create: `docs/superpowers/notes/2026-05-16-cross-channel-disturbance-coupling-v0-evidence.md`

- [ ] **Step 1: Run report**

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --cross-channel-disturbance-coupling
```

- [ ] **Step 2: Run focused tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test cross_channel_disturbance_coupling --test curriculum_report
```

- [ ] **Step 3: Run full verification**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
git diff --check
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

- [ ] **Step 4: Record evidence**

Write the commands, outcomes, important report numbers, and any residual
pre-existing formatting issues into the evidence note.
