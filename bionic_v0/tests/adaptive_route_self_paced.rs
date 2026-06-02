//! 改造2: route promotion is self-paced by the route's own noise, not a fixed
//! "exactly 4 stable cycles" count.
//!
//! Old: `stable_run >= 4` — a magic number; a calm route needed the same 4
//! confirmations as a noisy one. New: the required confirmations scale with the
//! route's err_ema (its noise floor). A low-noise route is trusted sooner; a
//! noisy route must prove itself longer. Like a person: you relax quickly in a
//! reliable place, double-check in a chaotic one.
//!
//! Contract: after the SAME number of good cycles, a low-noise route reaches a
//! higher fast_path_strength than a high-noise route (it was promoted sooner).

use bionic_v0::adaptive_stability::{AdaptiveRouteSample, AdaptiveStabilityState};

fn good_sample(route_id: u64, t_us: u64, err: f64) -> AdaptiveRouteSample {
    AdaptiveRouteSample {
        route_id,
        route_code: 1,
        t_us,
        err,
        resource_cost: 0.05,
        closure_gain: 0.95,
    }
}

#[test]
fn low_noise_route_is_promoted_sooner_than_noisy_route() {
    let mut st = AdaptiveStabilityState::default();

    // Route 1: very low noise (err ~0.02). Route 2: high noise (err ~0.30),
    // but BOTH supply "good" cycles (high closure, low cost). Feed equal counts.
    let mut t = 1_000_000u64;
    for _ in 0..6 {
        st.observe_sample(good_sample(1, t, 0.02));
        st.observe_sample(good_sample(2, t, 0.30));
        t += 100_000;
    }

    let calm = st.route(1).expect("route 1").fast_path_strength;
    let noisy = st.route(2).expect("route 2").fast_path_strength;

    assert!(
        calm > noisy,
        "low-noise route must be promoted sooner (higher fast_path): calm={calm}, noisy={noisy}"
    );
}
