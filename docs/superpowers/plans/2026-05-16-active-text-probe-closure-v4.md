# Active Text Probe Closure v4 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `Active Text Probe Closure v4`, proving that ambiguity in a continuous text disturbance stream can trigger bounded framed-byte probe write-out, receive ordinary ingress return, form action-consequence pairs, reduce ambiguity, recover supported binding, and reenter better than cold baseline.

**Architecture:** Extend only observer-side curriculum/report/test paths. Reuse `mark_disturbance`, `language_binding`, `framed_byte_egress`, `action_output`, `action_consequence`, `Viewport`, object slots, relation edges, and time-window targets. Add no tokenizer, parser, vocabulary, semantic layer, naming gate, concept allocator, text controller, request/response controller, or body-side human-readable output.

**Tech Stack:** Rust crate `bionic_v0`, Cargo integration tests, existing `curriculum_report` binary, existing framed-byte/action-consequence substrate, Superpowers TDD/debugging/verification skills.

---

## Mandatory Guardrails

Before editing code, read:

1. `/Users/joe/unnamed/AGENTS.md`
2. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-text-disturbance-open-stream-v3-design.md`
3. `/Users/joe/unnamed/docs/superpowers/specs/2026-04-21-endogenous-activity-and-probe-laws-design.md`
4. `/Users/joe/unnamed/docs/superpowers/specs/2026-04-21-action-output-substrate-minimal-contract-design.md`
5. `/Users/joe/unnamed/docs/superpowers/specs/2026-04-21-action-consequence-loop-and-imitation-laws-design.md`
6. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-05-live-io-preflight-life-sign-contract-design.md`

Hard bans:

- no body-side `println!` or `eprintln!`;
- no `semantic_readiness`, `symbolizer`, `concept_id`, token ids, word ids,
  object names, text-understanding fields, naming gates, or text controllers;
- no parser, tokenizer, vocabulary, dictionary, grammar tree, meaning DB, or
  text generation path;
- no adapter-side invented binding feedback;
- no fixed internal world categories, target/result labels, source identity,
  owner, ethics, or personality;
- no new body organ/module for active text probing.

High-risk term boundary for this plan:

- `text`, `probe`, `return`, `target`, `correction`, `trigger`, and
  `closure` are external design descriptions or engineering report fields.
- Stream ids, mark hashes, write sequences, window ids, pair counts, byte
  checksums, and target refs are engineering indices only.
- The implementation must not store what a text item means, what an output
  says, what object is named, whether a return is a success, or who owns a
  source.
- Observer-side reports may print compact numeric evidence; body-side code
  must stay silent.

## Files

- Modify: `bionic_v0/src/curriculum.rs`
  - Add `ActiveTextProbeClosureEvidence`.
  - Add `run_active_text_probe_closure_curriculum`.
  - Add private helpers for probe byte derivation, probe write/return cycles,
    pair similarity measurement, and cold baseline measurement.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--active-text-probe-closure`.
  - Emit compact numeric observer evidence.
- Create: `bionic_v0/tests/active_text_probe_closure.rs`
  - TDD tests for all v4 acceptance criteria and hygiene.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Add CLI output coverage.
- Create after implementation:
  - `docs/superpowers/notes/2026-05-16-active-text-probe-closure-v4-evidence.md`

## v4 Acceptance Target

The implementation is accepted only when observer-side evidence proves all of:

1. a v3-style mixed text disturbance stream produces stable supported binding;
2. conflict raises mark ambiguity without sedimenting wrong binding;
3. raised ambiguity leads to at least two bounded framed-byte probe writes;
4. probe writes create action-output write epochs;
5. ordinary framed-byte returns create consequence windows;
6. write epochs and return windows pair through action-consequence substrate;
7. pair similarity is numeric and nonzero;
8. supported correction after probe return reduces ambiguity;
9. supported binding recovers after correction;
10. persistent reentry beats cold baseline;
11. report output is observer-side numeric only;
12. hygiene tests prove no rejected ontology or body-side human-readable output.

## Task 1: RED Tests For Active Probe Closure Evidence

**Files:**

- Create: `bionic_v0/tests/active_text_probe_closure.rs`
- Later modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Create failing suite test**

Create `bionic_v0/tests/active_text_probe_closure.rs`:

```rust
use bionic_v0::curriculum::{run_active_text_probe_closure_curriculum, CurriculumConfig};

