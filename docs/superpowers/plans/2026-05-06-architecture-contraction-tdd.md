# Architecture Contraction TDD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Contract `bionic_v0` back toward the silicon-life substrate by removing or downgrading program-like ontology, fixed feature taxonomies, command-style triggers, and misleading completion documentation.

**Architecture:** Preserve the current perception, viewport, object-slot, relation, snapshot, observer, action-output, consequence, and vocal substrate. Replace domain-specific “numeric-only” feature taxonomies with generic curve-pressure samples, collapse discrete action triggers into one continuous drive path, quarantine orphan motivational code, and strengthen regression tests so future work cannot re-add the same pattern under new names.

**Tech Stack:** Rust crate `bionic_v0`, Cargo integration tests, existing `Viewport`, `snapshot`, `observer`, and docs under `docs/superpowers`.

---

## Boundary Rules

This is a contraction plan. Do not add new body organs, new world categories, new identity/source/trust/personality layers, shell/SSH carriers, or language features while executing it.

All behavior changes must follow TDD:

1. Write the failing test first.
2. Run the targeted command and verify the expected failure.
3. Implement the smallest production-code change.
4. Run the targeted test until it passes.
5. Run the relevant local test group.
6. Commit that task before starting the next task.

Full verification command:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

High-risk terms in this plan are external review terms. They must not become body-side ontology fields.

## File Map

- Modify: `bionic_v0/tests/cleanup_hygiene.rs`
  Add active-source hygiene checks for orphan motivational systems and new disguised ontology.
- Modify: `bionic_v0/tests/body_no_human_output.rs`
  Expand body human-output hygiene beyond `println!` / `eprintln!`.
- Create: `bionic_v0/tests/architecture_contraction.rs`
  Cross-module contraction tests for priors, triggers, adaptive stability, code ingress projections, and docs.
- Move: `bionic_v0/src/intention.rs` -> `bionic_v0/experimental/intention.rs.quarantine`
  Quarantine orphan intent system so it cannot be wired back without an explicit architecture review.
- Modify: `bionic_v0/src/bin/body.rs`
  Remove panic text paths and remove direct calls to discrete probe trigger functions.
- Modify: `bionic_v0/src/vib.rs`
  Remove human-readable body-side error string.
- Create: `bionic_v0/src/substrate_pressure.rs`
  Generic curve-pressure primitives replacing domain-specific feature taxonomies.
- Modify: `bionic_v0/src/evolution_priors.rs`
  Retain only generic numeric pressure weights and modification caps.
- Modify: `bionic_v0/src/adapter.rs`
  Replace visual/acoustic/spatial named primitive feature calls with generic pressure samples.
- Modify: `bionic_v0/src/action_consequence.rs`
  Replace named action-consequence retention features with generic support/deviation/temporal pressure.
- Modify: `bionic_v0/src/evidence_tension.rs`
  Replace epistemic/source-reliability feature names with generic unresolved-pressure samples.
- Modify: `bionic_v0/src/language_binding.rs`
  Replace communication/cross-modal named pressure feature calls with generic pressure samples.
- Modify: `bionic_v0/src/learned_bias.rs`
  Replace learned-bias axes that encode world domains with generic pressure axes.
- Modify: `bionic_v0/src/life_closure_producers.rs`
  Emit learned-bias events against generic axes and numeric locators only.
- Modify: `bionic_v0/src/probe_law.rs`
  Collapse `tick_surprise_trigger`, `tick_attention_trigger`, and `tick_relation_instability_trigger` into the single probe-law drive path.
- Modify: `bionic_v0/src/adaptive_stability.rs`
  Remove named route kinds and named repair phases; keep route-local numeric dynamics.
- Modify: `bionic_v0/src/code_ingress.rs`
  Remove ASCII/glyph projection as a pre-decoded body-side projection.
- Modify: `bionic_v0/src/snapshot.rs`
  Roundtrip renamed/generic structures and current snapshot version.
- Modify: `bionic_v0/src/bin/observer.rs`
  Render renamed numeric fields without reintroducing ontology labels.
- Modify: `bionic_v0/src/lib.rs`
  Export `substrate_pressure`, update constants, and keep snapshot version coherent.
- Modify: `docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md`
  Bring completion map and snapshot version in line with code and mark contraction boundaries.

## Task 1: Quarantine Orphan Intention System

**Files:**
- Modify: `bionic_v0/tests/cleanup_hygiene.rs`
- Move: `bionic_v0/src/intention.rs` -> `bionic_v0/experimental/intention.rs.quarantine`

- [ ] **Step 1: Write the failing test**

Append this test to `bionic_v0/tests/cleanup_hygiene.rs`:

```rust
#[test]
fn active_source_contains_no_orphan_intention_system() {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
    assert!(
        !std::path::Path::new(&format!("{}/intention.rs", root)).exists(),
        "src/intention.rs must not exist in active body source; quarantine it under experimental"
    );

    let lib_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/lib.rs"))
        .expect("read lib.rs");
    assert!(
        !lib_src.contains("pub mod intention") && !lib_src.contains("mod intention"),
        "lib.rs must not expose an intention module"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test cleanup_hygiene active_source_contains_no_orphan_intention_system -- --nocapture
```

