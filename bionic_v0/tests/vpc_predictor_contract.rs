use std::sync::Arc;

use bionic_v0::predictor::Ar2Predictor;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::vpc::{
    ErrAlignmentKind, HorizonKind, ObservationDomainKind, ObservationDomainRef, PredictionHorizon,
    PredictionPublicationTrace, PredictorLifecycleState, PublicationStatus, VpcContractState,
};
use bionic_v0::{
    MOD_PHOT, MOD_VIB, N_OBJECT_SLOTS, PHOT_BANDS, VIB_BANDS, VPC_OPEN_PREDICTION_CAP,
    VPC_PREDICTOR_TRACE_CAP, VPC_PUBLICATION_TRACE_CAP,
};

const VPC_FAMILY_AR2: u8 = 1;
const VPC_FAMILY_PHOT: u8 = 2;
const VPC_FAMILY_RELATION: u8 = 3;

fn relation_edge_idx(src: usize, dst: usize) -> usize {
    assert_ne!(src, dst);
    if dst < src {
        src * (N_OBJECT_SLOTS - 1) + dst
    } else {
        src * (N_OBJECT_SLOTS - 1) + dst - 1
    }
}

fn tick_relation_slot(vp: &Arc<Viewport>, slot_id: usize, t_us: u64) {
    let vib_obs = [0.1f64; VIB_BANDS];
    let vib_pred = [0.09f64; VIB_BANDS];
    let vib_err = [0.01f64; VIB_BANDS];
    let vib_surp = [0.5f64; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &vib_obs, &vib_pred, &vib_err, &vib_surp);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);

    let phot_obs = vec![0.2f64; PHOT_BANDS];
    let phot_pred = vec![0.18f64; PHOT_BANDS];
    let phot_err = vec![0.02f64; PHOT_BANDS];
    let phot_surp = vec![0.4f64; PHOT_BANDS];
    vp.write_slot_phot(slot_id, t_us, &phot_obs, &phot_pred, &phot_err, &phot_surp);
    vp.refresh_slot_fusion(slot_id, MOD_PHOT, t_us);
    // (Phase 3 cutover) refresh_slot_high_layer removed — ObjectHighLayer deleted.
}

fn run_relation_cycle(vp: &Arc<Viewport>, slot_a: usize, slot_b: usize, t_a: u64, t_b: u64) {
    tick_relation_slot(vp, slot_a, t_a);
    tick_relation_slot(vp, slot_b, t_b);
    let t_cycle = t_a.max(t_b);
    vp.refresh_global_attention(t_cycle);
    vp.refresh_global_relations(t_cycle);
    vp.refresh_global_relation_predictor(t_cycle);
}

#[test]
fn predictor_family_contract_state_starts_empty_and_engineering_only() {
    let st = VpcContractState::default();
    assert!(st.predictor_traces.is_empty());
    assert!(!format!("{:?}", st).contains("semantic"));
    assert!(!format!("{:?}", st).contains("concept"));
}

#[test]
fn predictor_contract_retains_bounded_trace_history_under_long_runs() {
    let mut st = VpcContractState::default();
    let domain = ObservationDomainRef {
        kind: ObservationDomainKind::Axis,
        primary: 0,
        secondary: 0,
        tertiary: 0,
    };

    for i in 0..(VPC_PREDICTOR_TRACE_CAP + VPC_PUBLICATION_TRACE_CAP + 512) {
        let publish_t_us = i as u64 * 10_000;
        let horizon = PredictionHorizon {
            kind: HorizonKind::Point,
            publish_t_us,
            target_start_t_us: publish_t_us + 10_000,
            target_end_t_us: publish_t_us + 10_000,
        };
        st.record_predictor_trace(
            VPC_FAMILY_AR2,
            domain,
            horizon,
            PredictorLifecycleState::OpenPublication,
        );
        st.record_resolved_publication(PredictionPublicationTrace {
            predictor_family: VPC_FAMILY_AR2,
            domain,
            horizon,
            alignment: ErrAlignmentKind::PointExact,
            status: PublicationStatus::Resolved,
        });
    }

    assert!(
        st.predictor_traces.len() <= VPC_PREDICTOR_TRACE_CAP,
        "predictor traces must stay bounded for overnight runs; len={}",
        st.predictor_traces.len()
    );
    assert!(
        st.publications.len() <= VPC_PUBLICATION_TRACE_CAP,
        "publication traces must stay bounded for overnight runs; len={}",
        st.publications.len()
    );
}

