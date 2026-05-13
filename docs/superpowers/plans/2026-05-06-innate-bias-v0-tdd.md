# Innate Bias v0 TDD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the first test-driven numeric policy surface for the Innate Bias v0 spec without introducing semantic, token, concept, source-identity, or personality ontology.

**Architecture:** Extend the existing `bionic_v0/src/evolution_priors.rs` `BiasPriors` surface with numeric feature structs and pure scoring helpers. The first implementation does not wire runtime callers into VIB/PHOT/code I/O/action loops; it establishes tested substrate-level bias shapes and modification caps that later plans can call. All values remain numeric, bounded, and reversible.

**Tech Stack:** Rust, `cargo test`, existing `bionic_v0::evolution_priors` module.

---

## File Structure

- Modify: `bionic_v0/src/evolution_priors.rs`
  - Add ordinary and protected `BiasAxis` variants.
  - Add matching fields to `ModifyRateCaps`.
  - Add numeric feature structs for the first bias families.
  - Add pure helper functions that convert numeric features into bounded attention, retention, surprise, or tension scores.
  - Add unit tests under the existing `#[cfg(test)] mod tests`.

No runtime integration files are touched in this plan. Runtime integration is intentionally deferred until each substrate loop has its own failing caller-level test.

## Task 1: Extend Modification Caps For Innate Bias Axes

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests inside the existing `#[cfg(test)] mod tests` in `bionic_v0/src/evolution_priors.rs`:

```rust
    #[test]
    fn test_innate_bias_cap_families_have_expected_learning_speeds() {
        let priors = BiasPriors::default();

        let code_cap = priors.modify_rate_cap_for(BiasAxis::CodeStream);
        let visual_cap = priors.modify_rate_cap_for(BiasAxis::VisualPrimitive);
        let acoustic_cap = priors.modify_rate_cap_for(BiasAxis::AcousticPrimitive);
        let communication_cap = priors.modify_rate_cap_for(BiasAxis::CommunicationLoop);
        let tearing_cap = priors.modify_rate_cap_for(BiasAxis::StructuralTearing);
        let evidence_cap = priors.modify_rate_cap_for(BiasAxis::EvidenceOverwritePrevention);

        assert!(code_cap >= 0.01, "code stream cap should remain learnable");
        assert!(visual_cap >= 0.01, "visual primitive cap should remain learnable");
        assert!(acoustic_cap >= 0.01, "acoustic primitive cap should remain learnable");
        assert!(
            communication_cap >= 0.01,
            "communication-loop cap should remain learnable"
        );
        assert!(
            tearing_cap <= 0.005,
            "structural tearing cap should be protected, got {}",
            tearing_cap
        );
        assert!(
            evidence_cap <= 0.005,
            "evidence overwrite prevention cap should be protected, got {}",
            evidence_cap
        );
        assert!(tearing_cap < code_cap);
        assert!(evidence_cap < communication_cap);
    }

    #[test]
    fn test_clamp_modification_protects_structural_tearing_axis() {
        let priors = BiasPriors::default();
        let requested = -0.42;
        let cap = priors.modify_rate_cap_for(BiasAxis::StructuralTearing);
        let applied = priors.clamp_modification(BiasAxis::StructuralTearing, requested);

        assert!(applied.abs() <= cap);
        assert_eq!(applied.signum(), requested.signum());
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: compile FAIL because `BiasAxis::CodeStream`, `BiasAxis::VisualPrimitive`, `BiasAxis::AcousticPrimitive`, `BiasAxis::CommunicationLoop`, `BiasAxis::StructuralTearing`, and `BiasAxis::EvidenceOverwritePrevention` do not exist.

- [ ] **Step 3: Implement minimal cap support**

Update `BiasAxis`:

```rust
pub enum BiasAxis {
    // Pre-existing axes (allow moderate learning)
    VibBand,
    Surprise,
    BandLearnRate,
    CenterWeight,
    TemporalWindow,
    SlotGrowth,
    // Innate ordinary axes (learnable)
    CodeStream,
    DecoderProjection,
    VisualPrimitive,
    AcousticPrimitive,
    ActionConsequence,
    SpatialDistance,
    CommunicationLoop,
    CrossModalBinding,
    // Layer 0 axes (near-immutable)
    DisruptionBand,
    DisruptionThreshold,
    DisruptionMultiplier,
    Band2Gain,
    StructuralTearing,
    IrreversibleDisruption,
    EvidenceOverwritePrevention,
}
```

Update `ModifyRateCaps`:

```rust
pub struct ModifyRateCaps {
    pub vib_band: f64,
    pub surprise: f64,
    pub band_learn_rate: f64,
    pub center_weight: f64,
    pub temporal_window: f64,
    pub slot_growth: f64,
    // Innate ordinary axes — learnable
    pub code_stream: f64,
    pub decoder_projection: f64,
    pub visual_primitive: f64,
    pub acoustic_primitive: f64,
    pub action_consequence: f64,
    pub spatial_distance: f64,
    pub communication_loop: f64,
    pub cross_modal_binding: f64,
    // Layer 0 — near-immutable
    pub disruption_band: f64,
    pub disruption_threshold: f64,
    pub disruption_multiplier: f64,
    pub band2_gain: f64,
    pub structural_tearing: f64,
    pub irreversible_disruption: f64,
    pub evidence_overwrite_prevention: f64,
}
```

Update `ModifyRateCaps::default()` with these values after `slot_growth: 0.05,`:

```rust
            // Innate ordinary axes: moderate learning rate cap
            code_stream: 0.05,
            decoder_projection: 0.05,
            visual_primitive: 0.05,
            acoustic_primitive: 0.05,
            action_consequence: 0.05,
            spatial_distance: 0.05,
            communication_loop: 0.05,
            cross_modal_binding: 0.05,
