# Text Grounding Suite v2 Highest-Principle Guarded Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `Text Grounding Suite v2`, proving multi-text grounding behavior with hold, variant generalization, primitive composition, conflict correction, and reentry evidence while preserving the highest principle.

**Architecture:** Extend only observer-side curriculum/report/test paths. Reuse existing `mark_disturbance`, `language_binding`, `Viewport`, object slots, relation edges, and time-window targets. Add no tokenizer, parser, vocabulary, semantic layer, naming gate, concept allocator, request/response controller, or body-side output.

**Tech Stack:** Rust crate `bionic_v0`, Cargo integration tests, existing `curriculum_report` binary, Superpowers TDD/debugging/verification skills, optional subagents with disjoint scopes.

---

## Mandatory Guardrails

Before any implementation worker edits code, it must read:

1. `/Users/joe/unnamed/AGENTS.md`
2. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-text-grounding-suite-v2-design.md`
3. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-15-mark-disturbance-binding-v0-design.md`
4. `/Users/joe/unnamed/docs/superpowers/specs/2026-05-16-text-disturbance-primitive-understanding-v1-design.md`

Hard bans:

- no body-side `println!` or `eprintln!`;
- no `semantic_readiness`, `symbolizer`, `concept_id`, token ids, word ids,
  object names, text-understanding fields, or naming gates;
- no parser, tokenizer, vocabulary, dictionary, grammar tree, meaning DB, or
  text generation path;
- no adapter-side invented binding feedback;
- no fixed internal world categories.

## Subagent Protocol

Subagents are allowed but must be used deliberately.

Recommended subagent lanes:

1. **Explorer: boundary audit**
   - Read specs and current text/mark code.
   - Return risks, forbidden terms to scan, and likely files to touch.
   - Read-only.

2. **Worker A: curriculum API**
   - Own write set: `bionic_v0/src/curriculum.rs`,
     `bionic_v0/tests/text_grounding_suite.rs`.
   - Implement only `TextGroundingSuiteEvidence`,
     `run_text_grounding_suite`, and helper functions.

3. **Worker B: report CLI**
   - Own write set: `bionic_v0/src/bin/curriculum_report.rs`,
     `bionic_v0/tests/curriculum_report.rs`.
   - Implement only `--text-grounding-suite` report output and tests.

4. **Reviewer: highest-principle audit**
   - Read-only.
   - Check for rejected architecture, body output, overclaiming, and missing
     tests.

Main agent responsibilities:

- write or approve all failing tests before implementation;
- keep workers on disjoint write sets;
- integrate results;
- run final verification;
- produce the final evidence report.

## Files

- Modify: `bionic_v0/src/curriculum.rs`
  - Add `TextGroundingSuiteEvidence`.
  - Add `run_text_grounding_suite`.
  - Add private helpers for base disturbances, variant disturbances,
    composition windows, conflict/correction, and cold baseline measurement.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--text-grounding-suite`.
  - Emit compact numeric observer evidence.
- Create: `bionic_v0/tests/text_grounding_suite.rs`
  - TDD tests for all v2 acceptance criteria and hygiene.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - Add CLI output coverage.
- Optional docs update after implementation:
  - `docs/superpowers/notes/YYYY-MM-DD-text-grounding-suite-v2-evidence.md`

## Task 1: RED Tests For Suite Evidence

- [ ] **Step 1: Create failing suite test**

Create `bionic_v0/tests/text_grounding_suite.rs`:

```rust
use bionic_v0::curriculum::{run_text_grounding_suite, CurriculumConfig};

