use std::sync::Arc;

use bionic_v0::predictor::Ar2Predictor;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
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

#[test]
fn snapshot_and_observer_expose_vpc_contract_as_trace_not_world_truth() {
    let vp = Arc::new(Viewport::new());
    let mut p = Ar2Predictor::new(0);

    vp.push_observation(0, 10_000, 0.10);
    vp.push_observation(0, 20_000, 0.20);
    p.tick(&vp);

    let dir = std::env::temp_dir().join("bionic_v0_vpc_contract_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");

    let snap = read_snapshot(&paths.final_path).expect("read ok");
    assert_eq!(snap.vpc_contract.publications.len(), 1);
    assert_eq!(snap.vpc_contract.open_predictions.len(), 1);
    assert_eq!(
        snap.vpc_contract.publications[0].domain.kind,
        ObservationDomainKind::Axis
    );

    let observer_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/observer.rs"))
            .expect("read observer.rs");
    assert!(observer_src.contains("vpc:"));
    assert!(!observer_src.contains("meaning"));
    assert!(!observer_src.contains("identity"));
}

#[test]
fn vpc_contract_source_contains_no_semantic_or_observer_backflow() {
    let vpc_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/vpc.rs"))
        .expect("read vpc.rs");
    let observer_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/observer.rs"))
            .expect("read observer.rs");

    for forbidden in [
        "semantic_readiness",
        "symbolizer",
        "concept_id",
        "token_id",
        "naming gate",
    ] {
        assert!(
            !vpc_src.contains(forbidden),
            "vpc.rs must not contain `{}`",
            forbidden
        );
    }

    assert!(
        !observer_src.contains("body truth"),
        "observer must not claim to define body truth"
    );
}

#[test]
fn predictor_publications_do_not_write_observation_truth() {
    let vp = Arc::new(Viewport::new());
    let mut p = Ar2Predictor::new(0);

    vp.push_observation(0, 10_000, 0.10);
    vp.push_observation(0, 20_000, 0.20);
    let source_writes_before = vp.source_writes.load(std::sync::atomic::Ordering::Relaxed);

    p.tick(&vp);

    let source_writes_after = vp.source_writes.load(std::sync::atomic::Ordering::Relaxed);
    assert_eq!(
        source_writes_after, source_writes_before,
        "predictor publications must not increase observation truth write count"
    );
}