```

Add these protected defaults after `band2_gain: 0.001,`:

```rust
            structural_tearing: 0.001,
            irreversible_disruption: 0.001,
            evidence_overwrite_prevention: 0.001,
```

Update `modify_rate_cap_for` with these match arms:

```rust
            BiasAxis::CodeStream => self.modify_rate_caps.code_stream,
            BiasAxis::DecoderProjection => self.modify_rate_caps.decoder_projection,
            BiasAxis::VisualPrimitive => self.modify_rate_caps.visual_primitive,
            BiasAxis::AcousticPrimitive => self.modify_rate_caps.acoustic_primitive,
            BiasAxis::ActionConsequence => self.modify_rate_caps.action_consequence,
            BiasAxis::SpatialDistance => self.modify_rate_caps.spatial_distance,
            BiasAxis::CommunicationLoop => self.modify_rate_caps.communication_loop,
            BiasAxis::CrossModalBinding => self.modify_rate_caps.cross_modal_binding,
            BiasAxis::StructuralTearing => self.modify_rate_caps.structural_tearing,
            BiasAxis::IrreversibleDisruption => self.modify_rate_caps.irreversible_disruption,
            BiasAxis::EvidenceOverwritePrevention => {
                self.modify_rate_caps.evidence_overwrite_prevention
            }
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: PASS for both tests.

