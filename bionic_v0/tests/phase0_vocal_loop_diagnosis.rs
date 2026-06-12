//! Phase 0 确诊 (2026-06-12): does the LIVE region-B learning/servo path carry
//! signal end-to-end, or is it starved upstream?
//!
//! Background: the production telemetry (`vocal_diag.csv`) showed closures=0 and
//! fit=0, which looked like "the loop is dead". But `last_self_bands_sum`/`fit`
//! ONLY update inside the closure sink, which needs self_match>0.85 (a ≈22ms
//! window that almost never opens). The servo + imitation learning actually run
//! off `self_voiced` — a path the old telemetry never recorded. So "fit=0" was a
//! telemetry blind spot, not proof of a dead loop.
//!
//! This test drives the real loop deterministically (no mic, no gateway) in the
//! same per-tick order as the body main loop, and reads the new Phase-0 diag
//! fields. It localizes B1: if signal flows here, the live break is in the
//! FEEDING path (gateway → observe_vocal_target), not the loop mechanism.

use bionic_v0::viewport::Viewport;
use std::sync::Arc;

const T0: u64 = 1_000_000;
const TICK_US: u64 = 20_000; // 50 Hz, matches the body sensory loop

/// Synthesize the body's own voice and feed it back as self-hearing — exactly
/// what body.rs does after synthesis each tick (efference copy, no microphone).
fn hear_self(vp: &Arc<Viewport>, t: u64) {
    let samples = bionic_v0::net_adapter::sample_vocal_pcm(vp);
    vp.observe_self_vocal_pcm(t, &samples);
}

/// Capture a fixed caregiver target contour (a vowel DIFFERENT from where the
/// body starts) so there is a real shape error for the servo to work against.
fn capture_target(pitch: f64, resonance: f64) -> ([f64; bionic_v0::VIB_BANDS], [f64; 64]) {
    let vp = Arc::new(Viewport::new());
    vp.begin_voicing(T0, 0.9, pitch, resonance);
    let samples = bionic_v0::net_adapter::sample_vocal_pcm(&vp);
    vp.observe_self_vocal_pcm(T0, &samples);
    let (bands, _) = vp.read_self_vocal_bands();
    let (mel64, _) = vp.read_self_vocal_mel64();
    (bands, mel64)
}

#[test]
fn phase0_full_path_carries_signal() {
    // Caregiver target: a high vowel. Body will start voicing elsewhere and the
    // region-B drive (feedforward + servo) should pull it toward this contour.
    let (tb, tmel) = capture_target(0.75, 0.25);
    assert!(
        tmel.iter().sum::<f64>() > 1e-6,
        "sanity: a real voiced target must have a non-zero mel64 fingerprint"
    );

    let vp = Arc::new(Viewport::new());
    // Kick off voicing from a different vowel so there is an initial gap.
    vp.begin_voicing(T0, 0.8, 0.35, 0.7);
    hear_self(&vp, T0);

    println!(
        "tick  region_b  self_voiced  shape_dist  learn_gap  servo_p  servo_r  tgt_mel64Σ  efficacy"
    );
    let mut last = None;
    for i in 0..80u64 {
        let t = T0 + (i + 1) * TICK_US;
        // 1) caregiver target reaches the loop (fresh energy each tick).
        vp.observe_vocal_target_full(t, 0.8, 0.75, 0.25, 0.5, tb, tmel);
        // 2) drive field runs region B: feedforward + servo steer the organ.
        vp.tick_vocal_drive_field(t);
        // 3) body hears its own (just-updated) voice — feeds NEXT tick's error.
        hear_self(&vp, t);

        let d = *vp.vocal_diag.read().unwrap();
        if i % 8 == 0 || i == 79 {
            println!(
                "{:>4}  {:>8}  {:>11}  {:>10.4}  {:>9.4}  {:>7.3}  {:>7.3}  {:>10.4}  {:>8.4}",
                i,
                d.region_b_ticks,
                d.last_self_voiced,
                d.last_shape_dist,
                d.last_learning_gap,
                d.last_servo_pitch_off,
                d.last_servo_res_off,
                d.last_target_mel64_sum,
                vp.read_learned_bias().vocal_efficacy,
            );
        }
        last = Some(d);
    }

    let d = last.unwrap();
    // --- B1 localization assertions ---
    assert!(
        d.region_b_ticks > 0,
        "FAIL@feed: caregiver target never reached region B (active_energy<0.01) \
         — the break is upstream of the loop (ingress/feeding)."
    );
    assert!(
        d.last_target_mel64_sum > 1e-6,
        "FAIL@mel64: target_mel64 is zero on the full path — fine fingerprint \
         starved; shape_dist would silently fall back to coarse 16-band."
    );
    assert!(
        d.last_self_voiced,
        "FAIL@self-hear: body never registered its own voice (efference copy \
         broken) — servo/learning get no error signal."
    );
    assert!(
        d.last_shape_dist.is_finite() && d.last_shape_dist > 0.0,
        "FAIL@error: shape_dist not a real positive distance, got {}",
        d.last_shape_dist
    );
    // The servo should have committed at least some steering over ~1.6s.
    assert!(
        d.last_servo_pitch_off.abs() + d.last_servo_res_off.abs() > 0.0,
        "FAIL@servo: servo never committed any correction — it is not steering \
         (offsets still 0 after 80 ticks)."
    );
    println!(
        "\nPASS: region-B signal flows end-to-end. If the LIVE nursery shows \
         closures=0/fit=0 while THIS passes, the break is in the FEEDING path \
         (gateway→observe_vocal_target), not the loop mechanism."
    );
}

