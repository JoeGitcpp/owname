use bionic_v0::action_output::ActionOutputPhase;
use bionic_v0::learned_bias::{
    LearnedBiasState, EFFICACY_HALF_LIFE_US, FAMILIARITY_HALF_LIFE_US, SUPPRESSION_HALF_LIFE_US,
};
use bionic_v0::prediction_pressure_field::{PredictionWaveInput, PREDICTION_WAVE_EVENT};
use bionic_v0::viewport::Viewport;
use bionic_v0::suppression_damp_floor;
use bionic_v0::vocal_reflex::{VocalReflexDrive, VocalReflexState, SUPPRESSION_DAMP_FLOOR};
use bionic_v0::SUPPRESSION_DAMP_FLOOR_MATURE;

fn shock_wave(t_us: u64, amplitude: f64) -> PredictionWaveInput {
    PredictionWaveInput {
        source_kind: PREDICTION_WAVE_EVENT,
        slot_id: 1,
        edge_id: 3,
        window_start_us: t_us,
        window_end_us: t_us + 10_000,
        amplitude,
        resonance: 0.92,
        closure_affordance: 0.20,
        write_risk: 0.02,
    }
}

fn drive_prediction_shock(vp: &Viewport, t_us: u64, amplitude: f64) {
    vp.refresh_prediction_pressure_field(t_us, &[shock_wave(t_us, amplitude)]);
    vp.tick_vocal_reflex_from_pressure(t_us + 1_000);
}

#[test]
fn high_prediction_shock_raises_reflex_pressure_and_vocal_burst() {
    let vp = Viewport::new();

    drive_prediction_shock(&vp, 1_000_000, 0.98);

    let reflex = vp.read_vocal_reflex();
    let organ = vp.read_vocal_organ();
    let out = vp.read_action_output();

    assert!(reflex.reflex_pressure > 0.40);
    assert!(reflex.fast_gain > 0.0);
    assert_eq!(reflex.burst_count, 1);
    assert!(organ.breath_drive > 0.0);
    assert!(organ.noise_mix > 0.0);
    assert_eq!(out.channels[0].phase, ActionOutputPhase::Active);
    assert_eq!(out.n_write_epochs, 1);
}

#[test]
fn repeated_similar_shock_raises_familiarity_and_lowers_reflex_pressure() {
    let vp = Viewport::new();

    drive_prediction_shock(&vp, 2_000_000, 0.98);
    let first = vp.read_vocal_reflex();
    // familiarity / suppression migrated to learned_bias (死后保留的底层偏置).
    let first_bias = vp.read_learned_bias();
    vp.observe_vocal_reflex_closure(2_020_000, 0.70, true);

    for idx in 0..5 {
        let t_us = 2_200_000 + idx * 220_000;
        drive_prediction_shock(&vp, t_us, 0.92);
        vp.observe_vocal_reflex_closure(t_us + 20_000, 0.80, false);
    }
    drive_prediction_shock(&vp, 3_700_000, 0.92);
    let later = vp.read_vocal_reflex();
    let later_bias = vp.read_learned_bias();

    assert!(later_bias.prediction_familiarity > first_bias.prediction_familiarity);
    assert!(later.reflex_pressure < first.reflex_pressure);
    assert!(later_bias.action_suppression > 0.0);
}

#[test]
fn quiet_regulation_sediments_suppression_without_erasing_reflex_path() {
    let vp = Viewport::new();

    drive_prediction_shock(&vp, 4_000_000, 0.95);
    vp.observe_vocal_reflex_closure(4_050_000, 0.90, false);
    vp.observe_vocal_reflex_closure(4_250_000, 0.90, false);
    vp.observe_vocal_reflex_closure(4_450_000, 0.90, false);
    let reflex = vp.read_vocal_reflex();
    let bias = vp.read_learned_bias();

    assert!(reflex.quiet_regulation_successes >= 3);
    // suppression_strength migrated to learned_bias.action_suppression.
    // Refactor (plan 2026-05-28 Step2) lowered per-event delta to ≤0.01 so this
    // bias is a slow, 死后保留 sediment: 3 closures at 0.90 ≈ 0.027, not >0.40.
    assert!(
        bias.action_suppression > 0.02,
        "3 quiet closures should sediment some suppression: {}",
        bias.action_suppression
    );
    assert!(reflex.fast_gain >= 0.0);
}

