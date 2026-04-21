use bionic_v0::viewport::{HealthState, Viewport};
use bionic_v0::{MOD_VIB, QUIET_ENERGY_THRESHOLD, QUIET_WINDOW_US};

#[test]
fn health_defaults_to_quiet() {
    let vp = Viewport::new();
    assert_eq!(vp.read_health_state(), HealthState::Quiet);
}

#[test]
fn health_fault_when_hearing_fault_set() {
    let vp = Viewport::new();
    vp.set_hearing_fault(true);
    vp.update_health_state(1_000_000);
    assert_eq!(vp.read_health_state(), HealthState::Fault);
}

#[test]
fn health_quiet_when_observations_stale() {
    let vp = Viewport::new();
    vp.push_observation(0, 1_000_000, 1.0);
    let stale_now = 1_000_000 + QUIET_WINDOW_US + 10_000;
    vp.update_health_state(stale_now);
    assert_eq!(vp.read_health_state(), HealthState::Quiet);
}

#[test]
fn health_quiet_when_observation_fresh_but_low_energy() {
    let vp = Viewport::new();
    let t = 1_000_000;
    vp.push_observation(0, t, QUIET_ENERGY_THRESHOLD * 0.1);
    vp.update_health_state(t + 1_000);
    assert_eq!(vp.read_health_state(), HealthState::Quiet);
}

#[test]
fn health_ok_when_observation_fresh_and_active() {
    let vp = Viewport::new();
    let t = 1_000_000;
    vp.push_observation(0, t, QUIET_ENERGY_THRESHOLD * 5.0);
    vp.update_health_state(t + 1_000);
    assert_eq!(vp.read_health_state(), HealthState::Ok);
}

#[test]
fn nomination_does_not_fire_while_health_is_quiet() {
    // Feed low-energy "noise-like" VIB ambient signal. Because shared
    // substrate constants gate both health and event nomination, a signal
    // that is below the quiet/event threshold must not:
    //   1) bump nomination_seq
    //   2) flip candidate_flag
    //   3) allocate a slot
    let vp = Viewport::new();
    let t = 1_000_000;
    // Drive observed band below threshold so health stays QUIET.
    let low = QUIET_ENERGY_THRESHOLD * 0.2;
    vp.push_observation(0, t, low);
    vp.update_health_state(t + 1_000);
    assert_eq!(vp.read_health_state(), HealthState::Quiet);

    // Ambient write must treat "below substrate event threshold" as non-event.
    let energy = low * low; // energy-like scalar, still << EVENT_NOMINATION_THRESHOLD
    let is_event = energy > bionic_v0::EVENT_NOMINATION_THRESHOLD;
    assert!(
        !is_event,
        "noise-like energy must not cross the shared substrate event threshold"
    );
    vp.write_ambient_for(MOD_VIB, energy, 0.0, None, is_event);

    let amb = vp.read_ambient();
    assert_eq!(amb.nomination_seq, 0);
    assert!(!amb.vib.candidate_flag);

    // No slot should have been allocated.
    for i in 0..bionic_v0::N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        assert!(!s.active, "slot {} should stay inactive under quiet", i);
    }
}