/// Phase 1 verification (2026-06-12): the servo/self_voiced path is now wired
/// into the evidence-gated closure sink. Driving ONLY tick_vocal_drive_field
/// (NO manual observe_vocal_reflex_closure) must now lift vocal_efficacy AND the
/// developmental clock off zero — the disconnect Phase 0 found is closed.
#[test]
fn phase1_servo_path_lifts_efficacy_and_maturity() {
    let (tb, tmel) = capture_target(0.75, 0.25);

    let vp = Arc::new(Viewport::new());
    vp.begin_voicing(T0, 0.8, 0.35, 0.7);
    hear_self(&vp, T0);

    // ~12s at 50Hz: enough for the rate-limited imitation closure (every 250ms,
    // ≈48 closures) and the 1s-capped maturity advance (≈12 advances) to act.
    for i in 0..600u64 {
        let t = T0 + (i + 1) * TICK_US;
        vp.observe_vocal_target_full(t, 0.8, 0.75, 0.25, 0.5, tb, tmel);
        vp.tick_vocal_drive_field(t); // <-- the ONLY learning driver; no manual closure
        hear_self(&vp, t);
    }

    let efficacy = vp.read_learned_bias().vocal_efficacy;
    let maturity = vp.developmental_maturity();
    let d = *vp.vocal_diag.read().unwrap();
    println!(
        "after 600 ticks: closures={} with_evidence={} efficacy={:.4} maturity={:.4}",
        d.closures, d.closures_with_evidence, efficacy, maturity
    );
    assert!(
        d.closures_with_evidence > 0,
        "imitation path must produce evidence-carrying closures via the servo \
         path (got {})",
        d.closures_with_evidence
    );
    assert!(
        efficacy > 0.0,
        "FIXED? vocal_efficacy must climb off zero through the newly-connected \
         servo path (no manual closures), got {}",
        efficacy
    );
    assert!(
        maturity > 0.0,
        "FIXED? developmental clock must advance through the servo path, got {}",
        maturity
    );
}

