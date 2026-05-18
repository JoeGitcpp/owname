# Text Disturbance Open-Stream v3 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `Text Disturbance Open-Stream v3`, proving that a continuous mixed text disturbance stream can support stable binding, delayed evidence, composition, correction, hold, and reentry without introducing text ontology.

**Architecture:** Extend only observer-side curriculum/report/test paths. Reuse existing `mark_disturbance`, `language_binding`, `Viewport`, object slots, relation edges, and time-window targets. Add no tokenizer, parser, vocabulary, semantic layer, naming gate, concept allocator, request/response controller, or body-side output.

**Tech Stack:** Rust crate `bionic_v0`, Cargo integration tests, existing `curriculum_report` binary, Superpowers TDD/debugging/verification skills.

---

## Mandatory Guardrails

Before editing code, read:

1. `/Users/joe/unnamed/AGENTS.md`
2. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-text-disturbance-open-stream-v3-design.md`
3. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-text-grounding-suite-v2-design.md`
4. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-15-mark-disturbance-binding-v0-design.md`

Hard bans:

- no body-side `println!` or `eprintln!`;
- no `semantic_readiness`, `symbolizer`, `concept_id`, token ids, word ids,
  object names, text-understanding fields, or naming gates;
- no parser, tokenizer, vocabulary, dictionary, grammar tree, meaning DB, or
  text generation path;
- no adapter-side invented binding feedback;
- no fixed internal world categories.

## Files

- Modify: `bionic_v0/src/curriculum.rs`
  - Add `TextOpenStreamEvidence`.
  - Add `run_text_open_stream_curriculum`.
  - Add private helpers for mixed stream events, delayed windows, leakage
    measurement, and cold baseline measurement.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--text-open-stream`.
  - Emit compact numeric observer evidence.
- Create: `bionic_v0/tests/text_open_stream.rs`
  - TDD tests for all v3 acceptance criteria and hygiene.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Add CLI output coverage.
- Create after implementation:
  - `docs/superpowers/notes/2026-05-16-text-open-stream-v3-evidence.md`

## Task 1: RED Tests For Open-Stream Evidence

- [ ] **Step 1: Create failing suite test**

Create `bionic_v0/tests/text_open_stream.rs`:

```rust
use bionic_v0::curriculum::{run_text_open_stream_curriculum, CurriculumConfig};

#[test]
fn text_open_stream_proves_mixed_stream_delay_composition_correction_and_reentry() {
    let evidence =
        run_text_open_stream_curriculum(CurriculumConfig::default()).expect("open stream runs");

    assert!(evidence.stream_events >= 20);
    assert!(evidence.supported_marks >= 6);
    assert!(evidence.unrelated_marks >= 4);
    assert!(evidence.variant_marks >= 3);
    assert!(evidence.object_targets >= 3);
    assert!(evidence.relation_targets >= 2);
    assert!(evidence.time_window_targets >= 2);
    assert!(evidence.composition_windows >= 2);
    assert!(evidence.delayed_windows >= 2);
    assert!(evidence.train_supported_avg > 0.5);
    assert!(evidence.train_supported_weakest > 0.0);
    assert!(evidence.unrelated_leakage_strength < evidence.train_supported_avg);
    assert!(evidence.hold_supported_ratio >= 0.70);
    assert!(evidence.variant_persistent_avg > evidence.variant_cold_avg);
    assert!(evidence.delayed_binding_strength > 0.0);
    assert!(evidence.composition_relation_strength > 0.0);
    assert!(evidence.conflict_ambiguity_delta > 0.0);
    assert!(evidence.wrong_binding_strength <= 0.01);
    assert!(evidence.correction_recovery_ratio >= 0.50);
    assert!(evidence.reentry_persistent_avg > evidence.reentry_cold_avg);
    assert!(evidence.reentry_persistent_weakest > evidence.reentry_cold_weakest);
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_open_stream text_open_stream_proves_mixed_stream_delay_composition_correction_and_reentry
```

Expected: compile failure because `run_text_open_stream_curriculum` does not
exist.

## Task 2: RED/Guard Test For Hygiene

- [ ] **Step 1: Add hygiene test**

In `bionic_v0/tests/text_open_stream.rs`, add:

```rust
#[test]
fn text_open_stream_sources_do_not_introduce_rejected_language_ontology() {
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
        "eprintln!",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "source contains rejected language ontology term: {term}"
            );
        }
    }
}
```

