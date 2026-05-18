# Product Cognition Baseline v1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a product-level cognition readiness checkpoint, using the "7-year human child" target only as an external benchmark for broad, stable, correctable cognition after training.

**Architecture:** Add an observer-side aggregate curriculum/report that requires Factory DNA Checkpoint v1 to pass, then verifies multi-target mark grounding, open-stream robustness, cross-channel coupling, sustained active probe closure, held-out generalization, chain memory, multi-step continuity, and long-horizon pressure evidence. No child model, personality, semantic ontology, fixed world facts, or request/response reasoning layer is introduced.

**Tech Stack:** Rust crate `bionic_v0`, existing curriculum runners, existing `curriculum_report` binary, Cargo integration tests.

---

## Dependency

Implement this plan only after `Factory DNA Checkpoint v1` is complete and `curriculum_report --factory-dna-checkpoint` emits `dna_checkpoint_pass=1`.

If the factory checkpoint is not implemented yet, stop here and execute:

`docs/superpowers/plans/2026-05-18-factory-dna-checkpoint-v1.md`

## Boundary Notes

- `Product cognition baseline` is an external product readiness standard. It is not an internal body module.
- `7-year human child` is an external benchmark phrase only. It must not introduce a child persona, human identity model, human social ontology, fixed morality, fixed language meanings, school curriculum, or cultural fact database.
- This checkpoint measures trained substrate capabilities: object/relation persistence, temporal binding, multimodal coupling, active probing, correction, generalization, memory sedimentation, and long-horizon reentry.
- Passing this checkpoint does not mean the body has innate human knowledge. It means the trained checkpoint has enough numeric evidence to be treated as the first product-grade baseline.

## File Structure

- Create: `bionic_v0/tests/product_cognition_baseline.rs`
  - Behavior test for product baseline pass criteria.
  - Hygiene test against child/personality/semantic ontology drift.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - CLI report coverage for `--product-cognition-baseline`.
- Modify: `bionic_v0/src/curriculum.rs`
  - Add `ProductCognitionBaselineEvidence`.
  - Add `run_product_cognition_baseline_curriculum`.
  - Aggregate existing curriculum evidence into one observer-side readiness report.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--product-cognition-baseline` flag.
  - Emit compact numeric evidence lines only.
- Do not modify `body` output behavior.
- Do not add an internal `Child`, `HumanChild`, `SevenYearOld`, `Personality`, `Semantic`, or `Concept` module.

## Chunk 1: RED Tests

### Task 1: Product Baseline Behavior Test

**Files:**
- Create: `bionic_v0/tests/product_cognition_baseline.rs`
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Write the failing behavior test**

Create `bionic_v0/tests/product_cognition_baseline.rs`:

```rust
use bionic_v0::curriculum::{run_product_cognition_baseline_curriculum, CurriculumConfig};

#[test]
fn product_cognition_baseline_requires_factory_dna_and_post_training_readiness() {
    let evidence = run_product_cognition_baseline_curriculum(CurriculumConfig::default())
        .expect("product baseline runs");

    assert_eq!(evidence.product_baseline_pass, 1);
    assert_eq!(evidence.factory_dna_checkpoint_pass, 1);
    assert!(evidence.course_count >= 10);
    assert_eq!(evidence.passing_courses, evidence.course_count);
    assert!(evidence.object_relation_readiness >= 1);
    assert!(evidence.text_grounding_readiness >= 1);
    assert!(evidence.open_stream_readiness >= 1);
    assert!(evidence.cross_channel_readiness >= 1);
    assert!(evidence.active_probe_readiness >= 1);
    assert!(evidence.heldout_generalization_readiness >= 1);
    assert!(evidence.chain_memory_readiness >= 1);
    assert!(evidence.multi_step_readiness >= 1);
    assert!(evidence.long_horizon_readiness >= 1);
    assert!(evidence.min_reentry_margin > 0.0);
    assert!(evidence.min_hold_ratio >= 0.70);
    assert!(evidence.max_wrong_binding_strength <= 0.01);
    assert_eq!(evidence.no_child_ontology_guard, 1);
}
```

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test product_cognition_baseline product_cognition_baseline_requires_factory_dna_and_post_training_readiness
```

Expected: FAIL because `run_product_cognition_baseline_curriculum` and `ProductCognitionBaselineEvidence` do not exist.

### Task 2: Product Baseline Report Test

**Files:**
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Add failing CLI output test**

Add:

```rust
#[test]
fn product_cognition_baseline_report_emits_numeric_evidence() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--product-cognition-baseline")
        .output()
        .expect("run curriculum_report");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("mode=product_cognition_baseline"));
    assert!(stdout.contains("factory_dna_checkpoint_pass="));
    assert!(stdout.contains("course_count="));
    assert!(stdout.contains("passing_courses="));
    assert!(stdout.contains("object_relation_readiness="));
    assert!(stdout.contains("text_grounding_readiness="));
    assert!(stdout.contains("open_stream_readiness="));
    assert!(stdout.contains("cross_channel_readiness="));
    assert!(stdout.contains("active_probe_readiness="));
    assert!(stdout.contains("heldout_generalization_readiness="));
    assert!(stdout.contains("chain_memory_readiness="));
    assert!(stdout.contains("multi_step_readiness="));
    assert!(stdout.contains("long_horizon_readiness="));
    assert!(stdout.contains("min_reentry_margin="));
    assert!(stdout.contains("min_hold_ratio="));
    assert!(stdout.contains("max_wrong_binding_strength="));
    assert!(stdout.contains("product_baseline_pass="));
}
```

- [ ] **Step 2: Run report test to verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report product_cognition_baseline_report_emits_numeric_evidence
```

Expected: FAIL because the report flag is missing.

## Chunk 2: Aggregate Evidence Model

### Task 3: Add Product Evidence Struct

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Add evidence type near existing evidence structs**

Add:

```rust
#[derive(Clone, Debug)]
pub struct ProductCognitionBaselineEvidence {
    pub factory_dna_checkpoint_pass: u8,
    pub course_count: u32,
    pub passing_courses: u32,
    pub object_relation_readiness: u8,
    pub text_grounding_readiness: u8,
    pub open_stream_readiness: u8,
    pub cross_channel_readiness: u8,
    pub active_probe_readiness: u8,
    pub heldout_generalization_readiness: u8,
    pub chain_memory_readiness: u8,
    pub multi_step_readiness: u8,
    pub long_horizon_readiness: u8,
    pub min_reentry_margin: f64,
    pub min_hold_ratio: f64,
    pub max_wrong_binding_strength: f64,
    pub no_child_ontology_guard: u8,
    pub product_baseline_pass: u8,
}
```

- [ ] **Step 2: Add local helpers**

Add private helpers if not already present:

```rust
fn pass_flag(value: bool) -> u8 {
    if value { 1 } else { 0 }
}

fn finite_min(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(f64::INFINITY, f64::min)
}

fn finite_max(values: &[f64]) -> f64 {
    values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .fold(f64::NEG_INFINITY, f64::max)
}
```

Use existing helpers instead if equivalent ones already exist in `curriculum.rs`.

## Chunk 3: Product Baseline Runner

### Task 4: Implement Aggregator Runner

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Add runner skeleton**

Add:

```rust
pub fn run_product_cognition_baseline_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<ProductCognitionBaselineEvidence> {
    let factory = run_factory_dna_checkpoint_curriculum(config)?;
    let text_grounding = run_text_grounding_suite(config)?;
    let text_open = run_text_open_stream_curriculum(config)?;
    let cross_channel = run_cross_channel_disturbance_coupling_curriculum(config)?;
    let sustained = run_sustained_active_probe_training_curriculum(config)?;
    let heldout = run_heldout_gap_generalization_curriculum(config)?;
    let multi_gap = run_multi_gap_selection_pixel_scale_curriculum(config)?;
    let chain_memory = run_chain_memory_sedimentation_curriculum(config)?;
    let multi_step = run_multi_step_chain_continuity_curriculum(config)?;
    let long_horizon = run_long_horizon_pressure_chain_curriculum(config)?;

    // Readiness computation in following steps.
}
```

This aggregate is intentionally observer-side. It can run each existing course in fresh course-local viewports because it is measuring the readiness suite, not one continuous body lifetime.

- [ ] **Step 2: Compute readiness flags**

Compute:

```rust
let factory_dna_checkpoint_pass = factory.dna_checkpoint_pass;

let object_relation_readiness = pass_flag(
    text_grounding.object_targets >= 3
        && text_grounding.relation_targets >= 2
        && text_grounding.time_window_targets >= 1
        && text_grounding.composition_relation_strength > 0.0,
);

let text_grounding_readiness = pass_flag(
    text_grounding.train_avg_strength > 0.5
        && text_grounding.hold_avg_ratio >= 0.80
        && text_grounding.reentry_persistent_avg > text_grounding.reentry_cold_avg
        && text_grounding.wrong_binding_strength <= 0.01,
);

