use std::sync::Arc;

use bionic_v0::action_consequence::ActionConsequenceWindowEvent;
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::adapter::{ModalityAdapter, RealPhotAdapter, VibAdapter};
use bionic_v0::evolution_priors::BiasPriors;
use bionic_v0::language_binding::{BindingEvent, BindingFeedback, BindingTargetRef, PatternKind};
use bionic_v0::phot_camera::{SynthCamStep, SyntheticCameraFrameProvider};
use bionic_v0::viewport::Viewport;
use bionic_v0::{MOD_PHOT, PAIR_HISTORY_CAP};

fn curves_with<F>(edit: F) -> Arc<BiasPriors>
where
    F: FnOnce(&mut BiasPriors),
{
    let mut curves = BiasPriors::default();
    edit(&mut curves);
    Arc::new(curves)
}

#[test]
fn live_vib_adapter_reads_viewport_birth_sensitivity_curves() {
    let curves = curves_with(|p| {
        p.pressure_weights.multiplier = 1.75;
    });
    let vp = Arc::new(Viewport::with_birth_sensitivity(curves));

    let adapter = VibAdapter::new(&vp);

    assert_eq!(adapter.priors().pressure_weights.multiplier, 1.75);
}

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

fn run_phot_path(visual_mul: f64) -> (f64, Vec<f64>) {
    let curves = curves_with(|p| {
        p.pressure_weights.multiplier = visual_mul;
    });
    let vp = Arc::new(Viewport::with_birth_sensitivity(curves.clone()));
    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.14,
            brightness: 0.95,
        },
        SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.14,
            brightness: 0.95,
        },
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::with_birth_sensitivity(provider, curves);

    for step in 0..3u64 {
        adapter.tick(&vp, 1_000_000 + step * 33_333);
    }

    let ambient_surprise = vp.read_ambient().phot.surprise;
    let mut observed = Vec::new();
    for slot_id in 0..bionic_v0::N_OBJECT_SLOTS {
        let slot = vp.read_slot(slot_id).unwrap();
        if slot.active && (slot.source_modality & MOD_PHOT) != 0 {
            if let Some(phot) = slot.phot {
                observed = phot.observed;
                break;
            }
        }
    }
    assert!(
        !observed.is_empty(),
        "PHOT caller must publish observed bands"
    );
    (ambient_surprise, observed)
}

#[test]
fn real_phot_caller_applies_birth_sensitivity_without_rewriting_observed_bands() {
    let (low_surprise, low_observed) = run_phot_path(0.0);
    let (high_surprise, high_observed) = run_phot_path(6.0);

    assert!(
        high_surprise > low_surprise,
        "higher visual birth sensitivity should raise PHOT surprise pressure: low={low_surprise}, high={high_surprise}"
    );
    assert_eq!(
        high_observed, low_observed,
        "birth sensitivity must not rewrite raw PHOT observed bands"
    );
}

fn add_action_pair(
    vp: &Viewport,
    t_us: u64,
    write_magnitude: f64,
    consequence_magnitude: f64,
    delay_us: u64,
) -> u64 {
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: write_magnitude,
        event_t_us: t_us,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: bionic_v0::action_consequence::ConsequenceModalityKind::GenericIngress,
        start_t_us: t_us + delay_us,
        end_t_us: t_us + delay_us + 1_000,
        magnitude: consequence_magnitude,
    });
    let window_id = vp
        .read_action_consequence()
        .windows
        .last()
        .expect("window must exist")
        .window_id;
    assert!(vp.pair_action_consequence(write_seq, window_id, delay_us));
    write_seq
}

#[test]
fn action_caller_retains_strong_write_return_curve_when_pair_history_overflows() {
    let curves = curves_with(|p| {
        p.pressure_weights.multiplier = 2.0;
    });
    let vp = Viewport::with_birth_sensitivity(curves);

    let retained_seq = add_action_pair(&vp, 1_000_000, 0.8, 0.8, 40_000);

    for i in 0..(PAIR_HISTORY_CAP + 8) {
        let t = 2_000_000 + i as u64 * 100_000;
        add_action_pair(&vp, t, 1.0, 0.0, 900_000);
    }

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs as usize, PAIR_HISTORY_CAP);
    assert!(
        st.pairs.iter().any(|p| p.write_seq == retained_seq),
        "strong aligned write-return pair should survive retention-pressure eviction"
    );
}

fn repeated_binding_strength(communication_mul: f64) -> f64 {
    let curves = curves_with(|p| {
        p.pressure_weights.multiplier = communication_mul;
    });
    let vp = Arc::new(Viewport::with_birth_sensitivity(curves));

    for i in 0..3u64 {
        vp.apply_binding_event_from_adapter(BindingEvent {
            pattern_kind: PatternKind::VibModality,
            pattern_hash: 0xA11CE,
            target: BindingTargetRef::ObjectSlot { slot_id: 0 },
            feedback: BindingFeedback::Align,
            evidence_ref: i + 1,
            event_t_us: 1_000 + i * 1_000,
        });
    }

    vp.read_language_binding().traces[0].strength.value
}

#[test]
fn language_binding_caller_uses_birth_sensitivity_as_update_pressure_not_word_meaning() {
    let low_strength = repeated_binding_strength(0.0);
    let high_strength = repeated_binding_strength(4.0);

    assert!(
        high_strength > low_strength,
        "communication birth sensitivity should only increase numeric binding update pressure"
    );

    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/language_binding.rs"
    ))
    .expect("read language_binding.rs");
    for forbidden in ["concept_id", "token_id", "word_meaning", "name_gate"] {
        assert!(!src.contains(forbidden), "must not introduce {forbidden}");
    }
}

#[test]
fn nursery_newborn_bias_differs_from_default_in_voice_motion_and_turn_taking_axes() {
    use bionic_v0::evolution_priors::BiasPriors;

    let default = BiasPriors::default();
    let newborn = BiasPriors::nursery_newborn();

    // Voice sensitivity: bands 1-3 boosted relative to default
    for band in 1..=3 {
        assert!(
            newborn.band_sensitivity[band] > default.band_sensitivity[band],
            "band {band}: newborn {:.3} > default {:.3}",
            newborn.band_sensitivity[band],
            default.band_sensitivity[band],
        );
    }

    // Motion/error sensitivity
    assert!(newborn.pressure_weights.err > default.pressure_weights.err);

    // Recurrence sensitivity
    assert!(newborn.pressure_weights.recurrence > default.pressure_weights.recurrence);

    // Turn-taking: temporal gap sensitivity
    assert!(newborn.pressure_weights.temporal_gap > default.pressure_weights.temporal_gap);

    // Cross-channel coupling
    assert!(newborn.pressure_weights.cross_channel > default.pressure_weights.cross_channel);

    // Overall pressure multiplier
    assert!(newborn.pressure_weights.multiplier > default.pressure_weights.multiplier);

    // Learning rate caps relaxed
    assert!(newborn.modify_rate_caps.err_gain > default.modify_rate_caps.err_gain);
    assert!(newborn.modify_rate_caps.recurrence_gain > default.modify_rate_caps.recurrence_gain);
    assert!(newborn.modify_rate_caps.temporal_gap_gain > default.modify_rate_caps.temporal_gap_gain);

    // Non-target bands stay at default
    assert!((newborn.band_sensitivity[0] - default.band_sensitivity[0]).abs() < 0.01);
}