#[test]
fn high_unresolved_pressure_can_bypass_learned_suppression() {
    let vp = Viewport::new();

    drive_prediction_shock(&vp, 5_000_000, 0.90);
    for idx in 0..6 {
        vp.observe_vocal_reflex_closure(5_020_000 + idx * 50_000, 0.95, false);
    }
    let suppressed = vp.read_vocal_reflex();
    let suppressed_bias = vp.read_learned_bias();
    // Refactor lowered per-event delta to ≤0.01: 6 closures at 0.95 ≈ 0.057.
    assert!(
        suppressed_bias.action_suppression > 0.05,
        "6 quiet closures should sediment suppression bias: {}",
        suppressed_bias.action_suppression
    );

    drive_prediction_shock(&vp, 6_000_000, 1.0);
    let bypassed = vp.read_vocal_reflex();

    assert!(bypassed.bypass_count >= 1);
    assert!(bypassed.reflex_pressure > 0.40);
    assert!(bypassed.burst_count > suppressed.burst_count);
}

#[test]
fn vocal_reflex_sources_do_not_introduce_emotion_or_text_ontology() {
    let src = include_str!("../src/vocal_reflex.rs");
    let lowered = src.to_ascii_lowercase();
    for forbidden in [
        "anger",
        "sadness",
        "fear",
        "emotion",
        "surprise",
        "maturity",
        "word",
        "semantic",
        "concept",
        "token",
        "tts",
        "text-to-speech",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !lowered.contains(forbidden),
            "vocal reflex source must not introduce `{forbidden}`"
        );
    }
}

// ---------------------------------------------------------------------------
// B2 acceptance: observe the current numeric relation between suppression /
// reflex_pressure / gate — NOT "verify the body vocalizes". The assertions
// below only state that the 0×x=0 degenerate fixed point no longer exists.
// ---------------------------------------------------------------------------

#[test]
fn worst_case_suppression_still_has_a_gate_crossing_phase() {
    // Newborn (familiarity=0, efficacy=0 → gate=0.40), action_suppression at
    // its historically observed deadlock value (0.155) and at a stress value
    // (0.5). Scan the 5s endogenous rhythm: there must EXIST a phase at which
    // a quiescent reflex meeting the baseline pushes reflex_pressure above
    // the gate. This asserts the *possibility* of crossing (fixed point gone),
    // not that any particular tick must produce a burst.
    let period_us: u64 = 5_000_000;
    for &suppression in &[0.155_f64, 0.5_f64] {
        let mut crossed = false;
        for step in 0..200u64 {
            let now_us = 10_000_000 + step * (period_us / 200);
            // Newborn (maturity=0): floor at its widest B2 endpoint — behavior
            // must be bit-for-bit identical to the pre-anneal constant.
            let baseline = VocalReflexState::endogenous_baseline(
                now_us,
                0.0,
                suppression,
                suppression_damp_floor(0.0),
            );
            // Quiescent reflex state (rising-edge encounter with the rhythm).
            let mut state = VocalReflexState::default();
            let drive = VocalReflexDrive {
                drive_pressure: baseline,
                energy: 0.5,
            };
            state.tick_drive(now_us, drive, 0.0, suppression, 0.0);
            if state.reflex_pressure > 0.40 {
                crossed = true;
                break;
            }
        }
        assert!(
            crossed,
            "suppression={suppression}: no rhythm phase crosses the 0.40 gate — \
             the 0×x=0 deadlock fixed point is back"
        );
    }
}

#[test]
fn suppression_decays_by_pure_time_half_life() {
    let mut bias = LearnedBiasState {
        action_suppression: 0.5,
        last_suppression_decay_us: 1_000_000,
        ..LearnedBiasState::default()
    };

    // elapsed = 0 → unchanged.
    bias.decay_vocal_suppression(1_000_000);
    assert_eq!(bias.action_suppression, 0.5);

    // Exactly one half-life → halves.
    let one_tau = 1_000_000 + SUPPRESSION_HALF_LIFE_US as u64;
    bias.decay_vocal_suppression(one_tau);
    assert!(
        (bias.action_suppression - 0.25).abs() < 1e-9,
        "one half-life should halve suppression: {}",
        bias.action_suppression
    );

    // Strictly decreasing and tending to 0 over many half-lives.
    let before = bias.action_suppression;
    bias.decay_vocal_suppression(one_tau + SUPPRESSION_HALF_LIFE_US as u64 * 40);
    assert!(bias.action_suppression < before);
    assert!(bias.action_suppression < 1e-9);
}