let open_stream_readiness = pass_flag(
    text_open.stream_events >= 20
        && text_open.supported_marks >= 6
        && text_open.hold_supported_ratio >= 0.70
        && text_open.reentry_persistent_avg > text_open.reentry_cold_avg
        && text_open.wrong_binding_strength <= 0.01,
);

let cross_channel_readiness = pass_flag(
    cross_channel.text_vib_coupling_strength > 0.0
        && cross_channel.text_phot_coupling_strength > 0.0
        && cross_channel.vib_phot_coupling_strength > 0.0
        && cross_channel.reentry_persistent_coupling > cross_channel.reentry_cold_coupling
        && cross_channel.wrong_coupling_strength <= 0.01,
);

let active_probe_readiness = pass_flag(
    sustained.cycles_run >= 12
        && sustained.paired_consequences >= 24
        && sustained.min_correction_recovery_ratio >= 0.80
        && sustained.max_wrong_binding_strength <= 0.01
        && sustained.persistent_reentry_margin > 0.10,
);
```

Adjust field names to match the current `SustainedActiveProbeTrainingEvidence` implementation if they differ.

- [ ] **Step 3: Compute generalization and memory flags**

Use existing course fields:

```rust
let heldout_generalization_readiness = pass_flag(
    heldout.heldout_marks >= 6
        && heldout.probe_frames > 0
        && heldout.persistent_reentry_strength > heldout.cold_reentry_strength,
);

let chain_memory_readiness = pass_flag(
    chain_memory.chain_links >= 3
        && chain_memory.sedimented_strength > 0.0
        && chain_memory.reentry_strength > chain_memory.cold_strength,
);

let multi_step_readiness = pass_flag(
    multi_step.chain_steps >= 3
        && multi_step.final_chain_strength > 0.0
        && multi_step.reentry_strength > multi_step.cold_strength,
);

let long_horizon_readiness = pass_flag(
    long_horizon.pressure_chain_steps >= 3
        && long_horizon.long_horizon_reentry_strength > long_horizon.cold_reentry_strength,
);
```

If exact field names differ, inspect the structs at the top of `bionic_v0/src/curriculum.rs` and keep the same logical checks:

- enough trained/held-out material;
- nonzero closure/coupling/memory strength;
- persistent reentry beats cold;
- wrong binding remains bounded.

- [ ] **Step 4: Compute aggregate margins**

Aggregate conservative floor metrics:

```rust
let min_reentry_margin = finite_min(&[
    text_grounding.reentry_persistent_avg - text_grounding.reentry_cold_avg,
    text_open.reentry_persistent_avg - text_open.reentry_cold_avg,
    cross_channel.reentry_persistent_coupling - cross_channel.reentry_cold_coupling,
    sustained.persistent_reentry_margin,
]);

let min_hold_ratio = finite_min(&[
    factory.hold_retention_ratio,
    text_grounding.hold_avg_ratio,
    text_open.hold_supported_ratio,
]);

let max_wrong_binding_strength = finite_max(&[
    text_grounding.wrong_binding_strength,
    text_open.wrong_binding_strength,
    cross_channel.wrong_coupling_strength,
    sustained.max_wrong_binding_strength,
]);
```

Include additional wrong-binding fields from held-out/multi-step courses when available.

- [ ] **Step 5: Compute pass fields**

Set:

```rust
let readiness_flags = [
    factory_dna_checkpoint_pass,
    object_relation_readiness,
    text_grounding_readiness,
    open_stream_readiness,
    cross_channel_readiness,
    active_probe_readiness,
    heldout_generalization_readiness,
    chain_memory_readiness,
    multi_step_readiness,
    long_horizon_readiness,
];
let course_count = readiness_flags.len() as u32;
let passing_courses = readiness_flags.iter().filter(|&&flag| flag == 1).count() as u32;
let no_child_ontology_guard = product_cognition_source_guard();
let product_baseline_pass = pass_flag(
    passing_courses == course_count
        && min_reentry_margin > 0.0
        && min_hold_ratio >= 0.70
        && max_wrong_binding_strength <= 0.01
        && no_child_ontology_guard == 1,
);
```

- [ ] **Step 6: Verify behavior GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test product_cognition_baseline
```

Expected: PASS.

## Chunk 4: Report Mode

### Task 5: Add Product Report Output

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Import runner**

