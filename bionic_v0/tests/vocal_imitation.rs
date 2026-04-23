use bionic_v0::vocal_imitation::VocalImitationState;

#[test]
fn imitation_pressure_rises_with_target_gap_and_drops_on_convergence() {
    let mut st = VocalImitationState::default();
    st.observe_gap(1_000, 0.9, 0.2);
    let high = st.imitation_pressure;
    st.observe_gap(2_000, 0.3, 0.2);
    assert!(st.imitation_pressure < high);
}