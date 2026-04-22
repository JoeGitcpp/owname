use bionic_v0::viewport::Viewport;
use bionic_v0::vpc::VpcContractState;

#[test]
fn vpc_contract_state_tracks_time_domain_and_open_prediction_without_semantics() {
    let st = VpcContractState::default();
    assert!(st.publications.is_empty());
    assert!(st.open_predictions.is_empty());
    assert!(!format!("{:?}", st).contains("concept"));
}

#[test]
fn viewport_exposes_empty_vpc_contract_state_by_default() {
    let vp = Viewport::new();
    let st = vp.read_vpc_contract();
    assert!(st.publications.is_empty());
    assert!(st.open_predictions.is_empty());
}
