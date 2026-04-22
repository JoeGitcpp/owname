use std::sync::Arc;

use bionic_v0::predictor::Ar2Predictor;
use bionic_v0::viewport::Viewport;
use bionic_v0::vpc::{ObservationDomainKind, PublicationStatus, VpcContractState};

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

#[test]
fn ar2_predictor_only_emits_err_after_later_observation_arrives() {
    let vp = Arc::new(Viewport::new());
    let mut p = Ar2Predictor::new(0);

    vp.push_observation(0, 10_000, 0.10);
    vp.push_observation(0, 20_000, 0.20);
    p.tick(&vp);

    let open = vp.read_vpc_contract();
    assert_eq!(open.publications.len(), 1);
    assert_eq!(open.open_predictions.len(), 1);
    assert_eq!(open.publications[0].status, PublicationStatus::Open);
    assert_eq!(open.publications[0].domain.kind, ObservationDomainKind::Axis);
    assert_eq!(open.publications[0].domain.primary, 0);
    assert!(vp.latest_err(0).is_none());

    vp.push_observation(0, 30_000, 0.30);
    p.tick(&vp);

    let resolved = vp.read_vpc_contract();
    assert!(
        resolved
            .publications
            .iter()
            .any(|trace| trace.status == PublicationStatus::Resolved),
        "later matching observation must resolve at least one open publication"
    );
    assert!(
        vp.latest_err(0).is_some(),
        "err must appear only after later observation arrives"
    );
}

#[test]
fn ar2_predictor_contract_state_stays_axis_local() {
    let vp = Arc::new(Viewport::new());
    let mut p = Ar2Predictor::new(0);

    vp.push_observation(0, 10_000, 0.10);
    vp.push_observation(0, 20_000, 0.20);
    p.tick(&vp);

    let st = vp.read_vpc_contract();
    assert!(
        st.publications
            .iter()
            .all(|trace| trace.domain.kind == ObservationDomainKind::Axis && trace.domain.primary == 0)
    );
    assert!(
        st.open_predictions
            .iter()
            .all(|trace| trace.domain.kind == ObservationDomainKind::Axis && trace.domain.primary == 0)
    );
}
