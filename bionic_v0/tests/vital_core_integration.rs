//! C1 integration: the vital core, driven through the viewport the way body
//! drives it. Sensory-input continuity = supply (proto-food/safety); silence +
//! exertion = drain. Sustained silence depletes the reserve toward Standby.

use std::sync::Arc;

use bionic_v0::viewport::Viewport;
use bionic_v0::vital_core::VitalState;

#[test]
fn continuous_input_keeps_reserve_up() {
    let vp = Arc::new(Viewport::new());
    let mut t = 1_000_000u64;
    // Fresh input every tick → high supply → reserve stays healthy.
    for _ in 0..200 {
        vp.push_observation(0, t, 0.3);
        vp.tick_vital_core(t);
        t += 100_000; // 0.1s apart → always "recent"
    }
    let v = vp.read_vital_core();
    assert_eq!(v.state, VitalState::Alive, "fed body stays alive");
    assert!(v.reserve > 0.5, "continuous input keeps reserve up: {}", v.reserve);
}

#[test]
fn prolonged_silence_depletes_toward_standby() {
    let vp = Arc::new(Viewport::new());
    // One early input, then long silence (no fresh observations) → supply
    // fades to 0, baseline metabolism drains the reserve.
    vp.push_observation(0, 1_000, 0.3);
    let mut t = 20_000_000u64; // already >10s since that input → supply 0
    for _ in 0..600 {
        vp.tick_vital_core(t);
        t += 100_000;
    }
    let v = vp.read_vital_core();
    assert!(
        v.reserve < 0.5,
        "prolonged sensory silence must deplete the reserve: {}",
        v.reserve
    );
}
