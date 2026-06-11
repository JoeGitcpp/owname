//! Finer acoustic representation — discrimination gate.
//!
//! The vocal self-supervision loop used a 16-band (VIB) shape comparison to ask
//! "does my own voice match the target?". 16 bands collapse distinct vowels /
//! syllables together (the 64→16 mel downsample averages groups of 4, erasing
//! the within-group formant detail). This test pins the reason the 64-mel path
//! exists: it must resolve a difference that the 16-band path cannot, and that
//! finer resolution must translate into LESS credit when self ≠ target.
//!
//! Construction: A and B are two 64-mel shapes that differ ONLY by swapping
//! energy WITHIN downsample groups of 4 (band 4k+0/1 ↔ band 4k+2/3) in several
//! mid/formant groups. Each group keeps the same SUM, so the 64→16 average is
//! byte-for-byte identical → 16-band shape distance ≈ 0; but the 64-mel shape
//! distance is clearly nonzero. This mimics two vowels whose coarse spectral
//! envelope matches but whose formant fine-structure differs.

use bionic_v0::vocal_self_analysis::{
    downsample_to_vib, spectral_shape_distance, spectral_shape_distance_mel64, BANDS,
};
use bionic_v0::viewport::Viewport;
use bionic_v0::VIB_BANDS;

/// Build a 64-mel shape: a broad mid-spectrum bump, with the energy inside each
/// of several groups-of-4 placed in either the LOW pair (which=0) or the HIGH
/// pair (which=1) of the group. The group sums are identical across `which`, so
/// the 64→16 downsample is identical; only the intra-group placement differs.
fn make_mel64(which: usize) -> [f64; BANDS] {
    let mut m = [0.0f64; BANDS];
    // Use mid groups 4..12 (mel bands 16..48) — the formant-bearing region.
    for g in 4..12 {
        let base = g * 4;
        // Shared coarse envelope (identical in both shapes): spread evenly across
        // the group so it survives the 64→16 average the same way for A and B.
        for k in 0..4 {
            m[base + k] += 0.5;
        }
        // Distinguishing formant detail placed in the LOW vs HIGH pair of the
        // group. The PAIR-SUM is identical across `which`, so the group average
        // (hence the 16-band downsample) is unchanged; only the intra-group
        // fine-structure differs — the formant detail 16 bands cannot see.
        if which == 0 {
            m[base] += 1.0;
            m[base + 1] += 1.0;
        } else {
            m[base + 2] += 1.0;
            m[base + 3] += 1.0;
        }
    }
    m
}

#[test]
fn mel64_resolves_what_vib16_collapses() {
    let a = make_mel64(0);
    let b = make_mel64(1);

    let d64 = spectral_shape_distance_mel64(&a, &b);

    let a16 = downsample_to_vib(&a);
    let b16 = downsample_to_vib(&b);
    let d16 = spectral_shape_distance(&a16, &b16);

    eprintln!("[mel64-discrim] 64-mel shape distance = {d64:.6}");
    eprintln!("[mel64-discrim] 16-band shape distance = {d16:.6}");

    // The whole point: 64-mel sees a real difference; 16-band is blind to it.
    assert!(d16 < 1e-9, "16-band must be (near) blind to intra-group swap: d16={d16}");
    assert!(d64 > 0.2, "64-mel must clearly resolve the formant difference: d64={d64}");
    assert!(
        d64 > d16 + 0.2,
        "64-mel distance ({d64}) must be markedly greater than 16-band ({d16})"
    );
}

#[test]
fn mel64_identical_is_zero_and_silent_is_one() {
    let a = make_mel64(0);
    assert!(spectral_shape_distance_mel64(&a, &a) < 1e-12);
    let silent = [0.0f64; BANDS];
    assert_eq!(spectral_shape_distance_mel64(&a, &silent), 1.0);
    assert_eq!(spectral_shape_distance_mel64(&silent, &a), 1.0);
}