#[test]
fn active_text_probe_closure_proves_probe_return_correction_and_reentry() {
    let evidence = run_active_text_probe_closure_curriculum(CurriculumConfig::default())
        .expect("active text probe closure runs");

    assert!(evidence.stream_events >= 20);
    assert!(evidence.supported_marks >= 6);
    assert!(evidence.train_supported_avg > 0.5);
    assert!(evidence.conflict_events >= 2);
    assert!(evidence.conflict_ambiguity_delta > 0.0);
    assert!(evidence.probe_trigger_events >= 2);
    assert!(evidence.probe_frames >= 2);
    assert!(evidence.probe_write_epochs >= 2);
    assert!(evidence.return_windows >= 2);
    assert!(evidence.paired_consequences >= 2);
    assert!(evidence.probe_pair_similarity_avg > 0.0);
    assert!(evidence.pre_probe_ambiguity > evidence.post_probe_ambiguity);
    assert!(evidence.ambiguity_reduction_ratio > 0.10);
    assert!(evidence.wrong_binding_strength <= 0.01);
    assert!(evidence.post_probe_supported_strength >= evidence.pre_probe_supported_strength);
    assert!(evidence.correction_recovery_ratio >= 0.50);
    assert!(evidence.egress_projection_confidence > 0.0);
    assert!(evidence.reentry_persistent_strength > evidence.reentry_cold_strength);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_text_probe_closure active_text_probe_closure_proves_probe_return_correction_and_reentry
```

Expected: compile failure because `run_active_text_probe_closure_curriculum`
does not exist.

## Task 2: RED/Guard Test For Highest-Principle Hygiene

**Files:**

- Modify: `bionic_v0/tests/active_text_probe_closure.rs`

- [ ] **Step 1: Add hygiene test**

Append this test to `bionic_v0/tests/active_text_probe_closure.rs`:

```rust
#[test]
fn active_text_probe_closure_sources_do_not_introduce_rejected_ontology() {
    let sources = [
        include_str!("../src/mark_disturbance.rs"),
        include_str!("../src/language_binding.rs"),
        include_str!("../src/framed_byte_egress.rs"),
        include_str!("../src/action_output.rs"),
        include_str!("../src/action_consequence.rs"),
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
        "request_response",
        "answer_model",
        "target_label",
        "result_label",
        "source_identity",
        "owner",
        "ethics",
        "personality",
        "eprintln!",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "source contains rejected active text probe term: {term}"
            );
        }
    }
}
```

- [ ] **Step 2: Verify guard**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_text_probe_closure active_text_probe_closure_sources_do_not_introduce_rejected_ontology
```

Expected: pass unless current source already contains a rejected term. If it
fails on a test-only forbidden list, do not scan the test file itself. If it
fails on an implementation comment, either remove the comment or add a written
boundary reason in the evidence note.

## Task 3: Implement Curriculum Evidence API

**Files:**

- Modify: `bionic_v0/src/curriculum.rs`
- Test: `bionic_v0/tests/active_text_probe_closure.rs`

- [ ] **Step 1: Add evidence struct**

In `bionic_v0/src/curriculum.rs`, add near the other curriculum evidence
structs:

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct ActiveTextProbeClosureEvidence {
    pub stream_events: u32,
    pub supported_marks: u32,
    pub train_supported_avg: f64,
    pub conflict_events: u32,
    pub conflict_ambiguity_delta: f64,
    pub probe_trigger_events: u32,
    pub probe_frames: u32,
    pub probe_write_epochs: u32,
    pub return_windows: u32,
    pub paired_consequences: u32,
    pub probe_pair_similarity_avg: f64,
    pub pre_probe_ambiguity: f64,
    pub post_probe_ambiguity: f64,
    pub ambiguity_reduction_ratio: f64,
    pub wrong_binding_strength: f64,
    pub pre_probe_supported_strength: f64,
    pub post_probe_supported_strength: f64,
    pub correction_recovery_ratio: f64,
    pub egress_projection_confidence: f64,
    pub reentry_persistent_strength: f64,
    pub reentry_cold_strength: f64,
}
```

- [ ] **Step 2: Add private helper signatures**

Add private helpers in `bionic_v0/src/curriculum.rs` near existing text
curriculum helpers:

```rust
fn active_probe_bytes(mark_hash: u64, ambiguity: f64, salt: u64) -> [u8; 8] {
    let scaled = (ambiguity.clamp(0.0, 1.0) * 65_535.0) as u64;
    let mut seed = mark_hash ^ scaled.rotate_left(9) ^ salt.rotate_left(17);
    let mut out = [0u8; 8];
    for (idx, byte) in out.iter_mut().enumerate() {
        seed = seed
            .wrapping_mul(0x0000_0100_0000_01b3)
            .wrapping_add(idx as u64 + 1);
        *byte = ((seed >> ((idx % 8) * 8)) as u8).rotate_left(1) ^ 0xa5;
    }
    out
}