- [ ] **Step 5: Run existing evolution priors tests**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors
```

Expected: all `evolution_priors` unit tests pass.

## Task 2: Add Computer-Native Code Bias Policy

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests inside `mod tests`:

```rust
    #[test]
    fn test_code_native_bias_prefers_repeated_corrected_ambiguous_streams() {
        let priors = BiasPriors::default();
        let low = CodeNativeFeatures {
            recurrence: 0.1,
            correction_pressure: 0.0,
            decoder_ambiguity: 0.0,
            output_return_alignment: 0.0,
            glyph_alignment: 0.0,
        };
        let high = CodeNativeFeatures {
            recurrence: 0.8,
            correction_pressure: 0.7,
            decoder_ambiguity: 0.6,
            output_return_alignment: 0.9,
            glyph_alignment: 0.8,
        };

        let low_score = compute_code_native_attention(&low, &priors);
        let high_score = compute_code_native_attention(&high, &priors);

        assert!(high_score > low_score);
        assert!((0.0..=2.0).contains(&high_score));
    }

    #[test]
    fn test_code_native_bias_clamps_out_of_range_inputs() {
        let priors = BiasPriors::default();
        let features = CodeNativeFeatures {
            recurrence: 2.0,
            correction_pressure: -1.0,
            decoder_ambiguity: 4.0,
            output_return_alignment: 3.0,
            glyph_alignment: -2.0,
        };

        let score = compute_code_native_attention(&features, &priors);
        assert!((0.0..=2.0).contains(&score));
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: compile FAIL because `CodeNativeFeatures` and `compute_code_native_attention` do not exist.

- [ ] **Step 3: Implement minimal policy**

Add this helper near the top of the file after `use crate::VIB_BANDS;`:

```rust
fn unit(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}
```

Add fields to `BiasPriors` before `modify_rate_caps`:

```rust
    // --- Innate v0: computer-native code stream bias ---
    pub code_stream_attention_mul: f64,
    pub code_recurrence_weight: f64,
    pub code_correction_weight: f64,
    pub code_ambiguity_weight: f64,
    pub code_return_alignment_weight: f64,
    pub code_glyph_alignment_weight: f64,
```

Add defaults before `modify_rate_caps: ModifyRateCaps::default(),`:

```rust
            code_stream_attention_mul: 1.25,
            code_recurrence_weight: 0.25,
            code_correction_weight: 0.25,
            code_ambiguity_weight: 0.15,
            code_return_alignment_weight: 0.20,
            code_glyph_alignment_weight: 0.15,
```

Add these items after `impl BiasPriors`:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CodeNativeFeatures {
    pub recurrence: f64,
    pub correction_pressure: f64,
    pub decoder_ambiguity: f64,
    pub output_return_alignment: f64,
    pub glyph_alignment: f64,
}

pub fn compute_code_native_attention(features: &CodeNativeFeatures, priors: &BiasPriors) -> f64 {
    let weighted = unit(features.recurrence) * priors.code_recurrence_weight
        + unit(features.correction_pressure) * priors.code_correction_weight
        + unit(features.decoder_ambiguity) * priors.code_ambiguity_weight
        + unit(features.output_return_alignment) * priors.code_return_alignment_weight
        + unit(features.glyph_alignment) * priors.code_glyph_alignment_weight;
    (weighted * priors.code_stream_attention_mul).clamp(0.0, 2.0)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: PASS for both tests.

## Task 3: Add Digital Structure Integrity And Disruption Policies

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests:

```rust
    #[test]
    fn test_digital_structure_integrity_surprise_rises_with_tearing_and_error_spread() {
        let priors = BiasPriors::default();
        let intact = DigitalStructureFeatures {
            closure_break: 0.0,
            reference_break: 0.0,
            shape_mismatch: 0.1,
            error_spread: 0.1,
            deletion_fraction: 0.0,
        };
        let torn = DigitalStructureFeatures {
            closure_break: 0.9,
            reference_break: 0.8,
            shape_mismatch: 0.7,
            error_spread: 0.9,
            deletion_fraction: 0.8,
        };

        let intact_score = compute_digital_structure_surprise(&intact, &priors);
        let torn_score = compute_digital_structure_surprise(&torn, &priors);

        assert!(torn_score > intact_score * 2.0);
        assert!((0.0..=4.0).contains(&torn_score));
    }

    #[test]
    fn test_general_disruption_score_rises_with_irreversible_instability() {
        let priors = BiasPriors::default();
        let stable = GeneralDisruptionFeatures {
            discontinuity: 0.1,
            collapse: 0.0,
            fragmentation: 0.0,
            irreversible_loss: 0.0,
            feedback_amplification: 0.0,
        };
        let disrupted = GeneralDisruptionFeatures {
            discontinuity: 0.8,
            collapse: 0.7,
            fragmentation: 0.7,
            irreversible_loss: 0.9,
            feedback_amplification: 0.8,
        };

        assert!(
            compute_general_disruption_score(&disrupted, &priors)
                > compute_general_disruption_score(&stable, &priors)
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: compile FAIL because the feature structs and functions do not exist.

- [ ] **Step 3: Implement minimal policies**

Add fields to `BiasPriors` before `modify_rate_caps`:

```rust
    // --- Innate v0: digital structure and disruption bias ---
    pub digital_structure_surprise_mul: f64,
    pub disruption_structure_mul: f64,
```

Add defaults:

```rust
            digital_structure_surprise_mul: 2.0,
            disruption_structure_mul: 2.0,
```

Add these items after `compute_code_native_attention`:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct DigitalStructureFeatures {
    pub closure_break: f64,
    pub reference_break: f64,
    pub shape_mismatch: f64,
    pub error_spread: f64,
    pub deletion_fraction: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GeneralDisruptionFeatures {
    pub discontinuity: f64,
    pub collapse: f64,
    pub fragmentation: f64,
    pub irreversible_loss: f64,
    pub feedback_amplification: f64,
}

pub fn compute_digital_structure_surprise(
    features: &DigitalStructureFeatures,
    priors: &BiasPriors,
) -> f64 {
    let weighted = unit(features.closure_break) * 0.20
        + unit(features.reference_break) * 0.20
        + unit(features.shape_mismatch) * 0.15
        + unit(features.error_spread) * 0.25
        + unit(features.deletion_fraction) * 0.20;
    (weighted * priors.digital_structure_surprise_mul).clamp(0.0, 4.0)
}

pub fn compute_general_disruption_score(
    features: &GeneralDisruptionFeatures,
    priors: &BiasPriors,
) -> f64 {
    let weighted = unit(features.discontinuity) * 0.20
        + unit(features.collapse) * 0.20
        + unit(features.fragmentation) * 0.20
        + unit(features.irreversible_loss) * 0.25
        + unit(features.feedback_amplification) * 0.15;
    (weighted * priors.disruption_structure_mul).clamp(0.0, 4.0)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: PASS for both tests.

## Task 4: Add Visual, Acoustic, Spatial, And Action Bias Policies

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests:

```rust
    #[test]
    fn test_visual_primitive_attention_rises_with_edges_motion_and_damage() {
        let priors = BiasPriors::default();
        let weak = VisualPrimitiveFeatures {
            edge_density: 0.1,
            color_contrast: 0.1,
            motion_change: 0.1,
            distance_change: 0.0,
            structural_damage: 0.0,
        };
        let strong = VisualPrimitiveFeatures {
            edge_density: 0.8,
            color_contrast: 0.7,
            motion_change: 0.8,
            distance_change: 0.6,
            structural_damage: 0.9,
        };

        assert!(
            compute_visual_primitive_attention(&strong, &priors)
                > compute_visual_primitive_attention(&weak, &priors)
        );
    }

    #[test]
    fn test_acoustic_primitive_attention_rises_with_rhythm_burst_and_instability() {
        let priors = BiasPriors::default();
        let weak = AcousticPrimitiveFeatures {
            rhythm: 0.1,
            pitch_stability: 0.1,
            sharp_transient: 0.0,
            broadband_burst: 0.0,
            contour_instability: 0.0,
        };
        let strong = AcousticPrimitiveFeatures {
            rhythm: 0.7,
            pitch_stability: 0.7,
            sharp_transient: 0.8,
            broadband_burst: 0.9,
            contour_instability: 0.8,
        };

        assert!(
            compute_acoustic_primitive_attention(&strong, &priors)
                > compute_acoustic_primitive_attention(&weak, &priors)
        );
    }

    #[test]
    fn test_spatial_distance_attention_rises_with_looming_and_boundary_crossing() {
        let priors = BiasPriors::default();
        let weak = SpatialDistanceFeatures {
            approach_rate: 0.1,
            looming: 0.0,
            occlusion_change: 0.0,
            boundary_crossing: 0.0,
            path_continuity: 0.1,
        };
        let strong = SpatialDistanceFeatures {
            approach_rate: 0.8,
            looming: 0.9,
            occlusion_change: 0.7,
            boundary_crossing: 0.8,
            path_continuity: 0.7,
        };

        assert!(
            compute_spatial_distance_attention(&strong, &priors)
                > compute_spatial_distance_attention(&weak, &priors)
        );
    }

    #[test]
    fn test_action_consequence_retention_rises_with_write_return_alignment() {
        let priors = BiasPriors::default();
        let weak = ActionConsequenceFeatures {
            write_return_alignment: 0.1,
            delay_stability: 0.1,
            magnitude_coupling: 0.1,
            retraction_error: 0.0,
            repeatability: 0.1,
        };
        let strong = ActionConsequenceFeatures {
            write_return_alignment: 0.9,
            delay_stability: 0.8,
            magnitude_coupling: 0.8,
            retraction_error: 0.7,
            repeatability: 0.9,
        };

        assert!(
            compute_action_consequence_retention(&strong, &priors)
                > compute_action_consequence_retention(&weak, &priors)
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: compile FAIL because the feature structs and functions do not exist.

- [ ] **Step 3: Implement minimal policies**

Add fields to `BiasPriors`:

```rust
    // --- Innate v0: multimodal primitive bias ---
    pub visual_attention_mul: f64,
    pub acoustic_attention_mul: f64,
    pub spatial_attention_mul: f64,
    pub action_consequence_retention_mul: f64,
```

Add defaults:

```rust
            visual_attention_mul: 1.5,
            acoustic_attention_mul: 1.5,
            spatial_attention_mul: 1.5,
            action_consequence_retention_mul: 1.5,
```

Add these items:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VisualPrimitiveFeatures {
    pub edge_density: f64,
    pub color_contrast: f64,
    pub motion_change: f64,
    pub distance_change: f64,
    pub structural_damage: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AcousticPrimitiveFeatures {
    pub rhythm: f64,
    pub pitch_stability: f64,
    pub sharp_transient: f64,
    pub broadband_burst: f64,
    pub contour_instability: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SpatialDistanceFeatures {
    pub approach_rate: f64,
    pub looming: f64,
    pub occlusion_change: f64,
    pub boundary_crossing: f64,
    pub path_continuity: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ActionConsequenceFeatures {
    pub write_return_alignment: f64,
    pub delay_stability: f64,
    pub magnitude_coupling: f64,
    pub retraction_error: f64,
    pub repeatability: f64,
}

pub fn compute_visual_primitive_attention(
    features: &VisualPrimitiveFeatures,
    priors: &BiasPriors,
) -> f64 {
    let weighted = unit(features.edge_density) * 0.20
        + unit(features.color_contrast) * 0.15
        + unit(features.motion_change) * 0.20
        + unit(features.distance_change) * 0.15
        + unit(features.structural_damage) * 0.30;
    (weighted * priors.visual_attention_mul).clamp(0.0, 2.0)
}

pub fn compute_acoustic_primitive_attention(
    features: &AcousticPrimitiveFeatures,
    priors: &BiasPriors,
) -> f64 {
    let weighted = unit(features.rhythm) * 0.15
        + unit(features.pitch_stability) * 0.15
        + unit(features.sharp_transient) * 0.25
        + unit(features.broadband_burst) * 0.25
        + unit(features.contour_instability) * 0.20;
    (weighted * priors.acoustic_attention_mul).clamp(0.0, 2.0)
}

pub fn compute_spatial_distance_attention(
    features: &SpatialDistanceFeatures,
    priors: &BiasPriors,
) -> f64 {
    let weighted = unit(features.approach_rate) * 0.20
        + unit(features.looming) * 0.25
        + unit(features.occlusion_change) * 0.15
        + unit(features.boundary_crossing) * 0.20
        + unit(features.path_continuity) * 0.20;
    (weighted * priors.spatial_attention_mul).clamp(0.0, 2.0)
}

pub fn compute_action_consequence_retention(
    features: &ActionConsequenceFeatures,
    priors: &BiasPriors,
) -> f64 {
    let weighted = unit(features.write_return_alignment) * 0.25
        + unit(features.delay_stability) * 0.20
        + unit(features.magnitude_coupling) * 0.20
        + unit(features.retraction_error) * 0.15
        + unit(features.repeatability) * 0.20;
    (weighted * priors.action_consequence_retention_mul).clamp(0.0, 2.0)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: PASS for all four tests.

## Task 5: Add Communication, Epistemic Tension, And Cross-Modal Binding Policies

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`

- [ ] **Step 1: Write the failing tests**

Add these tests:

```rust
    #[test]
    fn test_communication_loop_retention_rises_with_turns_and_corrections() {
        let priors = BiasPriors::default();
        let weak = CommunicationLoopFeatures {
            turn_alternation: 0.1,
            response_window_alignment: 0.1,
            correction_density: 0.0,
            repeated_external_pattern: 0.1,
            support_deviation_history: 0.1,
        };
        let strong = CommunicationLoopFeatures {
            turn_alternation: 0.9,
            response_window_alignment: 0.8,
            correction_density: 0.7,
            repeated_external_pattern: 0.8,
            support_deviation_history: 0.9,
        };

        assert!(
            compute_communication_loop_retention(&strong, &priors)
                > compute_communication_loop_retention(&weak, &priors)
        );
    }

    #[test]
    fn test_epistemic_tension_rises_with_conflict_and_low_support() {
        let priors = BiasPriors::default();
        let settled = EpistemicTensionFeatures {
            assertion_observation_conflict: 0.0,
            prediction_error: 0.1,
            source_reliability_gap: 0.0,
            cross_modal_disagreement: 0.0,
            reproducibility_gap: 0.1,
        };
        let tense = EpistemicTensionFeatures {
            assertion_observation_conflict: 0.9,
            prediction_error: 0.8,
            source_reliability_gap: 0.7,
            cross_modal_disagreement: 0.8,
            reproducibility_gap: 0.9,
        };

        assert!(compute_epistemic_tension(&tense, &priors) > compute_epistemic_tension(&settled, &priors));
    }

    #[test]
    fn test_cross_modal_binding_bias_rises_with_multi_channel_alignment() {
        let priors = BiasPriors::default();
        let weak = CrossModalBindingFeatures {
            code_glyph_alignment: 0.1,
            sound_motion_alignment: 0.0,
            action_return_alignment: 0.1,
            spatial_surface_alignment: 0.1,
            repeated_pattern_alignment: 0.1,
        };
        let strong = CrossModalBindingFeatures {
            code_glyph_alignment: 0.9,
            sound_motion_alignment: 0.7,
            action_return_alignment: 0.8,
            spatial_surface_alignment: 0.8,
            repeated_pattern_alignment: 0.9,
        };

        assert!(
            compute_cross_modal_binding_bias(&strong, &priors)
                > compute_cross_modal_binding_bias(&weak, &priors)
        );
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: compile FAIL because the feature structs and functions do not exist.

- [ ] **Step 3: Implement minimal policies**

Add fields to `BiasPriors`:

```rust
    // --- Innate v0: communication, evidence tension, and cross-modal binding ---
    pub communication_retention_mul: f64,
    pub epistemic_tension_mul: f64,
    pub cross_modal_binding_mul: f64,
```

Add defaults:

```rust
            communication_retention_mul: 1.5,
            epistemic_tension_mul: 2.0,
            cross_modal_binding_mul: 1.5,
```

Add these items:

```rust
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CommunicationLoopFeatures {
    pub turn_alternation: f64,
    pub response_window_alignment: f64,
    pub correction_density: f64,
    pub repeated_external_pattern: f64,
    pub support_deviation_history: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct EpistemicTensionFeatures {
    pub assertion_observation_conflict: f64,
    pub prediction_error: f64,
    pub source_reliability_gap: f64,
    pub cross_modal_disagreement: f64,
    pub reproducibility_gap: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CrossModalBindingFeatures {
    pub code_glyph_alignment: f64,
    pub sound_motion_alignment: f64,
    pub action_return_alignment: f64,
    pub spatial_surface_alignment: f64,
    pub repeated_pattern_alignment: f64,
}

pub fn compute_communication_loop_retention(
    features: &CommunicationLoopFeatures,
    priors: &BiasPriors,
) -> f64 {
    let weighted = unit(features.turn_alternation) * 0.20
        + unit(features.response_window_alignment) * 0.20
        + unit(features.correction_density) * 0.20
        + unit(features.repeated_external_pattern) * 0.20
        + unit(features.support_deviation_history) * 0.20;
    (weighted * priors.communication_retention_mul).clamp(0.0, 2.0)
}

pub fn compute_epistemic_tension(
    features: &EpistemicTensionFeatures,
    priors: &BiasPriors,
) -> f64 {
    let weighted = unit(features.assertion_observation_conflict) * 0.25
        + unit(features.prediction_error) * 0.25
        + unit(features.source_reliability_gap) * 0.15
        + unit(features.cross_modal_disagreement) * 0.20
        + unit(features.reproducibility_gap) * 0.15;
    (weighted * priors.epistemic_tension_mul).clamp(0.0, 4.0)
}

pub fn compute_cross_modal_binding_bias(
    features: &CrossModalBindingFeatures,
    priors: &BiasPriors,
) -> f64 {
    let weighted = unit(features.code_glyph_alignment) * 0.25
        + unit(features.sound_motion_alignment) * 0.15
        + unit(features.action_return_alignment) * 0.25
        + unit(features.spatial_surface_alignment) * 0.15
        + unit(features.repeated_pattern_alignment) * 0.20;
    (weighted * priors.cross_modal_binding_mul).clamp(0.0, 2.0)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: PASS for all three tests.

## Task 6: Guard Against Ontology Drift And Run Verification

**Files:**
- Modify: `bionic_v0/src/evolution_priors.rs`

- [ ] **Step 1: Write the guard tests**

Add these tests:

```rust
    #[test]
    fn test_innate_bias_outputs_are_bounded_for_extreme_inputs() {
        let priors = BiasPriors::default();
        let code = CodeNativeFeatures {
            recurrence: 99.0,
            correction_pressure: 99.0,
            decoder_ambiguity: 99.0,
            output_return_alignment: 99.0,
            glyph_alignment: 99.0,
        };
        let digital = DigitalStructureFeatures {
            closure_break: 99.0,
            reference_break: 99.0,
            shape_mismatch: 99.0,
            error_spread: 99.0,
            deletion_fraction: 99.0,
        };
        let epistemic = EpistemicTensionFeatures {
            assertion_observation_conflict: 99.0,
            prediction_error: 99.0,
            source_reliability_gap: 99.0,
            cross_modal_disagreement: 99.0,
            reproducibility_gap: 99.0,
        };

        assert!((0.0..=2.0).contains(&compute_code_native_attention(&code, &priors)));
        assert!((0.0..=4.0).contains(&compute_digital_structure_surprise(&digital, &priors)));
        assert!((0.0..=4.0).contains(&compute_epistemic_tension(&epistemic, &priors)));
    }

    #[test]
    fn test_source_reliability_gap_does_not_create_global_trust_or_distrust() {
        let priors = BiasPriors::default();
        let reliable_but_conflicted = EpistemicTensionFeatures {
            assertion_observation_conflict: 0.8,
            prediction_error: 0.8,
            source_reliability_gap: 0.0,
            cross_modal_disagreement: 0.8,
            reproducibility_gap: 0.8,
        };
        let unreliable_but_supported = EpistemicTensionFeatures {
            assertion_observation_conflict: 0.0,
            prediction_error: 0.0,
            source_reliability_gap: 0.8,
            cross_modal_disagreement: 0.0,
            reproducibility_gap: 0.0,
        };

        assert!(
            compute_epistemic_tension(&reliable_but_conflicted, &priors)
                > compute_epistemic_tension(&unreliable_but_supported, &priors),
            "tension must be dominated by evidence conflict, not a source handle alone"
        );
    }
```

- [ ] **Step 2: Run guard tests**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors::tests
```

Expected: PASS for both tests. If the first test fails, clamp the relevant helper output. If the second test fails, reduce the `source_reliability_gap` weight below direct conflict and prediction-error weights.

- [ ] **Step 3: Search for banned architecture terms in the implementation**

Run:

```bash
cd /Users/joe/unnamed && rg -n "semantic_readiness|SymbolizerState|refresh_symbolizer|concept_id|ObjectConceptBinding|RelationConceptBinding|println!|eprintln!" bionic_v0/src/evolution_priors.rs
```

Expected: no matches.

- [ ] **Step 4: Run the focused test suite**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test --lib evolution_priors
```

Expected: all evolution-priors tests pass.

- [ ] **Step 5: Run the full test suite**

Run:

```bash
cd /Users/joe/unnamed/bionic_v0 && cargo test
```

Expected: all tests pass.

- [ ] **Step 6: Commit implementation**

Run:

```bash
cd /Users/joe/unnamed && git add bionic_v0/src/evolution_priors.rs && git commit -m "feat: add innate bias v0 numeric policy surface"
```

Expected: commit succeeds. Commit message records that this is a policy surface only and does not add runtime semantic, token, concept, source-identity, or personality ontology.

## Self-Review Notes

- Spec coverage: this plan covers the first numeric surface for all ten first-version bias families from `2026-05-06-innate-bias-v0-design.md`.
- Runtime integration gap: by design, no VIB/PHOT/code I/O/action caller is changed here. Each caller requires its own later TDD plan with a failing integration test.
- Ontology boundary: all new data is numeric feature input or bounded numeric output. There are no dictionaries, source-trust lists, code-quality labels, visual object classes, token ids, or concept ids.
- Life-sign compatibility: action-consequence and code egress are represented only as numeric policy helpers in this plan; any actual egress integration must later go through write epochs and ordinary ingress return.