Expected: FAIL because `bionic_v0/src/intention.rs` exists.

- [ ] **Step 3: Quarantine the orphan source**

Run:

```bash
git mv bionic_v0/src/intention.rs bionic_v0/experimental/intention.rs.quarantine
```

Add this note to `bionic_v0/experimental/README.md`:

```markdown
## `intention.rs.quarantine`

Quarantined on 2026-05-06 by the architecture contraction plan.
Reason: fixed `INTENT_0..3`, `IntentionBank`, vitality gates, and goal-directed
state are program-like motivational ontology. This file is archaeological
reference only and must not be wired into `body`, `viewport`, `snapshot`, or
observer.
```

- [ ] **Step 4: Verify targeted test passes**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test cleanup_hygiene active_source_contains_no_orphan_intention_system -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add bionic_v0/tests/cleanup_hygiene.rs bionic_v0/experimental/README.md bionic_v0/experimental/intention.rs.quarantine
git commit -m "refactor: quarantine orphan intention system"
```

## Task 2: Harden Body Human-Output Hygiene

**Files:**
- Modify: `bionic_v0/tests/body_no_human_output.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/vib.rs`

- [ ] **Step 1: Write the failing test**

Replace the contents of `bionic_v0/tests/body_no_human_output.rs` with:

```rust
//! Cleanup spec 2026-04-20 §4.5: body-owned code must not emit or embed
//! human-readable diagnostics. Visible output belongs to observer processes.