fn average_pair_similarity(vp: &Viewport) -> f64 {
    let ac = vp.read_action_consequence();
    if ac.pairs.is_empty() {
        0.0
    } else {
        ac.pairs.iter().map(|pair| pair.curve_similarity).sum::<f64>() / ac.pairs.len() as f64
    }
}
```

- [ ] **Step 3: Add curriculum function**

Add:

```rust
pub fn run_active_text_probe_closure_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<ActiveTextProbeClosureEvidence> {
    const STREAM_ID: u64 = 47;
    const PROBE_STREAM_ID: u64 = 53;
    const TRAIN_EPISODES: u32 = 4;
    const REENTRY_EPISODES: u32 = 2;
    const HOLD_US: u64 = 2_200_000;
    const SUPPORTED_MARKS: [&[u8]; 6] = [
        b"probe-a",
        b"probe-b",
        b"probe-c",
        b"probe-ab",
        b"probe-bc",
        b"probe-window",
    ];
    const UNRELATED_MARKS: [&[u8]; 4] = [
        b"probe-noise-x",
        b"probe-noise-y",
        b"probe-noise-z",
        b"probe-noise-window",
    ];

    let vp = Arc::new(Viewport::new());
    let targets = setup_text_grounding_targets(&vp, config);
    let step_us = config.step_us.max(1);
    let supported_targets = [
        targets.object_targets[0],
        targets.object_targets[1],
        targets.object_targets[2],
        targets.relation_targets[0],
        targets.relation_targets[1],
        targets.time_target,
    ];

    // Implement with these stages:
    // 1. Train a mixed stream: supported marks bind to supported_targets;
    //    unrelated marks are observed only.
    // 2. Record train strengths and base ambiguity.
    // 3. Apply two conflict observations using BindingFeedback::Deviate against
    //    an unsupported object target; measure ambiguity rise and wrong binding.
    // 4. For each conflict-triggered probe:
    //    a. derive bounded bytes with active_probe_bytes;
    //    b. call vp.begin_framed_byte_egress;
    //    c. call vp.observe_framed_byte_return on the same probe stream after a
    //       short delay with nonzero magnitude and bounded deviation;
    //    d. call vp.stop_framed_byte_egress;
    //    e. reobserve the supported mark and Align to its supported target.
    // 5. Measure action_output write epochs, framed-byte frames/returns,
    //    action-consequence pairs, pair similarity, ambiguity reduction, and
    //    supported binding recovery.
    // 6. Reenter supported marks on the persistent viewport and compare with a
    //    cold viewport that receives only reentry exposure.
    // 7. Return ActiveTextProbeClosureEvidence.
}
```

Implementation notes:

- Reuse existing helpers: `setup_text_grounding_targets`,
  `setup_simple_text_targets`, `run_text_binding_series`, `bind_mark_set`,
  `strengths_for_targets`, `mean_strength`, `mark_ambiguity`,
  `symbol_binding_strength`, and `ratio_or_zero`.
- Count `probe_write_epochs` from `vp.read_action_output().n_write_epochs`.
- Count `return_windows` from `vp.read_framed_byte_egress().n_returns`.
- Count `paired_consequences` from `vp.read_action_consequence().n_pairs`.
- Use `observe_framed_byte_return`; it already creates ordinary consequence
  windows and tries auto-pairing.
- Do not write files or use `LiveByteClosureDriver` in this curriculum. This
  curriculum needs deterministic in-memory proof, not filesystem loopback.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_text_probe_closure
```

Expected: all active text probe closure tests pass.

## Task 4: Report CLI

**Files:**

