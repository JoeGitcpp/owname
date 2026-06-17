use std::sync::Arc;

use bionic_v0::language_binding::{
    BindingEvent, BindingFeedback, BindingStatus, BindingStrength, BindingTargetRef, BindingTrace,
    LanguageBindingState, PatternKind,
};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn language_binding_state_tracks_traces_not_concept_ids() {
    let st = LanguageBindingState::default();
    assert_eq!(st.n_bindings, 0);
    assert!(!format!("{:?}", st).contains("concept"));
}

#[test]
fn language_binding_trace_uses_numeric_patterns_and_reversible_evidence() {
    let trace = BindingTrace {
        pattern_kind: PatternKind::PhotModality,
        pattern_hash: 17,
        target: BindingTargetRef::ObjectSlot { slot_id: 2 },
        strength: BindingStrength::new(0.25),
        status: BindingStatus::Candidate,
        evidence_refs: vec![11, 13],
        reinforce_count: 0,
        weaken_count: 0,
        last_update_t_us: 99,
    };

    assert!(matches!(trace.pattern_kind, PatternKind::PhotModality));
    assert!(matches!(
        trace.target,
        BindingTargetRef::ObjectSlot { slot_id: 2 }
    ));
    assert_eq!(trace.status, BindingStatus::Candidate);
    assert_eq!(trace.evidence_refs, vec![11, 13]);
}

#[test]
fn language_binding_viewport_exposes_binding_state_without_readiness_gates() {
    let vp = Viewport::new();
    let st = vp.read_language_binding();
    assert_eq!(st.n_bindings, 0);
    assert!(st.traces.is_empty());
}

fn sample_binding_event(
    feedback: BindingFeedback,
    evidence_ref: u64,
    event_t_us: u64,
) -> BindingEvent {
    BindingEvent {
        pattern_kind: PatternKind::PhotModality,
        pattern_hash: 17,
        target: BindingTargetRef::ObjectSlot { slot_id: 2 },
        feedback,
        evidence_ref,
        event_t_us,
    }
}

#[test]
fn language_binding_event_learning_is_reversible() {
    let mut st = LanguageBindingState::default();

    st.apply_binding_event(sample_binding_event(BindingFeedback::Align, 11, 100));
    assert_eq!(st.n_bindings, 1);
    assert_eq!(st.traces[0].status, BindingStatus::Candidate);
    assert_eq!(st.traces[0].evidence_refs, vec![11]);

    st.apply_binding_event(sample_binding_event(BindingFeedback::Align, 13, 200));
    let reinforced_strength = st.traces[0].strength.value;
    assert_eq!(st.traces[0].status, BindingStatus::Reinforced);
    assert!(reinforced_strength > 0.25);
    assert_eq!(st.traces[0].evidence_refs, vec![11, 13]);

    st.apply_binding_event(sample_binding_event(BindingFeedback::Deviate, 17, 300));
    assert_eq!(st.traces[0].status, BindingStatus::Weakened);
    assert!(st.traces[0].strength.value < reinforced_strength);
    assert_eq!(st.traces[0].evidence_refs, vec![11, 13, 17]);
}

#[test]
fn language_binding_viewport_applies_events_without_mutating_slots() {
    let vp = Viewport::new();
    vp.apply_language_binding_event(sample_binding_event(BindingFeedback::Align, 23, 400));

    let st = vp.read_language_binding();
    assert_eq!(st.n_bindings, 1);
    assert_eq!(st.traces[0].status, BindingStatus::Candidate);
    assert!(!vp.read_slot(0).unwrap().active);
}

#[test]
fn language_binding_layer_contains_no_rejected_architecture_terms() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/language_binding.rs"
    ))
    .expect("read language_binding.rs");

    for forbidden in [
        "semantic_readiness",
        "symbolizer",
        "concept_id",
        "token_id",
        "naming gate",
    ] {
        assert!(
            !src.contains(forbidden),
            "language_binding.rs must not contain `{}`",
            forbidden
        );
    }
}