- [ ] **Step 2: Verify guard**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_open_stream text_open_stream_sources_do_not_introduce_rejected_language_ontology
```

Expected: pass unless current source already contains a rejected term. If it
fails on a false positive in comments or docs embedded in source, refine the
source path or term list only with a written boundary reason.

## Task 3: Implement Curriculum API

- [ ] **Step 1: Add evidence struct**

In `bionic_v0/src/curriculum.rs`, add:

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct TextOpenStreamEvidence {
    pub stream_events: u32,
    pub supported_marks: u32,
    pub unrelated_marks: u32,
    pub variant_marks: u32,
    pub object_targets: u32,
    pub relation_targets: u32,
    pub time_window_targets: u32,
    pub composition_windows: u32,
    pub delayed_windows: u32,
    pub train_supported_avg: f64,
    pub train_supported_weakest: f64,
    pub unrelated_leakage_strength: f64,
    pub hold_supported_ratio: f64,
    pub variant_persistent_avg: f64,
    pub variant_cold_avg: f64,
    pub delayed_binding_strength: f64,
    pub composition_relation_strength: f64,
    pub conflict_ambiguity_delta: f64,
    pub wrong_binding_strength: f64,
    pub correction_recovery_ratio: f64,
    pub reentry_persistent_avg: f64,
    pub reentry_cold_avg: f64,
    pub reentry_persistent_weakest: f64,
    pub reentry_cold_weakest: f64,
}
```

- [ ] **Step 2: Add `run_text_open_stream_curriculum`**

The function should:

1. create one persistent `Viewport`;
2. allocate three live object slots;
3. seed relations and time-window targets;
4. observe at least twenty timed mark events across supported, unrelated,
   variant, delayed, conflict, and reentry phases;
5. align supported marks to existing substrate targets;
6. observe unrelated marks without supported target alignment;
7. align variants in persistent and cold streams for comparison;
8. bind two delayed windows after later target intervals;
9. bind two adjacent-pair windows to relation/time-window targets;
10. apply conflict events with `BindingFeedback::Deviate`;
11. apply correction events with `BindingFeedback::Align`;
12. compare persistent reentry against cold baseline;
13. return all numeric evidence.

- [ ] **Step 3: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_open_stream
```

Expected: all open-stream tests pass.

## Task 4: Report CLI

- [ ] **Step 1: Write failing report test**

In `bionic_v0/tests/curriculum_report.rs`, add:

```rust
#[test]
fn text_open_stream_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--text-open-stream")
        .output()
        .expect("run curriculum_report text open stream");

    assert!(
        output.status.success(),
        "text open stream curriculum_report should exit successfully: status={:?} stderr={}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8(output.stdout).expect("stdout is utf8");
    assert!(stdout.contains("curriculum_report_v0"));
    assert!(stdout.contains("mode=text_open_stream"));
    assert!(stdout.contains("stream_events="));
    assert!(stdout.contains("supported_marks="));
    assert!(stdout.contains("unrelated_marks="));
    assert!(stdout.contains("variant_marks="));
    assert!(stdout.contains("train_supported_avg="));
    assert!(stdout.contains("unrelated_leakage_strength="));
    assert!(stdout.contains("hold_supported_ratio="));
    assert!(stdout.contains("variant_persistent_avg="));
    assert!(stdout.contains("variant_cold_avg="));
    assert!(stdout.contains("delayed_binding_strength="));
    assert!(stdout.contains("composition_relation_strength="));
    assert!(stdout.contains("conflict_ambiguity_delta="));
    assert!(stdout.contains("wrong_binding_strength="));
    assert!(stdout.contains("correction_recovery_ratio="));
    assert!(stdout.contains("reentry_persistent_avg="));
    assert!(stdout.contains("reentry_cold_avg="));
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report text_open_stream_report_emits_numeric_stage_evidence
```

Expected: fail because `--text-open-stream` is not implemented.

- [ ] **Step 3: Implement report mode**

Modify `bionic_v0/src/bin/curriculum_report.rs` to parse
`--text-open-stream`, call `run_text_open_stream_curriculum`, and print every
numeric field in the evidence struct.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report text_open_stream_report_emits_numeric_stage_evidence
```

Expected: pass.

## Task 5: Evidence Note And Final Verification

- [ ] **Step 1: Run report**

Run:

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --text-open-stream
```

Expected: emits `mode=text_open_stream` and all numeric fields.

- [ ] **Step 2: Create evidence note**

Create `docs/superpowers/notes/2026-05-16-text-open-stream-v3-evidence.md`
with:

- exact report command;
- exact numeric output;
- focused test commands;
- full verification commands;
- boundary note that text remains external mark disturbance.

- [ ] **Step 3: Run focused tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_open_stream --test curriculum_report
```

Expected: pass.

- [ ] **Step 4: Run full tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Expected: pass.

- [ ] **Step 5: Run diff hygiene**

Run:

```bash
git diff --check
```

Expected: pass.

- [ ] **Step 6: Run format check**

Run:

```bash
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml -- --check
```

Expected: pass, or fail only on pre-existing unrelated files with exact paths
reported in the handoff.

## Self-Review

- Spec coverage: every v3 acceptance item maps to Task 1, Task 3, Task 4, or
  Task 5.
- Placeholder scan: no TBD/TODO/fill-later placeholders remain.
- Type consistency: public API names are `TextOpenStreamEvidence` and
  `run_text_open_stream_curriculum`; report mode is `--text-open-stream` and
  output mode is `text_open_stream`.
