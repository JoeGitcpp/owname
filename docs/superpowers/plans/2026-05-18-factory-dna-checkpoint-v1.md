# Factory DNA Checkpoint v1 Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a factory checkpoint course that proves the body starts with usable digital/character structure, primitive visual marker, primitive acoustic pitch-band, and cross-channel binding sensitivity without preloading meanings.

**Architecture:** Add an observer-side curriculum/report that exercises existing `BiasPriors`, `Viewport`, mark disturbance, VIB, PHOT, language binding, hold/reentry, and conflict-correction paths. The checkpoint emits compact numeric evidence only; the body receives no fixed vocabulary, visual class table, speech inventory, semantic gate, or human-readable output.

**Tech Stack:** Rust crate `bionic_v0`, existing `Viewport`, `BiasPriors`, `curriculum.rs`, `curriculum_report` binary, Cargo integration tests.

---

## Boundary Notes

- `Factory DNA` means the recoverable birth-sensitivity surface plus a minimal pre-run training checkpoint. It is not permanent knowledge.
- `digit`, `character`, `visual marker`, `pitch`, `audio`, and `video` are external descriptions of disturbance carriers or observer evidence. They must not become body ontology units.
- The body may become more sensitive to sequence order, recurrence, boundary, frequency-band separation, visual boundary changes, and cross-channel co-occurrence.
- The body must not store what any digit/character means, what any visual marker is named, what any sound is called, or whether a mark is true.
- This checkpoint may create learned deltas and binding traces. Real runtime evidence must remain able to weaken, overwrite, or reverse them.

## File Structure

- Create: `bionic_v0/tests/factory_dna_checkpoint.rs`
  - Behavior test for the factory checkpoint pass standard.
  - Hygiene test for rejected ontology and body-side output drift.
- Modify: `bionic_v0/tests/curriculum_report.rs`
  - CLI report coverage for `--factory-dna-checkpoint`.
- Modify: `bionic_v0/src/curriculum.rs`
  - Add `FactoryDnaCheckpointEvidence`.
  - Add `run_factory_dna_checkpoint_curriculum`.
  - Add private helpers for digital/character sequence marks, primitive visual marker samples, primitive VIB low/high band samples, and cross-channel pair scoring.
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
  - Add `--factory-dna-checkpoint` flag.
  - Emit numeric evidence lines only.
- Optional modify: `bionic_v0/src/evolution_priors.rs`
  - Add generic VIB band-center helper only if the curriculum cannot compute high/low separation locally without duplication.
  - Do not add domain-specific fields such as `digit_bias`, `character_bias`, `visual_marker_bias`, or `speech_bias`.

## Chunk 1: RED Tests

### Task 1: Factory Checkpoint Behavior Test

**Files:**
- Create: `bionic_v0/tests/factory_dna_checkpoint.rs`
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Write the failing test**

Create `bionic_v0/tests/factory_dna_checkpoint.rs`:

```rust
use bionic_v0::curriculum::{run_factory_dna_checkpoint_curriculum, CurriculumConfig};

#[test]
fn factory_dna_checkpoint_proves_birth_structure_pitch_visual_and_cross_channel_readiness() {
    let evidence =
        run_factory_dna_checkpoint_curriculum(CurriculumConfig::default()).expect("course runs");

    assert_eq!(evidence.dna_checkpoint_pass, 1);
    assert!(evidence.digital_structure_events >= 12);
    assert!(evidence.character_digit_mark_traces >= 6);
    assert!(evidence.visual_marker_events >= 8);
    assert!(evidence.acoustic_low_band_events >= 4);
    assert!(evidence.acoustic_high_band_events >= 4);
    assert!(evidence.pitch_band_separation_margin > 0.20);
    assert!(evidence.cross_channel_pairs >= 8);
    assert!(evidence.cross_channel_coupling_strength > 0.0);
    assert!(evidence.conflict_ambiguity_delta > 0.0);
    assert!(evidence.wrong_binding_strength <= 0.01);
    assert!(evidence.hold_retention_ratio >= 0.80);
    assert!(evidence.reentry_persistent_strength > evidence.reentry_cold_strength);
    assert_eq!(evidence.no_fixed_ontology_guard, 1);
}
```

