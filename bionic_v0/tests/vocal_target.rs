use bionic_v0::vocal_target::VocalTargetState;

#[test]
fn vocal_target_activates_from_external_contour_and_decays_when_idle() {
    let mut st = VocalTargetState::default();
    st.observe_external_contour(1_000, 0.8, 0.45, 0.30, 0.12);
    assert!(st.active_energy > 0.0);

    st.tick_decay(5_000_000);
    assert!(st.active_energy < 0.8);
}