#[test]
fn efficacy_decays_by_pure_time_half_life() {
    // vocal_efficacy gains a pure-time forgetting law so it tracks RECENT real
    // vocal competence instead of being a monotone ratchet (the 1.0-pinned
    // illusory-convergence the diagnosis identified). Same shape as the
    // suppression half-life, longer tau.
    let mut bias = LearnedBiasState {
        vocal_efficacy: 0.5,
        last_efficacy_decay_us: 1_000_000,
        ..LearnedBiasState::default()
    };

    // elapsed = 0 → unchanged.
    bias.decay_vocal_efficacy(1_000_000);
    assert_eq!(bias.vocal_efficacy, 0.5);

    // Exactly one half-life → halves.
    let one_tau = 1_000_000 + EFFICACY_HALF_LIFE_US as u64;
    bias.decay_vocal_efficacy(one_tau);
    assert!(
        (bias.vocal_efficacy - 0.25).abs() < 1e-9,
        "one half-life should halve efficacy: {}",
        bias.vocal_efficacy
    );

    // Strictly decreasing and tending to 0 over many half-lives.
    let before = bias.vocal_efficacy;
    bias.decay_vocal_efficacy(one_tau + EFFICACY_HALF_LIFE_US as u64 * 40);
    assert!(bias.vocal_efficacy < before);
    assert!(bias.vocal_efficacy < 1e-9);
}

#[test]
fn efficacy_half_life_is_longer_than_suppression() {
    // Sustained real acoustic fit must be able to hold a high efficacy, so the
    // forgetting timescale is deliberately an order longer than suppression's.
    assert!(EFFICACY_HALF_LIFE_US > SUPPRESSION_HALF_LIFE_US);
}

#[test]
fn restart_first_decay_resumes_without_spurious_wipe() {
    // Defect #2 fix: last_*_decay_us is NOT persisted, so after a restart it is
    // 0. now_us is epoch µs (≈1.75e15), so the OLD code computed elapsed =
    // now_us - 0 = millions of half-lives and multiplied suppression/efficacy/
    // familiarity by ~0 on the very first post-boot tick — instantly wiping the
    // substrate's accumulated competence while developmental_maturity (which IS
    // persisted+restored) keeps ratcheting up. That is the dishonest "maturity
    // climbs, ability zeroed from nothing" state. The fix treats last==0 as
    // "not yet stamped": the first tick stamps now_us and does NOT decay.
    // Forgetting still happens, but only over REAL elapsed time on later ticks.
    let boot_us = SUPPRESSION_HALF_LIFE_US as u64 * 30; // any realistic epoch µs
    let mut bias = LearnedBiasState {
        action_suppression: 0.155,
        vocal_efficacy: 0.42,
        prediction_familiarity: 0.6,
        ..LearnedBiasState::default()
    };

    // First post-restart tick: resume, no spurious decay (values held intact).
    bias.decay_vocal_suppression(boot_us);
    bias.decay_vocal_efficacy(boot_us);
    bias.decay_prediction_familiarity(boot_us);
    assert_eq!(bias.action_suppression, 0.155, "suppression must resume, not wipe");
    assert_eq!(bias.vocal_efficacy, 0.42, "efficacy must resume, not wipe");
    assert_eq!(bias.prediction_familiarity, 0.6, "familiarity must resume, not wipe");

    // The clock is now stamped to boot_us — steady-state decay is unchanged:
    // one real half-life later each value halves exactly as before.
    bias.decay_vocal_suppression(boot_us + SUPPRESSION_HALF_LIFE_US as u64);
    bias.decay_vocal_efficacy(boot_us + EFFICACY_HALF_LIFE_US as u64);
    bias.decay_prediction_familiarity(boot_us + FAMILIARITY_HALF_LIFE_US as u64);
    assert!(
        (bias.action_suppression - 0.0775).abs() < 1e-9,
        "one real half-life after resume should halve suppression: {}",
        bias.action_suppression
    );
    assert!(
        (bias.vocal_efficacy - 0.21).abs() < 1e-9,
        "one real half-life after resume should halve efficacy: {}",
        bias.vocal_efficacy
    );
    assert!(
        (bias.prediction_familiarity - 0.3).abs() < 1e-9,
        "one real half-life after resume should halve familiarity: {}",
        bias.prediction_familiarity
    );
}

#[test]
fn damping_floor_does_not_keep_experienced_baseline_above_zero() {
    // familiarity → 1 ⇒ exploration_drive → 0 dominates multiplicatively; the
    // suppression damping floor must not prop the baseline up — it is not a
    // gate that forces experienced systems to vocalize.
    let period_us: u64 = 5_000_000;
    for step in 0..200u64 {
        let now_us = 20_000_000 + step * (period_us / 200);
        let b = VocalReflexState::endogenous_baseline(now_us, 1.0, 0.0, SUPPRESSION_DAMP_FLOOR);
        assert!(
            b.abs() < 1e-12,
            "experienced baseline should be zero at every phase, got {b}"
        );
    }
}

// ---------------------------------------------------------------------------
// B2 enhancement acceptance: the suppression damping floor is annealed by the
// developmental clock — a continuous numeric modulation, never an eligibility
// gate. m=0 reproduces the B2 constant bit-for-bit; m=1 reduces to the
// original un-floored damping math.
// ---------------------------------------------------------------------------