#[test]
fn predictor_contract_retains_bounded_open_predictions_and_resolves_recent_indices() {
    let mut st = VpcContractState::default();
    let domain = ObservationDomainRef {
        kind: ObservationDomainKind::Axis,
        primary: 0,
        secondary: 0,
        tertiary: 0,
    };

    let mut recent_target_t_us = 0;
    for i in 0..(VPC_OPEN_PREDICTION_CAP + VPC_PUBLICATION_TRACE_CAP + 32) {
        let publish_t_us = i as u64 * 10_000;
        recent_target_t_us = publish_t_us + 10_000;
        let horizon = PredictionHorizon {
            kind: HorizonKind::Point,
            publish_t_us,
            target_start_t_us: recent_target_t_us,
            target_end_t_us: recent_target_t_us,
        };
        st.record_open_publication(
            PredictionPublicationTrace {
                predictor_family: VPC_FAMILY_AR2,
                domain,
                horizon,
                alignment: ErrAlignmentKind::PointExact,
                status: PublicationStatus::Open,
            },
            VPC_FAMILY_AR2,
            domain,
            horizon,
            ErrAlignmentKind::PointExact,
        );
    }

    assert!(
        st.open_predictions.len() <= VPC_OPEN_PREDICTION_CAP,
        "open predictions must stay bounded; len={}",
        st.open_predictions.len()
    );
    assert!(
        st.publications.len() <= VPC_PUBLICATION_TRACE_CAP,
        "publication backing records must stay bounded; len={}",
        st.publications.len()
    );
    assert!(
        st.resolve_open_publication(
            VPC_FAMILY_AR2,
            domain,
            recent_target_t_us,
            recent_target_t_us,
        ),
        "most recent open prediction should still resolve after cap eviction"
    );
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
    assert!(phot.surprise.iter().all(|v| v.abs() < 1e-12));

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
    assert!(phot.surprise.iter().any(|v| *v > 0.0));

    let st = vp.read_vpc_contract();
    assert!(st.predictor_traces.iter().any(|trace| {
        trace.predictor_family == VPC_FAMILY_PHOT
            && trace.domain.kind == ObservationDomainKind::SlotFamily
            && trace.domain.primary == slot_id as u16
            && trace.horizon.kind == HorizonKind::Point
            && trace.lifecycle == PredictorLifecycleState::ResolvedPublication
    }));
}

#[test]
fn relation_predictor_contract_warmup_records_edge_local_trace_without_confidence() {
    let vp = Arc::new(Viewport::new());
    let slot_a = vp.allocate_slot(MOD_VIB, 10_000, 1).expect("slot a");
    let slot_b = vp.allocate_slot(MOD_VIB, 10_000, 2).expect("slot b");

    run_relation_cycle(&vp, slot_a, slot_b, 10_000, 10_000);

    let st = vp.read_vpc_contract();
    let edge_idx = relation_edge_idx(slot_a, slot_b) as u16;
    assert!(st.predictor_traces.iter().any(|trace| {
        trace.predictor_family == VPC_FAMILY_RELATION
            && trace.domain.kind == ObservationDomainKind::RelationEdge
            && trace.domain.primary == edge_idx
            && trace.horizon.kind == HorizonKind::Point
            && trace.lifecycle == PredictorLifecycleState::Warmup
    }));

    let gr = vp.read_global_relations();
    let edge = gr.edges[relation_edge_idx(slot_a, slot_b)];
    assert_eq!(edge.pred.relation_err, 0.0);
    assert_eq!(edge.pred.relation_surprise, 0.0);
}