#[test]
fn body_owned_sources_contain_no_human_readable_output_paths() {
    let manifest = env!("CARGO_MANIFEST_DIR");
    let body_owned = [
        "src/bin/body.rs",
        "src/vib.rs",
        "src/adapter.rs",
        "src/vocal_adapter.rs",
    ];
    let forbidden = [
        "println!",
        "eprintln!",
        "dbg!",
        "panic!(\"",
        "expect(\"",
        "format!(\"",
    ];

    for rel in body_owned {
        let path = std::path::Path::new(manifest).join(rel);
        let src = std::fs::read_to_string(&path).expect("read body-owned source");
        for needle in forbidden {
            assert!(
                !src.contains(needle),
                "{} must not contain body-side human-readable output path `{}`",
                rel,
                needle
            );
        }
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test body_no_human_output -- --nocapture
```

Expected: FAIL on `expect("register vib")`, `expect("register phot")`, or `format!("unsupported audio sample format...")`.

- [ ] **Step 3: Remove body-side human strings**

In `bionic_v0/src/bin/body.rs`, replace:

```rust
reg.register(Box::new(VibAdapter::new(&vp_reg)))
    .expect("register vib");
```

with:

```rust
let _ = reg.register(Box::new(VibAdapter::new(&vp_reg)));
```

Replace:

```rust
reg.register(Box::new(RealPhotAdapter::with_birth_sensitivity(
    cam,
    vp_reg.birth_sensitivity(),
)))
.expect("register phot");
```

with:

```rust
let _ = reg.register(Box::new(RealPhotAdapter::with_birth_sensitivity(
    cam,
    vp_reg.birth_sensitivity(),
)));
```

In `bionic_v0/src/vib.rs`, replace:

```rust
_ => Err(format!("unsupported audio sample format: {:?}", fmt).into()),
```

with:

```rust
_ => Err(Box::new(std::io::Error::from(std::io::ErrorKind::Unsupported))),
```

- [ ] **Step 4: Verify targeted test passes**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test body_no_human_output -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Run body-adjacent tests**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test vocal_body_contract --test ordered_global_refresh -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add bionic_v0/tests/body_no_human_output.rs bionic_v0/src/bin/body.rs bionic_v0/src/vib.rs
git commit -m "test: harden body output hygiene"
```

## Task 3: Add Architecture Contraction Guard Tests

**Files:**
- Create: `bionic_v0/tests/architecture_contraction.rs`

- [ ] **Step 1: Write failing guard tests**

Create `bionic_v0/tests/architecture_contraction.rs`:

```rust
fn read_src(rel: &str) -> String {
    std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|_| panic!("read {}", rel))
}

#[test]
fn evolution_priors_must_not_define_world_domain_bias_axes() {
    let src = read_src("src/evolution_priors.rs");
    for forbidden in [
        "CodeStream",
        "DecoderProjection",
        "VisualPrimitive",
        "AcousticPrimitive",
        "ActionConsequence",
        "SpatialDistance",
        "CommunicationLoop",
        "CrossModalBinding",
        "StructuralTearing",
        "IrreversibleDisruption",
        "EvidenceOverwritePrevention",
        "CodeNativeFeatures",
        "DigitalStructureFeatures",
        "GeneralDisruptionFeatures",
        "VisualPrimitiveFeatures",
        "AcousticPrimitiveFeatures",
        "SpatialDistanceFeatures",
        "ActionConsequenceFeatures",
        "CommunicationLoopFeatures",
        "EpistemicTensionFeatures",
        "CrossModalBindingFeatures",
    ] {
        assert!(
            !src.contains(forbidden),
            "evolution_priors.rs must not encode world-domain ontology `{}` as numeric-only bias",
            forbidden
        );
    }
}

#[test]
fn probe_law_must_have_single_output_drive_path() {
    let probe = read_src("src/probe_law.rs");
    let body = read_src("src/bin/body.rs");
    for forbidden in [
        "tick_surprise_trigger",
        "tick_attention_trigger",
        "tick_relation_instability_trigger",
    ] {
        assert!(
            !probe.contains(forbidden) && !body.contains(forbidden),
            "probe output must flow through one continuous drive path; remove `{}`",
            forbidden
        );
    }
}

#[test]
fn adaptive_stability_must_not_encode_named_repair_routes_or_phases() {
    let src = read_src("src/adaptive_stability.rs");
    for forbidden in [
        "AdaptiveRouteKind",
        "Attention",
        "ActionConsequence",
        "VocalPrototype",
        "Bypassing",
        "Relearning",
        "FastPathActive",
    ] {
        assert!(
            !src.contains(forbidden),
            "adaptive stability must be route-local numeric dynamics, not named repair ontology `{}`",
            forbidden
        );
    }
}

#[test]
fn code_ingress_must_not_predecode_ascii_or_glyph_projection_in_body() {
    let src = read_src("src/code_ingress.rs");
    for forbidden in [
        "CODE_DECODER_ASCII_SHAPE",
        "ASCII_SHAPE",
        "ascii",
        "glyph",
        "Glyph",
    ] {
        assert!(
            !src.contains(forbidden),
            "code ingress must not install pre-decoded text/glyph projection `{}` body-side",
            forbidden
        );
    }
}

#[test]
fn global_architecture_map_matches_current_snapshot_version() {
    let lib = read_src("src/lib.rs");
    let docs = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md"),
    )
    .expect("read global architecture map");

    let version_line = lib
        .lines()
        .find(|line| line.contains("pub const SNAPSHOT_VERSION"))
        .expect("snapshot version constant");
    let version = version_line
        .split('=')
        .nth(1)
        .expect("version rhs")
        .trim()
        .trim_end_matches(';');

    assert!(
        docs.contains(&format!("snapshot v{}", version)),
        "architecture map must mention current snapshot v{}",
        version
    );
    assert!(
        !docs.contains("snapshot v41"),
        "architecture map still contains stale snapshot v41"
    );
}
```

- [ ] **Step 2: Run tests to verify expected failures**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test architecture_contraction -- --nocapture
```

Expected: FAIL on current domain-specific priors, probe trigger functions, adaptive route names, ASCII projection, and stale snapshot documentation.

- [ ] **Step 3: Commit the red tests**

```bash
git add bionic_v0/tests/architecture_contraction.rs
git commit -m "test: add architecture contraction guards"
```

These tests intentionally remain red until Tasks 4-10 complete.

## Task 4: Introduce Generic Substrate Pressure Primitives

**Files:**
- Create: `bionic_v0/src/substrate_pressure.rs`
- Modify: `bionic_v0/src/lib.rs`
- Test: `bionic_v0/tests/substrate_pressure.rs`

- [ ] **Step 1: Write the failing tests**

Create `bionic_v0/tests/substrate_pressure.rs`:

```rust
use bionic_v0::substrate_pressure::{
    compute_pressure, PressureSample, PressureWeights,
};

#[test]
fn generic_pressure_rises_with_error_ambiguity_and_deviation() {
    let weights = PressureWeights::default();
    let quiet = compute_pressure(
        PressureSample {
            err: 0.05,
            ambiguity: 0.05,
            deviation: 0.05,
            ..Default::default()
        },
        &weights,
    );
    let active = compute_pressure(
        PressureSample {
            err: 0.8,
            ambiguity: 0.6,
            deviation: 0.7,
            ..Default::default()
        },
        &weights,
    );
    assert!(active > quiet);
}

#[test]
fn generic_pressure_is_bounded_for_extreme_inputs() {
    let pressure = compute_pressure(
        PressureSample {
            err: 99.0,
            recurrence: 99.0,
            ambiguity: 99.0,
            alignment: 99.0,
            support: 99.0,
            deviation: 99.0,
            temporal_gap: 99.0,
            cross_channel: 99.0,
        },
        &PressureWeights::default(),
    );
    assert!((0.0..=2.0).contains(&pressure));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test substrate_pressure -- --nocapture
```

Expected: FAIL because `substrate_pressure` module does not exist.

- [ ] **Step 3: Implement generic pressure module**

Create `bionic_v0/src/substrate_pressure.rs`:

```rust
fn unit(x: f64) -> f64 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PressureSample {
    pub err: f64,
    pub recurrence: f64,
    pub ambiguity: f64,
    pub alignment: f64,
    pub support: f64,
    pub deviation: f64,
    pub temporal_gap: f64,
    pub cross_channel: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PressureWeights {
    pub err: f64,
    pub recurrence: f64,
    pub ambiguity: f64,
    pub alignment: f64,
    pub support: f64,
    pub deviation: f64,
    pub temporal_gap: f64,
    pub cross_channel: f64,
    pub multiplier: f64,
}

impl Default for PressureWeights {
    fn default() -> Self {
        Self {
            err: 0.20,
            recurrence: 0.10,
            ambiguity: 0.15,
            alignment: 0.15,
            support: 0.10,
            deviation: 0.15,
            temporal_gap: 0.10,
            cross_channel: 0.05,
            multiplier: 1.0,
        }
    }
}

pub fn compute_pressure(sample: PressureSample, weights: &PressureWeights) -> f64 {
    let weighted = unit(sample.err) * weights.err
        + unit(sample.recurrence) * weights.recurrence
        + unit(sample.ambiguity) * weights.ambiguity
        + unit(sample.alignment) * weights.alignment
        + unit(sample.support) * weights.support
        + unit(sample.deviation) * weights.deviation
        + unit(sample.temporal_gap) * weights.temporal_gap
        + unit(sample.cross_channel) * weights.cross_channel;
    (weighted * weights.multiplier.max(0.0)).clamp(0.0, 2.0)
}
```

Add to `bionic_v0/src/lib.rs` near the other modules:

```rust
pub mod substrate_pressure;
```

- [ ] **Step 4: Verify targeted tests pass**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test substrate_pressure -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add bionic_v0/src/substrate_pressure.rs bionic_v0/src/lib.rs bionic_v0/tests/substrate_pressure.rs
git commit -m "feat: add generic substrate pressure primitives"
```

## Task 5: Replace Domain-Specific Birth Priors With Generic Weights

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`
- Modify: `bionic_v0/src/adapter.rs`
- Modify: `bionic_v0/src/action_consequence.rs`
- Modify: `bionic_v0/src/evidence_tension.rs`
- Modify: `bionic_v0/src/language_binding.rs`
- Modify: `bionic_v0/tests/innate_bias_runtime_callers.rs`
- Modify: `bionic_v0/tests/evidence_tension_active_verification.rs`
- Modify: `bionic_v0/tests/language_binding_runtime.rs`
- Modify: `bionic_v0/tests/action_consequence_loop.rs`

- [ ] **Step 1: Write failing caller-level tests**

In `bionic_v0/tests/innate_bias_runtime_callers.rs`, add:

```rust
#[test]
fn birth_sensitivity_uses_generic_pressure_weights_not_domain_taxonomy() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/evolution_priors.rs"
    ))
    .expect("read evolution_priors.rs");
    for forbidden in [
        "code_stream_attention_mul",
        "visual_attention_mul",
        "acoustic_attention_mul",
        "spatial_attention_mul",
        "communication_retention_mul",
        "epistemic_tension_mul",
        "cross_modal_binding_mul",
        "compute_visual_primitive_attention",
        "compute_acoustic_primitive_attention",
        "compute_epistemic_tension",
    ] {
        assert!(
            !src.contains(forbidden),
            "birth sensitivity must use generic pressure weights, not `{}`",
            forbidden
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test innate_bias_runtime_callers birth_sensitivity_uses_generic_pressure_weights_not_domain_taxonomy -- --nocapture
```

Expected: FAIL because current `BiasPriors` has domain-specific multipliers and compute functions.

- [ ] **Step 3: Replace `BiasPriors` shape**

In `bionic_v0/src/evolution_priors.rs`, reduce `BiasAxis` to generic update axes:

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiasAxis {
    ErrGain,
    RecurrenceGain,
    AmbiguityGain,
    AlignmentGain,
    SupportGain,
    DeviationGain,
    TemporalGapGain,
    CrossChannelGain,
    PressureMultiplier,
}
```

Replace `BiasPriors` fields with:

```rust
#[derive(Clone, Debug)]
pub struct BiasPriors {
    pub band_sensitivity: [f64; crate::VIB_BANDS],
    pub pressure_weights: crate::substrate_pressure::PressureWeights,
    pub modify_rate_caps: ModifyRateCaps,
}
```

Update `Default` so it uses `PressureWeights::default()` and keeps `band_sensitivity: [0.5; VIB_BANDS]`.

Keep `compute_disruption_score(err, priors)` only as a generic band-error helper. It may read `band_sensitivity`; it must not mention disruption, tearing, or irreversible structure.

- [ ] **Step 4: Migrate callers**

Replace calls as follows:

In `bionic_v0/src/adapter.rs`, replace visual/acoustic/spatial primitive calls with:

```rust
let pressure = crate::substrate_pressure::compute_pressure(
    crate::substrate_pressure::PressureSample {
        err: base_inst_surprise,
        deviation: spread,
        temporal_gap: max_span,
        cross_channel: if regions.len() > 1 { spread } else { 0.0 },
        ..Default::default()
    },
    &self.birth_sensitivity.pressure_weights,
);
let inst_surprise = base_inst_surprise * (1.0 + 0.25 * pressure);
```

For VIB, replace acoustic primitive pressure with:

```rust
let acoustic_pressure = crate::substrate_pressure::compute_pressure(
    crate::substrate_pressure::PressureSample {
        err: max_err,
        deviation: (max_err - min_err).abs(),
        cross_channel: broadband,
        ..Default::default()
    },
    &self.priors.pressure_weights,
);
inst_surprise *= 1.0 + 0.25 * acoustic_pressure;
```

In `bionic_v0/src/action_consequence.rs`, replace `compute_action_consequence_retention(...)` with:

```rust
crate::substrate_pressure::compute_pressure(
    crate::substrate_pressure::PressureSample {
        alignment: pair.curve_similarity,
        support: if pair.status == ConsequencePairStatus::Paired { 1.0 } else { 0.0 },
        deviation: 1.0 - pair.curve_similarity,
        temporal_gap: 1.0 - delay_stability,
        recurrence: pair.curve_similarity,
        ..Default::default()
    },
    &birth_sensitivity.pressure_weights,
)
```

In `bionic_v0/src/evidence_tension.rs`, replace `EpistemicTensionFeatures` with `PressureSample`.

In `bionic_v0/src/language_binding.rs`, replace communication and cross-modal feature calls with one `PressureSample` using `recurrence`, `alignment`, `support`, `deviation`, and `cross_channel`.

- [ ] **Step 5: Verify targeted tests pass**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test innate_bias_runtime_callers --test evidence_tension_active_verification --test language_binding_runtime --test action_consequence_loop -- --nocapture
```

Expected: PASS after updating test imports and assertions to generic pressure names.

- [ ] **Step 6: Verify contraction guard for priors passes**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test architecture_contraction evolution_priors_must_not_define_world_domain_bias_axes -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add bionic_v0/src/evolution_priors.rs bionic_v0/src/adapter.rs bionic_v0/src/action_consequence.rs bionic_v0/src/evidence_tension.rs bionic_v0/src/language_binding.rs bionic_v0/tests/innate_bias_runtime_callers.rs bionic_v0/tests/evidence_tension_active_verification.rs bionic_v0/tests/language_binding_runtime.rs bionic_v0/tests/action_consequence_loop.rs
git commit -m "refactor: replace domain priors with generic pressure"
```

## Task 6: Convert Learned Bias To Generic Axes

**Files:**
- Modify: `bionic_v0/src/learned_bias.rs`
- Modify: `bionic_v0/src/life_closure_producers.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/tests/learned_bias.rs`
- Modify: `bionic_v0/tests/life_closure_hardening.rs`

- [ ] **Step 1: Write failing tests**

In `bionic_v0/tests/learned_bias.rs`, add:

```rust
#[test]
fn learned_bias_axes_are_generic_pressure_axes_only() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/learned_bias.rs"
    ))
    .expect("read learned_bias.rs");
    for forbidden in [
        "CodeStream",
        "ActionConsequence",
        "VisualPrimitive",
        "CommunicationLoop",
        "CrossModalBinding",
        "StructuralTearing",
        "IrreversibleDisruption",
        "EvidenceOverwritePrevention",
    ] {
        assert!(
            !src.contains(forbidden),
            "learned bias must not sediment domain axis `{}`",
            forbidden
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test learned_bias learned_bias_axes_are_generic_pressure_axes_only -- --nocapture
```

Expected: FAIL because `learned_bias.rs` still serializes domain `BiasAxis` variants.

- [ ] **Step 3: Update learned-bias events**

Keep `LearnedBiasEventInput.axis: BiasAxis`, but after Task 5 `BiasAxis` is generic. Update axis serialization in `axis_to_u64` / `axis_from_u64` to match:

```rust
BiasAxis::ErrGain => 0,
BiasAxis::RecurrenceGain => 1,
BiasAxis::AmbiguityGain => 2,
BiasAxis::AlignmentGain => 3,
BiasAxis::SupportGain => 4,
BiasAxis::DeviationGain => 5,
BiasAxis::TemporalGapGain => 6,
BiasAxis::CrossChannelGain => 7,
BiasAxis::PressureMultiplier => 8,
```

Update `bias_axis_is_protected` so only `PressureMultiplier` is protected:

```rust
pub fn bias_axis_is_protected(axis: BiasAxis) -> bool {
    matches!(axis, BiasAxis::PressureMultiplier)
}
```

In `bionic_v0/src/life_closure_producers.rs`, replace:

```rust
axis: BiasAxis::ActionConsequence,
```

with:

```rust
axis: BiasAxis::AlignmentGain,
```

Replace:

```rust
axis: BiasAxis::CodeStream,
```

with:

```rust
axis: BiasAxis::CrossChannelGain,
```

- [ ] **Step 4: Verify targeted tests pass**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test learned_bias --test life_closure_hardening -- --nocapture
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add bionic_v0/src/learned_bias.rs bionic_v0/src/life_closure_producers.rs bionic_v0/src/snapshot.rs bionic_v0/tests/learned_bias.rs bionic_v0/tests/life_closure_hardening.rs
git commit -m "refactor: sediment learned bias on generic axes"
```

## Task 7: Collapse Probe Output Triggers Into One Drive Path

**Files:**
- Modify: `bionic_v0/src/probe_law.rs`
- Modify: `bionic_v0/src/bin/body.rs`
- Modify: `bionic_v0/src/lib.rs`
- Modify: `bionic_v0/tests/probe_tick.rs`
- Modify: `bionic_v0/tests/live_io_preflight.rs`
- Modify: `bionic_v0/tests/live_probe_entry.rs`

- [ ] **Step 1: Write failing behavior test**

In `bionic_v0/tests/probe_tick.rs`, add:

```rust
#[test]
fn multiple_probe_pressures_emit_at_most_one_write_event_per_refresh_cycle() {
    let vp = Viewport::new();
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.global_surprise_load = 0.9;
        rs.routing_focus_slot_ids[0] = 2;
    }
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = true;
        gr.edges[0].pred.relation_surprise = 0.9;
    }
    {
        vp.probe_law.write().unwrap().last_top_slot_id = 1;
    }

    vp.refresh_probe_law(1_000_000);
    vp.tick_probe_output_from_law(1_000_000);

    let out = vp.read_action_output();
    assert_eq!(
        out.n_write_epochs, 1,
        "one probe refresh cycle must produce at most one write epoch"
    );
}
```

- [ ] **Step 2: Run test to verify current trigger architecture fails guard**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test architecture_contraction probe_law_must_have_single_output_drive_path -- --nocapture
```

Expected: FAIL because `tick_surprise_trigger`, `tick_attention_trigger`, and `tick_relation_instability_trigger` still exist.

- [ ] **Step 3: Move trigger pressure into `refresh_probe_law`**

In `bionic_v0/src/probe_law.rs`, inside `refresh_probe_law`, replace:

```rust
let novelty = self.read_global_reasoning_memory_summary().novelty_score;
let evidence_pressure = self.read_evidence_tension().unresolved_pressure;
let drive = novelty.max(evidence_pressure);
```

with:

```rust
let novelty = self.read_global_reasoning_memory_summary().novelty_score;
let evidence_pressure = self.read_evidence_tension().unresolved_pressure;
let reasoning = self.read_global_reasoning();
let surprise_pressure = reasoning.global_surprise_load;
let current_top = reasoning.routing_focus_slot_ids[0];
let relation_pressure = {
    let gr = self.global_relations.read().unwrap();
    gr.edges
        .iter()
        .filter(|e| e.active)
        .map(|e| e.pred.relation_surprise)
        .fold(0.0_f64, f64::max)
};
let slot_switch_pressure = {
    let prev = self.probe_law.read().unwrap().last_top_slot_id;
    if current_top >= 0 && prev != current_top {
        1.0
    } else {
        0.0
    }
};
let drive = novelty
    .max(evidence_pressure)
    .max(surprise_pressure)
    .max(relation_pressure)
    .max(slot_switch_pressure);
```

After updating the probe-law state, store `last_top_slot_id = current_top`.

Delete these functions:

```rust
tick_surprise_trigger
tick_attention_trigger
tick_relation_instability_trigger
```

In `bionic_v0/src/bin/body.rs`, delete calls to those functions from `run_global_refresh_cycle`.

In `bionic_v0/src/lib.rs`, remove unused trigger-threshold constants after tests are updated:

```rust
SURPRISE_TRIGGER_THRESHOLD
RELATION_INSTABILITY_TRIGGER_THRESHOLD
```

- [ ] **Step 4: Update tests to use continuous drive**

Replace tests that call deleted trigger functions with tests that call:

```rust
vp.refresh_probe_law(t);
vp.tick_probe_output_from_law(t);
```

Keep tests that assert no semantic fields in `probe_law.rs`.

- [ ] **Step 5: Verify targeted tests pass**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test probe_tick --test live_probe_entry --test live_io_preflight -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Verify contraction guard passes**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test architecture_contraction probe_law_must_have_single_output_drive_path -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add bionic_v0/src/probe_law.rs bionic_v0/src/bin/body.rs bionic_v0/src/lib.rs bionic_v0/tests/probe_tick.rs bionic_v0/tests/live_io_preflight.rs bionic_v0/tests/live_probe_entry.rs
git commit -m "refactor: collapse probe triggers into continuous drive"
```

## Task 8: Make Adaptive Stability Numeric Route Dynamics Only

**Files:**
- Modify: `bionic_v0/src/adaptive_stability.rs`
- Modify: `bionic_v0/src/life_closure_producers.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Modify: `bionic_v0/tests/adaptive_stability.rs`
- Modify: `bionic_v0/tests/life_closure_hardening.rs`

- [ ] **Step 1: Write failing test**

In `bionic_v0/tests/adaptive_stability.rs`, add:

```rust
#[test]
fn adaptive_stability_route_traces_use_numeric_codes_not_named_routes_or_phases() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/adaptive_stability.rs"
    ))
    .expect("read adaptive_stability.rs");
    for forbidden in [
        "AdaptiveRouteKind",
        "AdaptiveRoutePhase",
        "VocalPrototype",
        "Bypassing",
        "Relearning",
        "FastPathActive",
    ] {
        assert!(
            !src.contains(forbidden),
            "adaptive stability must not expose named route or repair phase `{}`",
            forbidden
        );
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test adaptive_stability adaptive_stability_route_traces_use_numeric_codes_not_named_routes_or_phases -- --nocapture
```

Expected: FAIL on current enums.

- [ ] **Step 3: Replace route kind and phase enums**

In `bionic_v0/src/adaptive_stability.rs`, delete `AdaptiveRouteKind` and `AdaptiveRoutePhase`.

Change `AdaptiveRouteSample` to:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AdaptiveRouteSample {
    pub route_id: u64,
    pub route_code: u16,
    pub t_us: u64,
    pub err: f64,
    pub resource_cost: f64,
    pub closure_gain: f64,
}
```

Change `AdaptiveRouteTrace` fields:

```rust
pub route_code: u16,
pub phase_code: u8,
```

Use numeric phase codes only:

```rust
const ROUTE_PHASE_OPEN: u8 = 0;
const ROUTE_PHASE_STABLE: u8 = 1;
const ROUTE_PHASE_DEGRADED: u8 = 2;
const ROUTE_PHASE_COMPRESSED: u8 = 3;
```

Update `observe`:

```rust
self.phase_code = if bad > 0.65 {
    ROUTE_PHASE_DEGRADED
} else if good > 0.72 && self.fast_path_strength > 0.35 {
    ROUTE_PHASE_COMPRESSED
} else if good > 0.72 {
    ROUTE_PHASE_STABLE
} else {
    ROUTE_PHASE_OPEN
};
```

In `bionic_v0/src/life_closure_producers.rs`, replace:

```rust
route_kind: AdaptiveRouteKind::ActionConsequence,
```

with:

```rust
route_code: 1,
```

Document `route_code` in a comment as an engineering locator only.

- [ ] **Step 4: Update snapshot and observer**

In `snapshot.rs`, write/read `route_code: u16` as `u64` and `phase_code: u8` as `u64`, matching the existing numeric style.

In `observer.rs`, render only numeric route code and phase code, for example:

```rust
format!("r{}:p{}:{:.3}", trace.route_code, trace.phase_code, trace.degradation_score)
```

- [ ] **Step 5: Verify targeted tests pass**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test adaptive_stability --test life_closure_hardening -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Verify contraction guard passes**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test architecture_contraction adaptive_stability_must_not_encode_named_repair_routes_or_phases -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add bionic_v0/src/adaptive_stability.rs bionic_v0/src/life_closure_producers.rs bionic_v0/src/snapshot.rs bionic_v0/src/bin/observer.rs bionic_v0/tests/adaptive_stability.rs bionic_v0/tests/life_closure_hardening.rs
git commit -m "refactor: make adaptive stability numeric route dynamics"
```

## Task 9: Remove Pre-Decoded ASCII/Glyph Code Projection

**Files:**
- Modify: `bionic_v0/src/code_ingress.rs`
- Modify: `bionic_v0/src/snapshot.rs`
- Modify: `bionic_v0/src/bin/observer.rs`
- Modify: `bionic_v0/tests/code_ingress.rs`

- [ ] **Step 1: Write failing behavior test**

In `bionic_v0/tests/code_ingress.rs`, add:

```rust
#[test]
fn code_ingress_records_raw_and_binary_projection_only_until_glyph_evidence_exists() {
    let mut state = bionic_v0::code_ingress::CodeIngressState::default();
    state.observe_frame(bionic_v0::code_ingress::CodeIngressFrameInput {
        stream_context_id: 3,
        t_start_us: 1_000,
        t_end_us: 1_100,
        bytes: b"abc",
        framing_validity: 0.9,
    });

    let kinds: Vec<u8> = state
        .projection_candidates
        .iter()
        .map(|projection| projection.decoder_kind_code)
        .collect();

    assert_eq!(
        kinds,
        vec![
            bionic_v0::code_ingress::CODE_DECODER_BYTE_UNIT,
            bionic_v0::code_ingress::CODE_DECODER_BINARY_FRAGMENT,
        ],
        "ASCII/glyph projection must not be created body-side without separate visual evidence"
    );
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test code_ingress code_ingress_records_raw_and_binary_projection_only_until_glyph_evidence_exists -- --nocapture
```

Expected: FAIL because ASCII input currently creates `CODE_DECODER_ASCII_SHAPE`.

- [ ] **Step 3: Remove ASCII projection**

In `bionic_v0/src/code_ingress.rs`, delete:

```rust
pub const CODE_DECODER_ASCII_SHAPE: u8 = 2;
```

Delete the block:

```rust
if input.bytes.iter().all(|b| *b <= 0x7f) {
    self.observe_projection(
        frame_seq,
        CODE_DECODER_ASCII_SHAPE,
        validity,
        input.bytes.len() as u32,
        input.t_end_us,
    );
}
```

Keep byte unit and binary fragment projections only.

- [ ] **Step 4: Update observer/tests**

Remove tests expecting three projections for ASCII frames. Assertions should check raw frame facts, two projection candidates, sequence windows, and ambiguity preservation.

- [ ] **Step 5: Verify targeted tests pass**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test code_ingress -- --nocapture
```

Expected: PASS.

- [ ] **Step 6: Verify contraction guard passes**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test architecture_contraction code_ingress_must_not_predecode_ascii_or_glyph_projection_in_body -- --nocapture
```

Expected: PASS.

- [ ] **Step 7: Commit**

```bash
git add bionic_v0/src/code_ingress.rs bionic_v0/src/snapshot.rs bionic_v0/src/bin/observer.rs bionic_v0/tests/code_ingress.rs
git commit -m "refactor: remove predecoded code glyph projection"
```

## Task 10: Align Architecture Map With Contracted Runtime

**Files:**
- Modify: `docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md`
- Test: `bionic_v0/tests/architecture_contraction.rs`

- [ ] **Step 1: Run failing doc coherence test**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test architecture_contraction global_architecture_map_matches_current_snapshot_version -- --nocapture
```

Expected: FAIL because the map still says `snapshot v41`.

- [ ] **Step 2: Update architecture map**

In `docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md`, replace:

```text
snapshot v41
```

with:

```text
snapshot v42
```

In the completion matrix, change these rows:

```markdown
| Evidence tension | Complete as numeric pressure path into probe law | ... |
| Learned bias persistence | Complete as delta sediment only | ... |
| Adaptive stability / local repair | Complete as route-local numeric fast path | ... |
```

to:

```markdown
| Evidence pressure | Contracted to generic pressure substrate | `substrate_pressure.rs`, `evidence_tension.rs`, `probe_law.rs` | contraction tests pass |
| Learned bias persistence | Contracted to generic pressure-axis sediment only | `learned_bias.rs`, snapshot summary | learned-bias and contraction tests pass |
| Adaptive route dynamics | Contracted to numeric route traces only | `adaptive_stability.rs`, snapshot / observer | adaptive-stability and contraction tests pass |
```

In remaining hardening items, delete the item about adding a monotonic pair sequence, because `pair_seq` already exists in snapshot v42.

- [ ] **Step 3: Verify doc coherence test passes**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test architecture_contraction global_architecture_map_matches_current_snapshot_version -- --nocapture
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md bionic_v0/tests/architecture_contraction.rs
git commit -m "docs: align architecture map with contracted runtime"
```

## Task 11: Run Full Contraction Verification

**Files:**
- No production files unless a previous test exposes a missed rename.

- [ ] **Step 1: Run all contraction guards**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test architecture_contraction -- --nocapture
```

Expected: PASS.

- [ ] **Step 2: Run body and cleanup hygiene**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml --test cleanup_hygiene --test body_no_human_output -- --nocapture
```

Expected: PASS.

- [ ] **Step 3: Run full crate test suite**

Run:

```bash
cargo test --manifest-path bionic_v0/Cargo.toml
```

Expected: PASS for all unit tests, integration tests, binary tests, and doc tests.

- [ ] **Step 4: Review high-risk source terms**

Run:

```bash
rg -n "semantic_readiness|SemanticReadinessState|refresh_semantic_readiness|symbolizer|SymbolizerState|refresh_symbolizer|concept_id|ObjectConceptBinding|RelationConceptBinding|CodeStream|DecoderProjection|VisualPrimitive|AcousticPrimitive|CommunicationLoop|CrossModalBinding|source_reliability_gap|CODE_DECODER_ASCII_SHAPE|tick_surprise_trigger|tick_attention_trigger|tick_relation_instability_trigger|AdaptiveRouteKind|AdaptiveRoutePhase|INTENT_" bionic_v0/src bionic_v0/tests docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md
```

Expected: no active-source matches except comments in frozen rejected-architecture documents and quarantine paths. Active `bionic_v0/src` must have no matches.

- [ ] **Step 5: Commit final verification note**

If code changed during verification, commit it:

```bash
git add bionic_v0 docs/superpowers/specs/2026-04-21-bionic-v0-global-architecture-map.md
git commit -m "test: verify architecture contraction guards"
```

If no files changed, record the command output in the final implementation report instead of creating an empty commit.

## Execution Notes

- Execute tasks in order. Tasks 4-9 depend on generic pressure primitives and guard tests from Tasks 3-4.
- Do not weaken guard tests to make implementation easier. If a guard catches a necessary engineering term, move the term behind a numeric locator or observer-only rendering path.
- Do not add replacement modules named `curiosity`, `ethics`, `owner`, `source trust`, `personality`, `goal`, or `intention`.
- Do not add a PTY, shell, SSH, command executor, chatbot loop, source identity layer, or observer-to-body feedback path in this plan.
- Keep commits small. A later revert should be able to undo one contraction task without corrupting unrelated substrate work.