/// Phase 1 expressivity probe (2026-06-12): does the formant servo reduce
/// shape_dist on a HELD, reachable, OFF-LINE target? This isolates "capability
/// works" from "fails on the nursery's moving target". Target F2=1700 Hz sits
/// off the resonance line (line at reso≈0.25 gives F2≈1225) but within the
/// servo's reach (1225 + ≈585 = 1810), with F1 matching the baseline — so
/// shape_dist can fall ONLY if the independent F2 servo actually steers.
#[test]
fn phase1_formant_servo_descends_on_held_offline_target() {
    use bionic_v0::vocal_organ::{VocalOrganPhase, VocalOrganState};
    use bionic_v0::vocal_self_analysis::SelfEar;
    use bionic_v0::vocal_synthesis::VocalSynthesizer;
    let sr = bionic_v0::vocal_synthesis::VOCAL_SAMPLE_RATE as f64;

    // Build the off-line target fingerprint by synthesizing an organ with an
    // explicit F2 the resonance line cannot reach, then analysing it with the
    // SAME ear the body hears itself through.
    let mut tgt = VocalOrganState::default();
    tgt.breath_drive = 0.9;
    tgt.pitch_tension = 0.5;
    tgt.resonance_position = 0.25;
    tgt.formant_f1 = 450.0;
    tgt.formant_f2 = 1780.0; // ≈max servo reach (baseline 1225 + ≈585) → big error → headroom
    tgt.formant_f3 = 2500.0;
    tgt.phase = VocalOrganPhase::Active;
    let mut tgt_syn = VocalSynthesizer::new();
    let mut ear = SelfEar::new(sr);
    let mut tb = [0.0; bionic_v0::VIB_BANDS];
    for _ in 0..16 {
        let s = tgt_syn.synthesize(&tgt, 1.0, 512);
        tb = ear.observe(&s);
    }
    let tmel = ear.read_self_mel64();

    let vp = Arc::new(Viewport::new());
    vp.begin_voicing(T0, 0.8, 0.5, 0.25); // baseline on the line: F2≈1225 (wrong)
    hear_self(&vp, T0);

    let (mut early, mut late, mut ne, mut nl) = (0.0f64, 0.0f64, 0, 0);
    for i in 0..900u64 {
        let t = T0 + (i + 1) * TICK_US;
        vp.observe_vocal_target_full(t, 0.8, 0.5, 0.25, 0.5, tb, tmel);
        vp.tick_vocal_drive_field(t);
        hear_self(&vp, t);
        let d = *vp.vocal_diag.read().unwrap();
        // "early" = the genuinely-cold first ticks (servo has barely moved); the
        // small mel64 metric scale + fast convergence means a later window would
        // already be converged, leaving only probe jitter to compare.
        if (2..12).contains(&i) {
            early += d.last_shape_dist;
            ne += 1;
        }
        if i >= 800 {
            late += d.last_shape_dist;
            nl += 1;
        }
    }
    let (e, l) = (early / ne as f64, late / nl as f64);
    println!("held off-line target: shape_dist early={:.4} late={:.4} (Δ={:+.4})", e, l, l - e);
    assert!(
        l < e,
        "formant servo must reduce shape_dist on a HELD reachable target \
         (cold early {:.4} → converged late {:.4}); if it does NOT, the servo isn't steering",
        e, l
    );
}

/// Build a target fingerprint (16-band + 64-mel) by synthesizing an organ with
/// explicit off-line formants, analysed through the same ear the body uses.
fn synth_target(f1: f64, f2: f64) -> ([f64; bionic_v0::VIB_BANDS], [f64; 64]) {
    use bionic_v0::vocal_organ::{VocalOrganPhase, VocalOrganState};
    use bionic_v0::vocal_self_analysis::SelfEar;
    use bionic_v0::vocal_synthesis::{VocalSynthesizer, VOCAL_SAMPLE_RATE};
    let mut o = VocalOrganState::default();
    o.breath_drive = 0.9;
    o.pitch_tension = 0.5;
    o.resonance_position = 0.25;
    o.formant_f1 = f1;
    o.formant_f2 = f2;
    o.formant_f3 = 2500.0;
    o.phase = VocalOrganPhase::Active;
    let mut syn = VocalSynthesizer::new();
    let mut ear = SelfEar::new(VOCAL_SAMPLE_RATE as f64);
    let mut tb = [0.0; bionic_v0::VIB_BANDS];
    for _ in 0..16 {
        let s = syn.synthesize(&o, 1.0, 512);
        tb = ear.observe(&s);
    }
    (tb, ear.read_self_mel64())
}

