use std::sync::Arc;

use bionic_v0::curriculum::{run_mark_binding_curriculum, CurriculumConfig};
use bionic_v0::language_binding::{BindingFeedback, BindingTargetRef, PatternKind};
use bionic_v0::mark_disturbance::MARK_CARRIER_BYTE;
use bionic_v0::viewport::{EventEvidence, Viewport};
use bionic_v0::MOD_VIB;

fn claim_live_slot(vp: &Arc<Viewport>, t_us: u64, hint: u64) -> usize {
    vp.claim_slot_for_event(&EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t_us,
        event_energy: 0.6,
        surprise: 0.3,
        spatial: None,
        nomination_seq_hint: hint,
    })
    .slot_id()
    .expect("slot")
}

#[test]
fn byte_mark_observation_records_recurrence_stability_and_hash() {
    let vp = Arc::new(Viewport::new());

    let first = vp.observe_mark_bytes(7, 1_000, 1_100, b"alpha", 1.0);
    let second = vp.observe_mark_bytes(7, 2_000, 2_100, b"alpha", 1.0);
    let st = vp.read_mark_disturbance();

    assert_eq!(first, second);
    assert_eq!(st.n_traces, 1);
    assert_eq!(st.traces[0].carrier_kind_code, MARK_CARRIER_BYTE);
    assert_eq!(st.traces[0].recurrence_count, 2);
    assert!(st.traces[0].mark_hash != 0);
    assert!(st.traces[0].stability > 0.5);
    assert!(st.traces[0].ambiguity < 0.5);
}

#[test]
fn aligned_mark_events_strengthen_symbol_binding_for_live_object_slot() {
    let vp = Arc::new(Viewport::new());
    let slot_id = claim_live_slot(&vp, 1_000, 11);

    vp.observe_mark_bytes(7, 2_000, 2_100, b"alpha", 1.0);
    assert!(vp.bind_latest_mark_to_target(
        BindingTargetRef::ObjectSlot {
            slot_id: slot_id as u8,
        },
        BindingFeedback::Align,
        2_200,
    ));
    let first_strength = vp.read_language_binding().traces[0].strength.value;

    vp.observe_mark_bytes(7, 3_000, 3_100, b"alpha", 1.0);
    assert!(vp.bind_latest_mark_to_target(
        BindingTargetRef::ObjectSlot {
            slot_id: slot_id as u8,
        },
        BindingFeedback::Align,
        3_200,
    ));

    let bindings = vp.read_language_binding();
    assert_eq!(bindings.n_bindings, 1);
    assert_eq!(bindings.traces[0].pattern_kind, PatternKind::SymbolSeq);
    assert!(bindings.traces[0].strength.value > first_strength);
}

#[test]
fn contradictory_mark_target_evidence_raises_ambiguity_without_rewriting_slots() {
    let vp = Arc::new(Viewport::new());
    let slot_a = claim_live_slot(&vp, 1_000, 11);
    let slot_b = claim_live_slot(&vp, 2_000, 22);

    vp.observe_mark_bytes(7, 3_000, 3_100, b"alpha", 1.0);
    assert!(vp.bind_latest_mark_to_target(
        BindingTargetRef::ObjectSlot {
            slot_id: slot_a as u8,
        },
        BindingFeedback::Align,
        3_200,
    ));
    let before = vp.read_mark_disturbance().traces[0].ambiguity;

    assert!(vp.bind_latest_mark_to_target(
        BindingTargetRef::ObjectSlot {
            slot_id: slot_b as u8,
        },
        BindingFeedback::Deviate,
        3_400,
    ));
    let after = vp.read_mark_disturbance();

    assert!(after.traces[0].ambiguity > before);
    assert!(vp.read_slot(slot_a).unwrap().active);
    assert!(vp.read_slot(slot_b).unwrap().active);
}

#[test]
fn mark_binding_curriculum_proves_hold_and_reentry_against_cold_baseline() {
    let evidence = run_mark_binding_curriculum(CurriculumConfig::default()).expect("course runs");

    assert_eq!(evidence.train_episodes, 4);
    assert_eq!(evidence.reentry_episodes, 2);
    assert!(evidence.train_recurrence >= 4);
    assert!(evidence.train_binding_strength > 0.5);
    assert!(
        evidence.hold_binding_strength_ratio >= 0.80,
        "hold should preserve most mark binding strength"
    );
    assert!(
        evidence.reentry_persistent_strength > evidence.reentry_cold_strength,
        "reentry should recover stronger than cold baseline"
    );
    assert!(evidence.final_ambiguity <= evidence.train_ambiguity + 0.25);
}

#[test]
fn mark_disturbance_sources_do_not_introduce_rejected_language_ontology() {
    let sources = [
        include_str!("../src/mark_disturbance.rs"),
        include_str!("../src/curriculum.rs"),
        include_str!("../src/bin/curriculum_report.rs"),
    ];

    for src in sources {
        for forbidden in [
            "semantic_readiness",
            "SymbolizerState",
            "concept_id",
            "token_id",
            "word_id",
            "object_name",
            "text_understanding",
            "name_gate",
            "println!(\"body",
            "eprintln!",
        ] {
            assert!(
                !src.contains(forbidden),
                "mark disturbance path must not contain `{}`",
                forbidden
            );
        }
    }
}