- [ ] **Step 2: Run test to verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test factory_dna_checkpoint factory_dna_checkpoint_proves_birth_structure_pitch_visual_and_cross_channel_readiness
```

Expected: FAIL because `run_factory_dna_checkpoint_curriculum` and `FactoryDnaCheckpointEvidence` do not exist.

### Task 2: Factory Checkpoint Report Test

**Files:**
- Modify: `bionic_v0/tests/curriculum_report.rs`
- Modify: `bionic_v0/src/bin/curriculum_report.rs`

- [ ] **Step 1: Add failing CLI output test**

Add a test near the existing curriculum report tests:

```rust
#[test]
fn factory_dna_checkpoint_report_emits_numeric_evidence() {
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_curriculum_report"))
        .arg("--factory-dna-checkpoint")
        .output()
        .expect("run curriculum_report");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("utf8 stdout");
    assert!(stdout.contains("mode=factory_dna_checkpoint"));
    assert!(stdout.contains("digital_structure_events="));
    assert!(stdout.contains("character_digit_mark_traces="));
    assert!(stdout.contains("visual_marker_events="));
    assert!(stdout.contains("acoustic_low_band_events="));
    assert!(stdout.contains("acoustic_high_band_events="));
    assert!(stdout.contains("pitch_band_separation_margin="));
    assert!(stdout.contains("cross_channel_pairs="));
    assert!(stdout.contains("cross_channel_coupling_strength="));
    assert!(stdout.contains("hold_retention_ratio="));
    assert!(stdout.contains("reentry_persistent_strength="));
    assert!(stdout.contains("reentry_cold_strength="));
    assert!(stdout.contains("dna_checkpoint_pass="));
}
```

- [ ] **Step 2: Run report test to verify RED**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report factory_dna_checkpoint_report_emits_numeric_evidence
```

Expected: FAIL because the flag/report branch is missing.

## Chunk 2: Curriculum Evidence And Runner

### Task 3: Add Evidence Struct

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Add evidence type near existing evidence structs**

Add:

```rust
#[derive(Clone, Debug)]
pub struct FactoryDnaCheckpointEvidence {
    pub digital_structure_events: u32,
    pub character_digit_mark_traces: u32,
    pub visual_marker_events: u32,
    pub acoustic_low_band_events: u32,
    pub acoustic_high_band_events: u32,
    pub pitch_band_separation_margin: f64,
    pub cross_channel_pairs: u32,
    pub cross_channel_coupling_strength: f64,
    pub conflict_ambiguity_delta: f64,
    pub wrong_binding_strength: f64,
    pub hold_retention_ratio: f64,
    pub reentry_persistent_strength: f64,
    pub reentry_cold_strength: f64,
    pub no_fixed_ontology_guard: u8,
    pub dna_checkpoint_pass: u8,
}
```