#[test]
fn relation_predictor_contract_resolves_same_cycle_edge_publication() {
    let vp = Arc::new(Viewport::new());
    let slot_a = vp.allocate_slot(MOD_VIB, 10_000, 1).expect("slot a");
    let slot_b = vp.allocate_slot(MOD_VIB, 10_000, 2).expect("slot b");
    let resolve_t_us = 20_000;

    run_relation_cycle(&vp, slot_a, slot_b, 10_000, 10_000);
    run_relation_cycle(&vp, slot_a, slot_b, resolve_t_us, resolve_t_us);

    let edge_idx = relation_edge_idx(slot_a, slot_b) as u16;
    let st = vp.read_vpc_contract();
    assert!(st.publications.iter().any(|trace| {
        trace.predictor_family == VPC_FAMILY_RELATION
            && trace.domain.kind == ObservationDomainKind::RelationEdge
            && trace.domain.primary == edge_idx
            && trace.horizon.kind == HorizonKind::Point
            && trace.horizon.publish_t_us == resolve_t_us
            && trace.horizon.target_start_t_us == resolve_t_us
            && trace.horizon.target_end_t_us == resolve_t_us
            && trace.alignment == ErrAlignmentKind::PointExact
            && trace.status == PublicationStatus::Resolved
    }));
}

#[test]
fn predictor_contract_snapshot_roundtrips_family_traces_without_semantics() {
    let vp = Arc::new(Viewport::new());
    let mut p = Ar2Predictor::new(0);

    vp.push_observation(0, 10_000, 0.10);
    p.tick(&vp);

    let dir = std::env::temp_dir().join(format!("bionic_v0_predictor_contract_snap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");

    let snap = read_snapshot(&paths.final_path).expect("read ok");
    assert_eq!(snap.vpc_contract.predictor_traces.len(), 1);
    assert_eq!(
        snap.vpc_contract.predictor_traces[0].predictor_family,
        VPC_FAMILY_AR2
    );
    assert_eq!(
        snap.vpc_contract.predictor_traces[0].lifecycle,
        PredictorLifecycleState::InsufficientContext
    );

    let observer_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/observer.rs"))
            .expect("read observer.rs");
    assert!(observer_src.contains("tr_recent["));
    assert!(!observer_src.contains("meaning"));
    assert!(!observer_src.contains("identity"));
}

#[test]
fn predictor_contract_hygiene_source_avoids_semantic_and_naming_layers() {
    for path in [
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/vpc.rs"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/predictor.rs"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/phot_predictor.rs"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/observer.rs"),
    ] {
        let src = std::fs::read_to_string(path).expect("read source");
        for forbidden in ["concept_id", "token_id", "naming gate", "symbolizer"] {
            assert!(
                !src.contains(forbidden),
                "{} must not contain `{}`",
                path,
                forbidden
            );
        }
    }
}

#[test]
fn predictor_contract_hygiene_warmup_stays_open_and_non_confident() {
    let vp = Arc::new(Viewport::new());
    let phot_slot = vp.allocate_slot(MOD_PHOT, 10_000, 1).expect("phot slot");
    let relation_a = vp.allocate_slot(MOD_VIB, 10_000, 2).expect("slot a");
    let relation_b = vp.allocate_slot(MOD_VIB, 10_000, 3).expect("slot b");

    vp.update_slot_phot_with_predictor(phot_slot, 10_000, &[0.3, 0.5, 0.1, 0.2]);
    run_relation_cycle(&vp, relation_a, relation_b, 10_000, 10_000);

    let phot = vp
        .read_slot(phot_slot)
        .expect("phot slot")
        .phot
        .expect("phot");
    assert!(phot.err.iter().all(|v| v.abs() < 1e-12));
    assert!(phot.surprise.iter().all(|v| v.abs() < 1e-12));

    let relation_edge = vp.read_global_relations().edges[relation_edge_idx(relation_a, relation_b)];
    assert_eq!(relation_edge.pred.relation_err, 0.0);
    assert_eq!(relation_edge.pred.relation_surprise, 0.0);

    let st = vp.read_vpc_contract();
    assert!(
        !st.publications.iter().any(|trace| {
            (trace.predictor_family == VPC_FAMILY_PHOT
                || trace.predictor_family == VPC_FAMILY_RELATION)
                && trace.status == PublicationStatus::Resolved
                && trace.horizon.publish_t_us == 10_000
        }),
        "warmup ticks must not look like resolved predictor publications"
    );
}