Add `run_product_cognition_baseline_curriculum` to the existing curriculum imports.

- [ ] **Step 2: Add flag branch**

Add a branch for `--product-cognition-baseline`:

```rust
let evidence = run_product_cognition_baseline_curriculum(CurriculumConfig::default())?;
println!("curriculum_report_v0");
println!("mode=product_cognition_baseline");
println!(
    "factory_dna_checkpoint_pass={}",
    evidence.factory_dna_checkpoint_pass
);
println!("course_count={}", evidence.course_count);
println!("passing_courses={}", evidence.passing_courses);
println!("object_relation_readiness={}", evidence.object_relation_readiness);
println!("text_grounding_readiness={}", evidence.text_grounding_readiness);
println!("open_stream_readiness={}", evidence.open_stream_readiness);
println!("cross_channel_readiness={}", evidence.cross_channel_readiness);
println!("active_probe_readiness={}", evidence.active_probe_readiness);
println!(
    "heldout_generalization_readiness={}",
    evidence.heldout_generalization_readiness
);
println!("chain_memory_readiness={}", evidence.chain_memory_readiness);
println!("multi_step_readiness={}", evidence.multi_step_readiness);
println!("long_horizon_readiness={}", evidence.long_horizon_readiness);
println!("min_reentry_margin={:.6}", evidence.min_reentry_margin);
println!("min_hold_ratio={:.6}", evidence.min_hold_ratio);
println!(
    "max_wrong_binding_strength={:.6}",
    evidence.max_wrong_binding_strength
);
println!("no_child_ontology_guard={}", evidence.no_child_ontology_guard);
println!("product_baseline_pass={}", evidence.product_baseline_pass);
```

- [ ] **Step 3: Verify report GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report product_cognition_baseline_report_emits_numeric_evidence
```

Expected: PASS.

## Chunk 5: Hygiene And Guard Tests

### Task 6: Add Product Hygiene Test

**Files:**
- Modify: `bionic_v0/tests/product_cognition_baseline.rs`

- [ ] **Step 1: Add rejected architecture scan**

Add:

```rust
#[test]
fn product_cognition_baseline_sources_do_not_introduce_child_or_semantic_ontology() {
    let sources = [
        include_str!("../src/curriculum.rs"),
        include_str!("../src/curriculum_governor.rs"),
        include_str!("../src/bin/curriculum_report.rs"),
        include_str!("../src/language_binding.rs"),
        include_str!("../src/mark_disturbance.rs"),
    ];

    for src in sources {
        for forbidden in [
            "semantic_readiness",
            "SymbolizerState",
            "concept_id",
            "token_id",
            "word_id",
            "object_name",
            "meaning_db",
            "dictionary",
            "parser",
            "tokenizer",
            "child_model",
            "seven_year_old",
            "human_child",
            "personality",
            "ethics",
            "identity",
            "owner",
            "eprintln!",
        ] {
            assert!(
                !src.contains(forbidden),
                "product cognition baseline path must not contain `{}`",
                forbidden
            );
        }
    }
}
```

- [ ] **Step 2: Add observer-only source guard**

Implement `product_cognition_source_guard()` in `curriculum.rs` by scanning compile-time source strings for the same forbidden terms or by returning `1` only after all hygiene tests cover the paths. Prefer a real local source scan if it follows existing project patterns.

- [ ] **Step 3: Run focused tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test product_cognition_baseline
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report product_cognition_baseline_report_emits_numeric_evidence
```

Expected: PASS.

## Chunk 6: Final Verification

### Task 7: Manual Report And Full Test Run

**Files:**
- Verify all touched files.

- [ ] **Step 1: Run factory report first**

Run:

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --factory-dna-checkpoint
```

Expected: numeric output with `dna_checkpoint_pass=1`.

- [ ] **Step 2: Run product report**

Run:

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --product-cognition-baseline
```

Expected: compact numeric output with `product_baseline_pass=1`.

- [ ] **Step 3: Format/check**

Run:

```bash
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --check
git diff --check
```

Expected: both pass.

- [ ] **Step 4: Run full crate tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add /Users/joe/unnamed/bionic_v0/src/curriculum.rs \
  /Users/joe/unnamed/bionic_v0/src/bin/curriculum_report.rs \
  /Users/joe/unnamed/bionic_v0/tests/product_cognition_baseline.rs \
  /Users/joe/unnamed/bionic_v0/tests/curriculum_report.rs
git commit -m "feat: add product cognition baseline checkpoint"
```