- [ ] **Step 2: Run behavior test**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test factory_dna_checkpoint factory_dna_checkpoint_proves_birth_structure_pitch_visual_and_cross_channel_readiness
```

Expected: still FAIL because the runner is missing.

### Task 4: Implement Factory Runner

**Files:**
- Modify: `bionic_v0/src/curriculum.rs`

- [ ] **Step 1: Add runner skeleton**

Add:

```rust
pub fn run_factory_dna_checkpoint_curriculum(
    config: CurriculumConfig,
) -> std::io::Result<FactoryDnaCheckpointEvidence> {
    const STREAM_ID: u64 = 101;
    const HOLD_US: u64 = 2_400_000;
    let step_us = config.step_us.max(1);
    let vp = Arc::new(Viewport::new());

    // Implementation filled in following steps.
}
```

- [ ] **Step 2: Allocate substrate targets**

Inside the runner:

1. Allocate at least three live object slots using existing `Viewport::allocate_slot`.
2. Form at least two relation targets using existing relation helper patterns in `curriculum.rs`.
3. Define at least two time-window targets with `BindingTargetRef::TimeWindow`.
4. Keep targets numeric and substrate-backed.

Expected target mix:

```rust
let object_targets = [
    BindingTargetRef::ObjectSlot { slot_id: slot_a as u8 },
    BindingTargetRef::ObjectSlot { slot_id: slot_b as u8 },
    BindingTargetRef::ObjectSlot { slot_id: slot_c as u8 },
];
let time_target = BindingTargetRef::TimeWindow {
    start_t_us: config.start_t_us,
    end_t_us: config.start_t_us + step_us * 3,
};
```

- [ ] **Step 3: Drive digital/character structure disturbances**

Use byte-mark disturbances as external marks. Do not tokenize. Use at least six distinct byte carriers representing external structure, for example:

```rust
const STRUCTURE_MARKS: [&[u8]; 6] = [
    b"0-1-2",
    b"2-1-0",
    b"a-a-a",
    b"a-b-a",
    b"[]",
    b"[x]",
];
```

For two episodes per mark:

1. Call `vp.observe_mark_bytes`.
2. Bind the latest mark to an object, relation, or time-window target.
3. Count `digital_structure_events`.

The byte content is observer-side material. The body only receives hashes, recurrence, stability, ambiguity, and binding events.

- [ ] **Step 4: Drive primitive visual marker samples**

Use existing `write_phot_spatial_sample` or add a local helper modeled after it. Drive at least eight PHOT samples with boundary-like changes:

```rust
for episode in 0..8u32 {
    let t_us = config.start_t_us + 500_000 + episode as u64 * step_us;
    write_phot_spatial_sample(&vp, slot_a, t_us, 0.18 + episode as f64 * 0.03, episode);
    visual_marker_events += 1;
}
```

This proves sensitivity to stable/moving visual markers, not recognition of named shapes.

- [ ] **Step 5: Drive primitive acoustic low/high band samples**

Add a private helper if needed:

```rust
fn write_vib_band_shape_sample(
    vp: &Viewport,
    slot_id: usize,
    t_us: u64,
    low_energy: f64,
    high_energy: f64,
) {
    let observed = [low_energy, low_energy * 0.8, high_energy * 0.8, high_energy];
    let predicted = observed.map(|x| x * 0.75);
    let err = observed.map(|x| x * 0.25);
    let surprise = observed.map(|x| 0.20 + x * 0.40);
    vp.write_slot_vib(slot_id, t_us, &observed, &predicted, &err, &surprise);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
    vp.refresh_slot_high_layer(slot_id, t_us);
}
```

Drive at least four low-heavy samples and four high-heavy samples. Compute:

```rust
fn vib_band_center(observed: &[f64; VIB_BANDS]) -> f64 {
    let mut weighted = 0.0;
    let mut total = 0.0;
    for (idx, value) in observed.iter().enumerate() {
        weighted += idx as f64 * value.max(0.0);
        total += value.max(0.0);
    }
    if total > 0.0 { weighted / total } else { 0.0 }
}
```

Set `pitch_band_separation_margin` to the high-center average minus the low-center average.

- [ ] **Step 6: Drive cross-channel bindings**

For each structure mark, create VIB and PHOT carrier hashes using existing `carrier_pattern_hash` and bind them to the same target:

```rust
let vib_hash = carrier_pattern_hash(mark_hash, target, 0x7100 + idx as u64);
let phot_hash = carrier_pattern_hash(mark_hash, target, 0x8100 + idx as u64);
apply_pattern_binding(&vp, PatternKind::VibModality, vib_hash, target, BindingFeedback::Align, t_us);
apply_pattern_binding(&vp, PatternKind::PhotModality, phot_hash, target, BindingFeedback::Align, t_us);
```

Compute `cross_channel_coupling_strength` with existing pair-coupling helpers. Count at least eight cross-channel pairs.

- [ ] **Step 7: Add conflict, hold, and reentry**

1. Present one previously supported mark against a wrong target with `BindingFeedback::Deviate`.
2. Measure `conflict_ambiguity_delta` and `wrong_binding_strength`.
3. Call `vp.tick_binding_maintenance(hold_t_us)`.
4. Measure `hold_retention_ratio`.
5. Re-present supported marks on the persistent viewport.
6. Re-present only the same reentry marks on a fresh cold viewport.
7. Measure `reentry_persistent_strength` and `reentry_cold_strength`.

- [ ] **Step 8: Add pass and guard fields**

Set:

```rust
let no_fixed_ontology_guard = factory_dna_source_guard();
let dna_checkpoint_pass = bool_flag(
    digital_structure_events >= 12
        && character_digit_mark_traces >= 6
        && visual_marker_events >= 8
        && acoustic_low_band_events >= 4
        && acoustic_high_band_events >= 4
        && pitch_band_separation_margin > 0.20
        && cross_channel_pairs >= 8
        && cross_channel_coupling_strength > 0.0
        && conflict_ambiguity_delta > 0.0
        && wrong_binding_strength <= 0.01
        && hold_retention_ratio >= 0.80
        && reentry_persistent_strength > reentry_cold_strength
        && no_fixed_ontology_guard == 1,
);
```

If `bool_flag` does not exist, add a private helper:

```rust
fn bool_flag(value: bool) -> u8 {
    if value { 1 } else { 0 }
}
```

- [ ] **Step 9: Verify behavior GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test factory_dna_checkpoint
```