- Modify: `bionic_v0/tests/curriculum_report.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Write failing report test**

In `bionic_v0/tests/curriculum_report.rs`, add:

```rust
#[test]
fn active_text_probe_closure_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--active-text-probe-closure")
        .output()
        .expect("run curriculum_report active text probe closure");

    assert!(
        output.status.success(),
        "active text probe closure curriculum_report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=active_text_probe_closure"));
    assert!(stdout.contains("stream_events="));
    assert!(stdout.contains("supported_marks="));
    assert!(stdout.contains("train_supported_avg="));
    assert!(stdout.contains("conflict_events="));
    assert!(stdout.contains("conflict_ambiguity_delta="));
    assert!(stdout.contains("probe_trigger_events="));
    assert!(stdout.contains("probe_frames="));
    assert!(stdout.contains("probe_write_epochs="));
    assert!(stdout.contains("return_windows="));
    assert!(stdout.contains("paired_consequences="));
    assert!(stdout.contains("probe_pair_similarity_avg="));
    assert!(stdout.contains("pre_probe_ambiguity="));
    assert!(stdout.contains("post_probe_ambiguity="));
    assert!(stdout.contains("ambiguity_reduction_ratio="));
    assert!(stdout.contains("wrong_binding_strength="));
    assert!(stdout.contains("post_probe_supported_strength="));
    assert!(stdout.contains("correction_recovery_ratio="));
    assert!(stdout.contains("egress_projection_confidence="));
    assert!(stdout.contains("reentry_persistent_strength="));
    assert!(stdout.contains("reentry_cold_strength="));
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report active_text_probe_closure_report_emits_numeric_stage_evidence
```

Expected: fail because `--active-text-probe-closure` is not implemented.

- [ ] **Step 3: Implement report mode**

Modify `bionic_v0/src/bin/curriculum_report.rs`:

- import `run_active_text_probe_closure_curriculum`;
- add a boolean for `--active-text-probe-closure`;
- place the branch before older text modes;
- print `mode=active_text_probe_closure`;
- print every field from `ActiveTextProbeClosureEvidence` as compact numeric
  key/value lines.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report active_text_probe_closure_report_emits_numeric_stage_evidence
```

Expected: pass.

## Task 5: Evidence Note And Automation Target

**Files:**

- Create: `docs/superpowers/notes/2026-05-16-active-text-probe-closure-v4-evidence.md`
- Update existing automation only after implementation is green:
  `persistent-curriculum-evidence-closure`

- [ ] **Step 1: Run report**

Run:

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --active-text-probe-closure
```

Expected: emits `mode=active_text_probe_closure` and all numeric fields.

- [ ] **Step 2: Create evidence note**

Create `docs/superpowers/notes/2026-05-16-active-text-probe-closure-v4-evidence.md`
with:

- exact report command;
- exact numeric output;
- RED test evidence;
- focused and full verification commands;
- boundary note that text remains external mark disturbance and framed-byte
  output remains bounded body write-out, not human text generation.

- [ ] **Step 3: Update automation prompt**

Use the Codex automation tool to update existing automation
`persistent-curriculum-evidence-closure`:

- keep `status=ACTIVE`;
- keep schedule `FREQ=HOURLY;INTERVAL=2`;
- change current target focus to Active Text Probe Closure v4;
- include the v4 spec/plan/evidence note paths;
- keep all highest-principle bans.

Do not create a duplicate automation.

## Task 6: Final Verification

**Files:**

- No new source files beyond previous tasks.

- [ ] **Step 1: Run focused tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test active_text_probe_closure --test curriculum_report
```

Expected: pass.

- [ ] **Step 2: Run full tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Expected: pass.

- [ ] **Step 3: Run diff hygiene**

Run:

```bash
git diff --check
```

Expected: pass.

- [ ] **Step 4: Run format check**

Run:

```bash
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

Expected: pass, or fail only on pre-existing unrelated files with exact paths
reported in the handoff. Do not edit unrelated files just to make this check
green unless the user explicitly asks.

## Self-Review

- Spec coverage: the plan covers active trigger, write-out, return, temporal
  pairing, correction, hold/reentry comparison, report output, automation, and
  highest-principle hygiene.
- Placeholder scan: no TBD/TODO/fill-later placeholders remain.
- Type consistency: public API names are `ActiveTextProbeClosureEvidence` and
  `run_active_text_probe_closure_curriculum`; report mode is
  `--active-text-probe-closure`; output mode is `active_text_probe_closure`.
- Scope check: this is one implementation target. It does not enter phot/vib
  bridging, natural language understanding, text generation, or live chat I/O.
