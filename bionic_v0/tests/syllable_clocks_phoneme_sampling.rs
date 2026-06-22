//! Step-3 fix (2026-06-22): the continuous syllable-frame oscillator clocks one
//! phoneme-motor sample per opening, so the inventory is populated by live voicing
//! (and the competition + inter-prototype repulsion actually fire).
//!
//! Regression for the dead-gate finding: phoneme sampling (record_voiced) used to
//! fire only on a fresh-utterance organ transition, which a continuous voicing
//! stream produces almost never — so on a real 6-min call every proto was frozen
//! at its seed value (rehearsal_count all == the heal value, never incremented).

use bionic_v0::viewport::Viewport;
use bionic_v0::VIB_BANDS;
use std::sync::Arc;

const T0: u64 = 1_000_000;
const TICK: u64 = 20_000; // 50 Hz, the body sensory-loop cadence

fn hear_self(vp: &Arc<Viewport>, t: u64) {
    let samples = bionic_v0::net_adapter::sample_vocal_pcm(vp);
    vp.observe_self_vocal_pcm(t, &samples);
}

#[test]
fn syllable_oscillator_clocks_phoneme_sampling_under_continuous_voicing() {
    let vp = Arc::new(Viewport::new());
    let mut tb = [0.0f64; VIB_BANDS];
    tb[2] = 1.0;
    tb[5] = 0.6;
    tb[9] = 0.3;
    let mut tmel = [0.0f64; 64];
    for (k, m) in tmel.iter_mut().enumerate() {
        *m = (-(((k as f64) - 20.0).powi(2)) / 40.0).exp();
    }

    vp.begin_voicing(T0, 0.8, 0.5, 0.5);
    hear_self(&vp, T0);
    // 4 s of CONTINUOUS voicing ≈ 12 syllable openings at 3 Hz.
    for i in 0..200u64 {
        let t = T0 + (i + 1) * TICK;
        vp.observe_vocal_target_full(t, 0.8, 0.5, 0.5, 0.5, tb, tmel);
        vp.tick_vocal_drive_field(t);
        hear_self(&vp, t);
    }

    // sum(rehearsal_count) ≈ number of record_voiced calls (each births=1 or +=1).
    let pm = vp.phoneme_memory.read().unwrap();
    let total: u64 = pm
        .protos
        .iter()
        .filter(|p| p.active)
        .map(|p| p.rehearsal_count)
        .sum();
    let active = pm.protos.iter().filter(|p| p.active).count();
    assert!(
        total > 3,
        "the syllable oscillator must clock MANY phoneme samples under continuous \
         voicing (total rehearsals={total}, active protos={active}); the old gate \
         fired ~once and left the inventory frozen"
    );
}