Expected: PASS.

## Chunk 3: Observer Report

### Task 5: Add Report Mode

**Files:**
- Modify: `bionic_v0/src/bin/curriculum_report.rs`
- Modify: `bionic_v0/tests/curriculum_report.rs`

- [ ] **Step 1: Import runner**

Add `run_factory_dna_checkpoint_curriculum` to the existing curriculum imports.

- [ ] **Step 2: Add flag branch**

Add a `--factory-dna-checkpoint` branch that prints:

```rust
let evidence = run_factory_dna_checkpoint_curriculum(CurriculumConfig::default())?;
println!("curriculum_report_v0");
println!("mode=factory_dna_checkpoint");
println!("digital_structure_events={}", evidence.digital_structure_events);
println!("character_digit_mark_traces={}", evidence.character_digit_mark_traces);
println!("visual_marker_events={}", evidence.visual_marker_events);
println!("acoustic_low_band_events={}", evidence.acoustic_low_band_events);
println!("acoustic_high_band_events={}", evidence.acoustic_high_band_events);
println!(
    "pitch_band_separation_margin={:.6}",
    evidence.pitch_band_separation_margin
);
println!("cross_channel_pairs={}", evidence.cross_channel_pairs);
println!(
    "cross_channel_coupling_strength={:.6}",
    evidence.cross_channel_coupling_strength
);
println!("conflict_ambiguity_delta={:.6}", evidence.conflict_ambiguity_delta);
println!("wrong_binding_strength={:.6}", evidence.wrong_binding_strength);
println!("hold_retention_ratio={:.6}", evidence.hold_retention_ratio);
println!(
    "reentry_persistent_strength={:.6}",
    evidence.reentry_persistent_strength
);
println!("reentry_cold_strength={:.6}", evidence.reentry_cold_strength);
println!("no_fixed_ontology_guard={}", evidence.no_fixed_ontology_guard);
println!("dna_checkpoint_pass={}", evidence.dna_checkpoint_pass);
```

- [ ] **Step 3: Verify report GREEN**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report factory_dna_checkpoint_report_emits_numeric_evidence
```

Expected: PASS.

## Chunk 4: Hygiene And Verification

### Task 6: Rejected Architecture Guard

**Files:**
- Modify: `bionic_v0/tests/factory_dna_checkpoint.rs`

- [ ] **Step 1: Add hygiene test**

Add:

```rust
#[test]
fn factory_dna_checkpoint_sources_do_not_introduce_fixed_ontology() {
    let sources = [
        include_str!("../src/curriculum.rs"),
        include_str!("../src/evolution_priors.rs"),
        include_str!("../src/mark_disturbance.rs"),
        include_str!("../src/language_binding.rs"),
        include_str!("../src/bin/curriculum_report.rs"),
    ];

    for src in sources {
        for forbidden in [
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
            "phoneme_inventory",
            "visual_label",
            "speech_recognizer",
            "digit_meaning",
            "character_meaning",
            "eprintln!",
        ] {
            assert!(
                !src.contains(forbidden),
                "factory DNA checkpoint path must not contain `{}`",
                forbidden
            );
        }
    }
}
```

- [ ] **Step 2: Run focused tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test factory_dna_checkpoint
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --test curriculum_report factory_dna_checkpoint_report_emits_numeric_evidence
```

Expected: PASS.

### Task 7: Final Verification

**Files:**
- Verify all touched files.

- [ ] **Step 1: Run observer report manually**

Run:

```bash
cargo run --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --bin curriculum_report -- --factory-dna-checkpoint
```

Expected: compact numeric output with `dna_checkpoint_pass=1`.

- [ ] **Step 2: Format/check**

Run:

```bash
cargo fmt --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml --check
git diff --check
```

Expected: both pass.

- [ ] **Step 3: Run full crate tests**

Run:

```bash
cargo test --manifest-path /Users/joe/unnamed/bionic_v0/Cargo.toml
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add /Users/joe/unnamed/bionic_v0/src/curriculum.rs \
  /Users/joe/unnamed/bionic_v0/src/bin/curriculum_report.rs \
  /Users/joe/unnamed/bionic_v0/tests/factory_dna_checkpoint.rs \
  /Users/joe/unnamed/bionic_v0/tests/curriculum_report.rs
git commit -m "feat: add factory dna checkpoint curriculum"
```