#[test]
fn language_binding_snapshot_carries_binding_traces_not_concepts() {
    let vp = Arc::new(Viewport::new());
    vp.apply_language_binding_event(sample_binding_event(BindingFeedback::Align, 29, 500));

    let dir = std::env::temp_dir().join(format!("bionic_v0_language_binding_snap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");

    let snap = read_snapshot(&paths.final_path).expect("read ok");
    assert_eq!(snap.language_binding.n_bindings, 1);
    assert_eq!(
        snap.language_binding.traces[0].target,
        BindingTargetRef::ObjectSlot { slot_id: 2 }
    );
    assert_eq!(
        snap.language_binding.traces[0].status,
        BindingStatus::Candidate
    );
    assert!(snap.language_binding.traces[0].strength.value > 0.0);

    let observer_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/observer.rs"))
            .expect("read observer.rs");
    assert!(observer_src.contains("bind:"));
    assert!(!observer_src.contains("concept_id"));
}

#[test]
fn lookup_binding_returns_strongest_target_for_pattern() {
    // Two competing traces for the SAME (kind, hash); lookup_binding must
    // surface the stronger one. This is the "compressed direct understanding"
    // entry point: on hearing a sound the body has paired before, find the
    // most-evidenced target without re-running co-occurrence.
    let mut st = LanguageBindingState::default();
    let weak_target = BindingTargetRef::ObjectSlot { slot_id: 3 };
    let strong_target = BindingTargetRef::ObjectSlot { slot_id: 5 };
    st.push_trace(BindingTrace {
        pattern_kind: PatternKind::VibModality,
        pattern_hash: 99,
        target: weak_target,
        strength: BindingStrength::new(0.2),
        status: BindingStatus::Candidate,
        evidence_refs: vec![1],
        reinforce_count: 1,
        weaken_count: 0,
        last_update_t_us: 100,
    });
    st.push_trace(BindingTrace {
        pattern_kind: PatternKind::VibModality,
        pattern_hash: 99,
        target: strong_target,
        strength: BindingStrength::new(0.8),
        status: BindingStatus::Reinforced,
        evidence_refs: vec![2, 3, 4],
        reinforce_count: 3,
        weaken_count: 0,
        last_update_t_us: 200,
    });
    let found = st.lookup_binding(PatternKind::VibModality, 99);
    assert_eq!(found, Some(strong_target));
    assert_eq!(st.lookup_binding(PatternKind::VibModality, 12345), None);
}

#[test]
fn lookup_associated_patterns_reverses_a_rendezvous_slot() {
    // Slot 7 is the rendezvous: one VIB pattern (the sound) and one PHOT
    // pattern (the seen action) both bind to it. Reverse-lookup recovers
    // both — this is how a slot gives back the cross-modal hypothesis
    // (heard sound × seen action ⇒ both → same slot ⇒ each implies the
    // other through this lookup).
    let mut st = LanguageBindingState::default();
    let target = BindingTargetRef::ObjectSlot { slot_id: 7 };
    st.push_trace(BindingTrace {
        pattern_kind: PatternKind::VibModality,
        pattern_hash: 0xAAA,
        target,
        strength: BindingStrength::new(0.5),
        status: BindingStatus::Reinforced,
        evidence_refs: vec![1],
        reinforce_count: 2,
        weaken_count: 0,
        last_update_t_us: 100,
    });
    st.push_trace(BindingTrace {
        pattern_kind: PatternKind::PhotModality,
        pattern_hash: 0xBBB,
        target,
        strength: BindingStrength::new(0.5),
        status: BindingStatus::Reinforced,
        evidence_refs: vec![2],
        reinforce_count: 2,
        weaken_count: 0,
        last_update_t_us: 100,
    });
    // Decoy: different slot — must not show up.
    st.push_trace(BindingTrace {
        pattern_kind: PatternKind::VibModality,
        pattern_hash: 0xCCC,
        target: BindingTargetRef::ObjectSlot { slot_id: 1 },
        strength: BindingStrength::new(0.5),
        status: BindingStatus::Reinforced,
        evidence_refs: vec![3],
        reinforce_count: 1,
        weaken_count: 0,
        last_update_t_us: 100,
    });
    let assoc = st.lookup_associated_patterns(target);
    assert_eq!(assoc.len(), 2, "rendezvous slot must surface BOTH modalities");
    assert!(assoc.iter().any(|(k, h, _)| *k == PatternKind::VibModality && *h == 0xAAA));
    assert!(assoc.iter().any(|(k, h, _)| *k == PatternKind::PhotModality && *h == 0xBBB));
    // Strengths come through.
    for (_, _, s) in &assoc {
        assert!((*s - 0.5).abs() < 1e-9);
    }
}