#[test]
fn text_grounding_suite_proves_multi_target_variant_composition_correction_and_reentry() {
    let evidence = run_text_grounding_suite(CurriculumConfig::default()).expect("suite runs");

    assert_eq!(evidence.train_marks, 6);
    assert!(evidence.object_targets >= 3);
    assert!(evidence.relation_targets >= 2);
    assert!(evidence.time_window_targets >= 1);
    assert!(evidence.variant_marks >= 3);
    assert!(evidence.composition_windows >= 2);
    assert!(evidence.train_avg_strength > 0.5);
    assert!(evidence.train_weakest_strength > 0.0);
    assert!(evidence.hold_avg_ratio >= 0.80);
    assert!(evidence.hold_weakest_ratio >= 0.60);
    assert!(evidence.variant_generalization_score > 0.0);
    assert!(evidence.variant_distinctness_ratio > 0.0);
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
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_grounding_suite text_grounding_suite_proves_multi_target_variant_composition_correction_and_reentry
```

Expected: compile failure because `run_text_grounding_suite` does not exist.

## Task 2: RED Test For Hygiene

- [ ] **Step 1: Add hygiene test**

In `bionic_v0/tests/text_grounding_suite.rs`, scan:

```rust
let sources = [
    include_str!("../src/mark_disturbance.rs"),
    include_str!("../src/language_binding.rs"),
    include_str!("../src/curriculum.rs"),
    include_str!("../src/bin/curriculum_report.rs"),
];
```

Reject:

```rust
[
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
]
```

- [ ] **Step 2: Verify**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_grounding_suite text_grounding_suite_sources_do_not_introduce_rejected_language_ontology
```

Expected: pass or fail based on current source strings. If it fails on a false
positive in comments or docs embedded in source, refine the source path or term
list only with a written boundary reason.

## Task 3: Implement Curriculum API

- [ ] **Step 1: Add evidence struct**

In `bionic_v0/src/curriculum.rs`, add:

```rust
#[derive(Clone, Copy, Debug, Default)]
pub struct TextGroundingSuiteEvidence {
    pub train_marks: u32,
    pub object_targets: u32,
    pub relation_targets: u32,
    pub time_window_targets: u32,
    pub variant_marks: u32,
    pub composition_windows: u32,
    pub train_avg_strength: f64,
    pub train_weakest_strength: f64,
    pub hold_avg_ratio: f64,
    pub hold_weakest_ratio: f64,
    pub variant_generalization_score: f64,
    pub variant_distinctness_ratio: f64,
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

- [ ] **Step 2: Add minimum helper set**

Use private helpers only:

- `run_text_binding_series`
- `strengths_for_targets`
- `mean_strength`
- `weakest_strength`
- `bind_composition_window`
- `variant_score`

Do not add new modules unless `curriculum.rs` becomes unmanageable.

- [ ] **Step 3: Build stage sequence**

`run_text_grounding_suite(config)` should:

1. create one persistent `Viewport`;
2. allocate three live object slots;
3. seed enough substrate state to form at least two relation edges;
4. train six base byte disturbances across objects, relation edges, and time
   windows;
5. train at least three variants against shared targets;
6. bind at least two adjacent-pair windows to relation/time-window targets;
7. tick maintenance for hold;
8. apply one or more conflict events with `BindingFeedback::Deviate`;
9. apply correction events with `BindingFeedback::Align`;
10. reenter supported evidence on persistent viewport;
11. run a cold viewport with reentry-only exposure;
12. return all numeric evidence.

- [ ] **Step 4: Verify GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_grounding_suite
```

Expected: all suite tests pass.

## Task 4: Report CLI

- [ ] **Step 1: Write failing report test**

In `bionic_v0/tests/curriculum_report.rs`, add:

```rust
#[test]
fn text_grounding_suite_report_emits_numeric_stage_evidence() {
    let output = Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--text-grounding-suite")
        .output()
        .expect("run curriculum_report text grounding suite");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("mode=text_grounding_suite"));
    assert!(stdout.contains("train_marks=6"));
    assert!(stdout.contains("variant_generalization_score="));
    assert!(stdout.contains("composition_relation_strength="));
    assert!(stdout.contains("correction_recovery_ratio="));
    assert!(stdout.contains("reentry_persistent_avg="));
    assert!(stdout.contains("reentry_cold_avg="));
}
```

- [ ] **Step 2: Verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report text_grounding_suite_report_emits_numeric_stage_evidence
```

Expected: fail because `--text-grounding-suite` is not wired.

- [ ] **Step 3: Implement CLI branch**

Modify `bionic_v0/src/bin/curriculum_report.rs` to parse
`--text-grounding-suite` and print all report fields from the evidence struct.

- [ ] **Step 4: Verify GREEN**

Run the same report test. Expected: pass.

## Task 5: Guarded Review Loop

- [ ] **Step 1: Run boundary searches**

```bash
rg -n "semantic_readiness|SymbolizerState|concept_id|token_id|word_id|object_name|text_understanding|name_gate|meaning_db|dictionary|parser|tokenizer|eprintln!" /Users/joe/unnamed/bionic_v0/src /Users/joe/unnamed/bionic_v0/tests
```

Expected: no new forbidden source usage in touched paths. Existing unrelated
matches must be classified before action.

- [ ] **Step 2: Optional reviewer subagent**

If subagents are available, dispatch a read-only reviewer:

```text
Read AGENTS.md, the v2 spec, and the diff. Do not edit files. Report whether
the implementation violates highest principle, rejected language architecture,
body/observer separation, or TDD acceptance.
```

- [ ] **Step 3: Fix only confirmed issues**

Use `superpowers:systematic-debugging` for unexpected failures or unstable
metrics. Do not adjust thresholds by guesswork.

## Task 6: Full Verification And Evidence Note

- [ ] **Step 1: Focused tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test text_grounding_suite
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report
```

- [ ] **Step 2: Full tests**

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

- [ ] **Step 3: Diff check**

```bash
git diff --check
```

- [ ] **Step 4: Capture report**

```bash
/Users/joe/unnamed/bionic_v0/target/debug/curriculum_report --text-grounding-suite
```

- [ ] **Step 5: Write evidence note**

Create:

`docs/superpowers/notes/YYYY-MM-DD-text-grounding-suite-v2-evidence.md`

Include:

- command list;
- exact numeric report;
- acceptance checklist;
- remaining risk;
- highest-principle boundary statement.

## Stop Conditions

Stop and redesign rather than patching forward if:

- v2 requires tokenizer/parser/vocabulary to pass;
- any body-side output becomes necessary;
- text disturbance starts creating objects by itself;
- a fixed meaning table appears attractive;
- cold baseline beats persistent reentry and debugging shows the acceptance
  model is invalid;
- variants can pass only by collapsing into the same trace.