/// Phase 2 (2026-06-12): a target the body has ALREADY learned should match
/// faster on recurrence than the first time it was seen — content-addressed
/// recall warm-starts the servo instead of cold-climbing. This is the property
/// the live nursery (cycling, repeating targets) needs and that the pure servo
/// lacked. Protocol: learn A, learn B, then re-present A and compare the early
/// shape_dist of the SECOND A block against the first.
#[test]
fn phase2_recall_speeds_up_recurring_target() {
    // Two clearly distinct vowels (differ in BOTH F1 and F2), both within the
    // servo's reach so each can be learned and stored.
    let (a_b, a_m) = synth_target(400.0, 1250.0);
    let (b_b, b_m) = synth_target(620.0, 1780.0);
    // Sanity: the two targets are distinguishable to recall (not the same key).
    let sep = bionic_v0::vocal_self_analysis::spectral_shape_distance_mel64(&a_m, &b_m);
    println!("target separation (mel64 shape dist) = {:.4}", sep);
    assert!(
        sep > bionic_v0::vocal_target_recall::RECALL_MATCH_DIST,
        "test targets too similar to separate (sep={:.3}, match_dist={:.3})",
        sep,
        bionic_v0::vocal_target_recall::RECALL_MATCH_DIST
    );

    let vp = Arc::new(Viewport::new());
    vp.begin_voicing(T0, 0.8, 0.5, 0.25);
    hear_self(&vp, T0);

    // Drive `dur` ticks of a fixed target starting at tick offset `t_off`,
    // returning mean shape_dist over the first `early` ticks of the block.
    let run_block =
        |vp: &Arc<Viewport>, t_off: u64, dur: u64, bands: [f64; bionic_v0::VIB_BANDS], mel: [f64; 64], early: u64| -> f64 {
            let (mut acc, mut n) = (0.0f64, 0u64);
            for i in 0..dur {
                let t = T0 + (t_off + i + 1) * TICK_US;
                vp.observe_vocal_target_full(t, 0.8, 0.5, 0.25, 0.5, bands, mel);
                vp.tick_vocal_drive_field(t);
                hear_self(vp, t);
                if i < early {
                    acc += vp.vocal_diag.read().unwrap().last_shape_dist;
                    n += 1;
                }
            }
            acc / n as f64
        };

    let early_a_first = run_block(&vp, 0, 350, a_b, a_m, 12); // A, cold
    let _learn_b = run_block(&vp, 350, 350, b_b, b_m, 12); // B, servo moves away
    let early_a_second = run_block(&vp, 700, 350, a_b, a_m, 12); // A again → recall

    println!(
        "recurring target A: early shape_dist  first={:.4}  second={:.4}  (Δ={:+.4})",
        early_a_first, early_a_second, early_a_second - early_a_first
    );
    let entries = vp.vocal_target_recall.read().unwrap().entries.len();
    println!("recall entries learned: {}", entries);
    assert!(
        entries >= 1,
        "the body must have memorised at least one target→correction"
    );
    assert!(
        early_a_second < early_a_first,
        "recurring target A should match FASTER the second time via recall \
         (first={:.4}, second={:.4})",
        early_a_first, early_a_second
    );
}

#[test]
fn phase0_parametric_path_starves_mel64() {
    // The coarse parametric ingress (observe_vocal_target, no raw mel frame)
    // leaves target_mel64 = zeros. This documents the suspected live failure
    // mode: shape_dist silently degrades to the coarse 16-band proxy that
    // collapses distinct vowels — exactly the "illusory convergence" risk.
    let vp = Arc::new(Viewport::new());
    vp.begin_voicing(T0, 0.8, 0.5, 0.5);
    hear_self(&vp, T0);

    for i in 0..10u64 {
        let t = T0 + (i + 1) * TICK_US;
        // NOTE: parametric path — no mel64 fingerprint supplied.
        vp.observe_vocal_target(t, 0.8, 0.5, 0.5, 0.5);
        vp.tick_vocal_drive_field(t);
        hear_self(&vp, t);
    }

    let d = *vp.vocal_diag.read().unwrap();
    assert!(
        d.region_b_ticks > 0,
        "region B should still run on the parametric path"
    );
    assert!(
        d.last_target_mel64_sum < 1e-9,
        "parametric path must leave target_mel64 zero (documents the starvation \
         mode), got {}",
        d.last_target_mel64_sum
    );
    println!(
        "CONFIRMED: parametric feeding leaves target_mel64=0 → shape_dist falls \
         back to coarse 16-band. If the nursery feeder uses this path, that is B1."
    );
}
