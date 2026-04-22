use std::sync::Arc;

use bionic_v0::predictor::Ar2Predictor;
use bionic_v0::viewport::Viewport;
use bionic_v0::vpc::{
    HorizonKind, ObservationDomainKind, PredictorLifecycleState, VpcContractState,
};

const VPC_FAMILY_AR2: u8 = 1;

#[test]
fn predictor_family_contract_state_starts_empty_and_engineering_only() {
    let st = VpcContractState::default();
    assert!(st.predictor_traces.is_empty());
    assert!(!format!("{:?}", st).contains("semantic"));
    assert!(!format!("{:?}", st).contains("concept"));
}

#[test]
fn ar2_predictor_emits_shared_family_trace_with_distinct_lifecycle_states() {
    let vp = Arc::new(Viewport::new());
    let mut p = Ar2Predictor::new(0);

    vp.push_observation(0, 10_000, 0.10);
    p.tick(&vp);

    let insufficient = vp.read_vpc_contract();
    assert!(insufficient.predictor_traces.iter().any(|trace| {
        trace.predictor_family == VPC_FAMILY_AR2
            && trace.domain.kind == ObservationDomainKind::Axis
            && trace.domain.primary == 0
            && trace.horizon.kind == HorizonKind::Point
            && trace.lifecycle == PredictorLifecycleState::InsufficientContext
    }));

    vp.push_observation(0, 20_000, 0.20);
    p.tick(&vp);

    let open = vp.read_vpc_contract();
    assert!(open.predictor_traces.iter().any(|trace| {
        trace.predictor_family == VPC_FAMILY_AR2
            && trace.domain.kind == ObservationDomainKind::Axis
            && trace.domain.primary == 0
            && trace.horizon.kind == HorizonKind::Point
            && trace.lifecycle == PredictorLifecycleState::OpenPublication
    }));

    vp.push_observation(0, 30_000, 0.30);
    p.tick(&vp);

    let resolved = vp.read_vpc_contract();
    assert!(resolved.predictor_traces.iter().any(|trace| {
        trace.predictor_family == VPC_FAMILY_AR2
            && trace.domain.kind == ObservationDomainKind::Axis
            && trace.domain.primary == 0
            && trace.horizon.kind == HorizonKind::Point
            && trace.lifecycle == PredictorLifecycleState::ResolvedPublication
    }));
}