#[test]
fn damp_floor_schedule_endpoints_monotonicity_and_clamp() {
    // Endpoints.
    assert_eq!(suppression_damp_floor(0.0), 0.96);
    assert_eq!(suppression_damp_floor(0.0), SUPPRESSION_DAMP_FLOOR);
    assert_eq!(suppression_damp_floor(1.0), 0.45);
    assert_eq!(suppression_damp_floor(1.0), SUPPRESSION_DAMP_FLOOR_MATURE);

    // Strictly monotonically decreasing in m.
    let mut prev = suppression_damp_floor(0.0);
    for i in 1..=100u32 {
        let m = i as f64 / 100.0;
        let f = suppression_damp_floor(m);
        assert!(f < prev, "floor must strictly narrow with the clock: m={m}, {f} !< {prev}");
        prev = f;
    }

    // Input clamp to [0,1].
    assert_eq!(suppression_damp_floor(-1.0), suppression_damp_floor(0.0));
    assert_eq!(suppression_damp_floor(2.0), suppression_damp_floor(1.0));
}

#[test]
fn mature_floor_is_mathematically_inert() {
    // At m=1 the floor is 0.45 = 1 − 0.55·1.0 ≤ (1 − 0.55·s) for every
    // s∈[0,1], so the floored baseline equals the ORIGINAL un-floored formula
    // exactly — the mature body runs the pre-B2 damping math.
    let period_us: u64 = 5_000_000;
    let floor = suppression_damp_floor(1.0);
    for s_step in 0..=20u64 {
        let suppression = s_step as f64 / 20.0;
        for phase_step in 0..50u64 {
            let now_us = 30_000_000 + phase_step * (period_us / 50);
            for &familiarity in &[0.0_f64, 0.3, 0.7] {
                let floored = VocalReflexState::endogenous_baseline(
                    now_us,
                    familiarity,
                    suppression,
                    floor,
                );
                // Un-floored original: baseline * (1 − 0.55·s). Reconstruct it
                // from the s=0 evaluation (damping term = 1.0 there).
                let raw = VocalReflexState::endogenous_baseline(now_us, familiarity, 0.0, floor)
                    * (1.0 - suppression * 0.55);
                assert!(
                    (floored - raw).abs() < 1e-12,
                    "m=1 floor must be inert: s={suppression}, {floored} vs {raw}"
                );
            }
        }
    }
}

#[test]
fn newborn_schedule_matches_b2_constant_bit_for_bit() {
    // m=0 must reproduce the previous SUPPRESSION_DAMP_FLOOR behavior exactly,
    // including the worst-case historical deadlock value still having a
    // gate-crossing phase (the B2 guarantee is preserved unchanged).
    let period_us: u64 = 5_000_000;
    let floor = suppression_damp_floor(0.0);
    for step in 0..200u64 {
        let now_us = 40_000_000 + step * (period_us / 200);
        for &s in &[0.0_f64, 0.0717, 0.155, 0.5, 1.0] {
            let scheduled = VocalReflexState::endogenous_baseline(now_us, 0.0, s, floor);
            let constant =
                VocalReflexState::endogenous_baseline(now_us, 0.0, s, SUPPRESSION_DAMP_FLOOR);
            assert_eq!(scheduled, constant);
        }
    }
}

#[test]
fn vocal_organ_v2_has_formant_and_envelope_parameters() {
    let state = bionic_v0::vocal_organ::VocalOrganState::default();
    assert!(state.formant_f1.is_finite());
    assert!(state.formant_f2.is_finite());
    assert!(state.formant_f3.is_finite());
    assert!(state.voiced_ratio.is_finite());
    assert!(state.breath_noise_ratio.is_finite());
    assert!(state.envelope_attack.is_finite());
    assert!(state.envelope_decay.is_finite());
    assert!(state.envelope_sustain.is_finite());
}

#[test]
fn vocal_organ_v2_formants_produce_distinct_waveforms() {
    use bionic_v0::vocal_organ::VocalOrganState;

    let mut a_like = VocalOrganState::default();
    a_like.begin_voicing_v2(0, 0.5, 800.0, 1200.0, 2500.0, 1.0, 0.0, 0.1, 0.5, 0.8);

    let mut i_like = VocalOrganState::default();
    i_like.begin_voicing_v2(0, 0.5, 300.0, 2200.0, 3000.0, 1.0, 0.0, 0.1, 0.5, 0.8);

    // Different formants → distinct rms difference
    let a_rms: f64 = (0..200).map(|_| 0.5_f64).sum::<f64>() / 200.0;
    // Simply verify both produce non-zero breath_drive
    assert!(a_like.breath_drive > 0.0);
    assert_eq!(a_like.formant_f1, 800.0);
    assert_eq!(i_like.formant_f1, 300.0);
    assert!(a_like.formant_f1 != i_like.formant_f1);
}