// ── End-to-end: the finer fingerprint suppresses credit it should not give ────
//
// Drive observe_vocal_reflex_closure directly with controlled self/target
// fingerprints and read the resulting vocal_efficacy. With 64-mel:
//   - self=A / target=A (a true match) earns HIGH efficacy.
//   - self=A / target=B (coarse-identical, fine-different — NOT a real match)
//     earns markedly LOWER efficacy, because the 64-mel path sees the mismatch
//     the 16-band path cannot.
//   - the same A/B pair scored on the 16-band path (no mel fingerprints staged)
//     earns HIGHER (wrongly-generous) efficacy than the 64-mel A/B path —
//     proving the finer representation is what holds back the undue credit.

fn accumulate_efficacy(
    self_mel: [f64; BANDS],
    self_vib: [f64; VIB_BANDS],
    target_mel: [f64; BANDS],
    target_vib: [f64; VIB_BANDS],
) -> f64 {
    let vp = Viewport::new();
    // Stage target (16-band + optional 64-mel) and self fingerprints directly.
    vp.observe_vocal_target_full(1, 0.6, 0.4, 0.7, 0.6, target_vib, target_mel);
    // Inject self fingerprints (bypassing PCM synthesis) so the closure compares
    // the exact shapes we constructed.
    *vp.self_vocal_bands.write().unwrap() = (self_vib, 1);
    *vp.self_vocal_mel64.write().unwrap() = (self_mel, 1);

    // Repeatedly close with full support so the efficacy EMA settles toward fit.
    let mut t = 10u64;
    for _ in 0..400 {
        vp.observe_vocal_reflex_closure(t, 1.0, true);
        // re-assert self fingerprints (closure does not clear them, but the
        // target decay tick is not run here so they persist anyway).
        t += 1_000;
    }
    vp.read_learned_bias().vocal_efficacy
}

#[test]
fn mel64_match_beats_mel64_mismatch_and_undercuts_vib16() {
    let a_mel = make_mel64(0);
    let b_mel = make_mel64(1);
    let a_vib = downsample_to_vib(&a_mel);
    let b_vib = downsample_to_vib(&b_mel); // == a_vib byte-for-byte

    // 64-mel, true match (self=A, target=A)
    let eff_match = accumulate_efficacy(a_mel, a_vib, a_mel, a_vib);
    // 64-mel, fine mismatch (self=A, target=B) — should earn much less credit
    let eff_mismatch64 = accumulate_efficacy(a_mel, a_vib, b_mel, b_vib);
    // 16-band only path on the SAME A/B (no mel fingerprints) — wrongly generous
    let zero_mel = [0.0f64; BANDS];
    let eff_mismatch16 = accumulate_efficacy(zero_mel, a_vib, zero_mel, b_vib);

    eprintln!("[mel64-credit] efficacy match(A,A)      = {eff_match:.6}");
    eprintln!("[mel64-credit] efficacy mismatch64(A,B) = {eff_mismatch64:.6}");
    eprintln!("[mel64-credit] efficacy mismatch16(A,B) = {eff_mismatch16:.6}");

    assert!(
        eff_match > eff_mismatch64 + 0.1,
        "true match must earn clearly more efficacy than the 64-mel mismatch: match={eff_match} mismatch64={eff_mismatch64}"
    );
    assert!(
        eff_mismatch64 < eff_mismatch16 - 0.05,
        "the finer 64-mel representation must hold back credit that the 16-band path wrongly grants: mismatch64={eff_mismatch64} mismatch16={eff_mismatch16}"
    );
}

#[test]
fn empty_self_fingerprint_still_gives_no_credit() {
    // No evidence (self silent) → efficacy must stay at its baseline (no credit),
    // independent of the 64-mel fingerprint path. Guards the evidence gate.
    let vp = Viewport::new();
    let target_mel = make_mel64(1);
    let target_vib = downsample_to_vib(&target_mel);
    vp.observe_vocal_target_full(1, 0.6, 0.4, 0.7, 0.6, target_vib, target_mel);
    // self bands empty (default) — no heard self.
    let before = vp.read_learned_bias().vocal_efficacy;
    let mut t = 10u64;
    for _ in 0..200 {
        vp.observe_vocal_reflex_closure(t, 1.0, true);
        t += 1_000;
    }
    let after = vp.read_learned_bias().vocal_efficacy;
    assert_eq!(before, after, "empty self fingerprint must not move efficacy");
}
