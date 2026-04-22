use std::sync::Arc;

use bionic_v0::predictor::Ar2Predictor;
use bionic_v0::viewport::Viewport;
use bionic_v0::vpc::{
    HorizonKind, ObservationDomainKind, PredictorLifecycleState, VpcContractState,
};
use bionic_v0::MOD_PHOT;

const VPC_FAMILY_AR2: u8 = 1;
const VPC_FAMILY_PHOT: u8 = 2;

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

#[test]
fn phot_predictor_contract_warmup_records_slot_family_trace_without_fake_confidence() {
    let vp = Arc::new(Viewport::new());
    let slot_id = vp.allocate_slot(MOD_PHOT, 10_000, 1).expect("slot");
    let observed = [0.3, 0.5, 0.1, 0.2];

    vp.update_slot_phot_with_predictor(slot_id, 10_000, &observed);

    let slot = vp.read_slot(slot_id).expect("slot snapshot");
    let phot = slot.phot.expect("phot family");
    assert!(phot.err.iter().all(|v| v.abs() < 1e-12));
    assert!(phot.precision.iter().all(|v| v.abs() < 1e-12));

    let st = vp.read_vpc_contract();
    assert!(st.predictor_traces.iter().any(|trace| {
        trace.predictor_family == VPC_FAMILY_PHOT
            && trace.domain.kind == ObservationDomainKind::SlotFamily
            && trace.domain.primary == slot_id as u16
            && trace.horizon.kind == HorizonKind::Point
            && trace.lifecycle == PredictorLifecycleState::Warmup
    }));
}

#[test]
fn phot_predictor_contract_resolves_after_warmup_with_slot_local_provenance() {
    let vp = Arc::new(Viewport::new());
    let slot_id = vp.allocate_slot(MOD_PHOT, 10_000, 1).expect("slot");
    let warm = [0.3, 0.5, 0.1, 0.2];
    let shifted = [0.6, 0.8, 0.4, 0.7];

    vp.update_slot_phot_with_predictor(slot_id, 10_000, &warm);
    vp.update_slot_phot_with_predictor(slot_id, 20_000, &warm);
    vp.update_slot_phot_with_predictor(slot_id, 30_000, &shifted);

    let slot = vp.read_slot(slot_id).expect("slot snapshot");
    let phot = slot.phot.expect("phot family");
    assert!(phot.err.iter().any(|v| v.abs() > 1e-9));
    assert!(phot.precision.iter().any(|v| *v > 0.0));

    let st = vp.read_vpc_contract();
    assert!(st.predictor_traces.iter().any(|trace| {
        trace.predictor_family == VPC_FAMILY_PHOT
            && trace.domain.kind == ObservationDomainKind::SlotFamily
            && trace.domain.primary == slot_id as u16
            && trace.horizon.kind == HorizonKind::Point
            && trace.lifecycle == PredictorLifecycleState::ResolvedPublication
    }));
}
