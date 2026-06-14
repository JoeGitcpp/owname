use std::sync::Arc;

use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::adapter::{ModalityAdapter, RealPhotAdapter, SyntheticVibAdapter};
use bionic_v0::language_binding::{
    BindingEvent, BindingFeedback, BindingStatus, BindingTargetRef, PatternKind,
};
use bionic_v0::phot_camera::{SynthCamStep, SyntheticCameraFrameProvider};
use bionic_v0::synthetic_vib::SyntheticVibSource;
use bionic_v0::viewport::{EventEvidence, Viewport};
use bionic_v0::{MOD_PHOT, MOD_VIB};

#[test]
fn viewport_adapter_binding_api_records_trace() {
    let vp = Arc::new(Viewport::new());
    vp.apply_binding_event_from_adapter(BindingEvent {
        pattern_kind: PatternKind::VibModality,
        pattern_hash: 0xDEAD_BEEF,
        target: BindingTargetRef::ObjectSlot { slot_id: 0 },
        feedback: BindingFeedback::Align,
        evidence_ref: 1,
        event_t_us: 1_000,
    });

    let st = vp.read_language_binding();
    assert_eq!(st.n_bindings, 1);
    assert_eq!(st.traces[0].pattern_hash, 0xDEAD_BEEF);
    assert_eq!(st.traces[0].status, BindingStatus::Candidate);
}

#[test]
fn synthetic_vib_adapter_does_not_fabricate_binding_feedback() {
    let vp = Arc::new(Viewport::new());
    let src = SyntheticVibSource::new(1_000_000, 80_000, 30_000);
    let mut adapter = SyntheticVibAdapter::new(src);

    for step in 0..180u64 {
        adapter.tick(&vp, 1_000_000 + step * 5_000);
    }

    assert_eq!(
        vp.read_language_binding().n_bindings,
        0,
        "sensory adapters may submit substrate evidence, but must not invent binding feedback"
    );
}

#[test]
fn real_phot_adapter_does_not_fabricate_binding_feedback() {
    let vp = Arc::new(Viewport::new());
    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::Blob {
            cx: 0.25,
            cy: 0.50,
            r: 0.12,
            brightness: 0.95,
        },
        SynthCamStep::Blob {
            cx: 0.75,
            cy: 0.50,
            r: 0.12,
            brightness: 0.95,
        },
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    adapter.tick(&vp, 1_000_000);
    adapter.tick(&vp, 1_033_333);
    adapter.tick(&vp, 1_066_666);

    assert_eq!(
        vp.read_language_binding().n_bindings,
        0,
        "sensory adapters may submit substrate evidence, but must not invent binding feedback"
    );
}

#[test]
fn binding_maintenance_ticker_decays_stale_strength() {
    let vp = Arc::new(Viewport::new());
    let make_event = |t: u64| BindingEvent {
        pattern_kind: PatternKind::VibModality,
        pattern_hash: 0xABCD,
        target: BindingTargetRef::ObjectSlot { slot_id: 1 },
        feedback: BindingFeedback::Align,
        evidence_ref: t,
        event_t_us: t,
    };

    vp.apply_binding_event_from_adapter(make_event(1_000));
    vp.apply_binding_event_from_adapter(make_event(2_000));
    let before = vp.read_language_binding();
    let before_strength = before.traces[0].strength.value;
    assert_eq!(before.traces[0].status, BindingStatus::Reinforced);

    vp.tick_binding_maintenance(4_000_000);

    let after = vp.read_language_binding();
    assert_eq!(after.n_bindings, 1);
    assert!(
        after.traces[0].strength.value <= before_strength,
        "maintenance ticker should not strengthen stale traces"
    );
}

#[test]
fn missing_pair_evidence_does_not_form_binding_event() {
    let vp = Arc::new(Viewport::new());

    let formed = vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xCAFE,
        BindingTargetRef::ObjectSlot { slot_id: 0 },
        1,
        1,
    );

    assert!(!formed);
    assert_eq!(vp.read_language_binding().n_bindings, 0);
}

fn claim_vib_slot(vp: &Arc<Viewport>, t: u64) -> usize {
    let ev = EventEvidence {
        modality_bit: MOD_VIB,
        event_t_us: t,
        event_energy: 0.5,
        surprise: 0.2,
        spatial: None,
        nomination_seq_hint: 1,
    };
    vp.claim_slot_for_event(&ev).slot_id().expect("slot")
}

fn paired_write_and_window(
    vp: &Arc<Viewport>,
    t: u64,
    write_mag: f64,
    window_mag: f64,
) -> (u64, u64) {
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: write_mag,
        event_t_us: t,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    let window_id = vp.accumulate_ingress_change(t + 100, window_mag);
    assert!(vp.close_action_consequence_window(window_id, t + 110));
    assert!(vp.try_auto_pair_latest_epoch_to_window(t + 120));
    (write_seq, window_id)
}

#[test]
fn strong_pair_evidence_forms_candidate_binding_for_live_slot() {
    let vp = Arc::new(Viewport::new());
    let slot_id = claim_vib_slot(&vp, 1_000);
    let (write_seq, window_id) = paired_write_and_window(&vp, 2_000, 0.7, 0.68);

    let formed = vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xA11CE,
        BindingTargetRef::ObjectSlot {
            slot_id: slot_id as u8,
        },
        write_seq,
        window_id,
    );

    let bindings = vp.read_language_binding();
    assert!(formed);
    assert_eq!(bindings.n_bindings, 1);
    assert_eq!(bindings.traces[0].status, BindingStatus::Candidate);
    assert_eq!(bindings.traces[0].pattern_hash, 0xA11CE);
}

#[test]
fn pair_evidence_does_not_bind_inactive_slot_target() {
    let vp = Arc::new(Viewport::new());
    let (write_seq, window_id) = paired_write_and_window(&vp, 2_000, 0.7, 0.68);

    let formed = vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xBEEF,
        BindingTargetRef::ObjectSlot { slot_id: 0 },
        write_seq,
        window_id,
    );

    assert!(!formed);
    assert_eq!(vp.read_language_binding().n_bindings, 0);
}

#[test]
fn ambiguous_pair_similarity_is_developmental() {
    // sim = 1 - |0.6 - 0.25| = 0.65 — between the strict DEVIATE_MAX (0.30) and
    // ALIGN_MIN (0.70). Under the developmental precision schedule its handling
    // depends on the developmental clock.

    // Newborn (maturity 0): the dead zone collapses to the neutral midpoint, so
    // a rough 0.65 match yields graded credit and forms a (weak) binding — this
    // is exactly what breaks the bootstrap deadlock (0×x=0 no longer holds).
    let vp = Arc::new(Viewport::new());
    let slot_id = claim_vib_slot(&vp, 1_000);
    let (write_seq, window_id) = paired_write_and_window(&vp, 2_000, 0.6, 0.25);
    let formed_newborn = vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xC0FFEE,
        BindingTargetRef::ObjectSlot {
            slot_id: slot_id as u8,
        },
        write_seq,
        window_id,
    );
    assert!(
        formed_newborn,
        "newborn precision gives graded credit to a rough match (no dead zone)"
    );

    // Mature (maturity →1): precision tightens to the strict [0.30, 0.70] dead
    // zone, so the same ambiguous 0.65 is (correctly) not forced into a binding.
    let vp2 = Arc::new(Viewport::new());
    for _ in 0..400 {
        vp2.advance_maturity(1.0);
    }
    assert!(vp2.developmental_maturity() > 0.9, "clock should be near mature");
    let slot_id2 = claim_vib_slot(&vp2, 1_000);
    let (w2, win2) = paired_write_and_window(&vp2, 2_000, 0.6, 0.25);
    let formed_mature = vp2.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xC0FFEE,
        BindingTargetRef::ObjectSlot {
            slot_id: slot_id2 as u8,
        },
        w2,
        win2,
    );
    assert!(
        !formed_mature,
        "mature precision dead-zones an ambiguous 0.65 match"
    );
    assert_eq!(vp2.read_language_binding().n_bindings, 0);
}

#[test]
fn low_pair_evidence_weakens_existing_binding_trace() {
    let vp = Arc::new(Viewport::new());
    let slot_id = claim_vib_slot(&vp, 1_000);
    let target = BindingTargetRef::ObjectSlot {
        slot_id: slot_id as u8,
    };

    let (good_write_seq, good_window_id) = paired_write_and_window(&vp, 2_000, 0.7, 0.69);
    assert!(vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xDAD,
        target,
        good_write_seq,
        good_window_id,
    ));
    let (good_write_seq_2, good_window_id_2) = paired_write_and_window(&vp, 3_000, 0.7, 0.69);
    assert!(vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xDAD,
        target,
        good_write_seq_2,
        good_window_id_2,
    ));
    let before = vp.read_language_binding().traces[0].strength.value;

    let (bad_write_seq, bad_window_id) = paired_write_and_window(&vp, 4_000, 0.9, 0.05);
    assert!(vp.try_form_binding_event_from_pair(
        PatternKind::SymbolSeq,
        0xDAD,
        target,
        bad_write_seq,
        bad_window_id,
    ));

    let after = vp.read_language_binding();
    assert_eq!(after.n_bindings, 1);
    assert_eq!(after.traces[0].status, BindingStatus::Weakened);
    assert!(after.traces[0].strength.value < before);
}

// Make a VIB slot active AND carry real band evidence (the "heard sound").
fn claim_and_voice_vib_slot(vp: &Arc<Viewport>, t: u64) -> usize {
    let sid = claim_vib_slot(vp, t);
    let observed = [0.5_f64; bionic_v0::VIB_BANDS];
    let zeros = [0.0_f64; bionic_v0::VIB_BANDS];
    vp.write_slot_vib(sid, t, &observed, &zeros, &zeros, &zeros);
    sid
}

fn claim_phot_slot(vp: &Arc<Viewport>, t: u64) -> usize {
    let ev = EventEvidence {
        modality_bit: MOD_PHOT,
        event_t_us: t,
        event_energy: 0.6,
        surprise: 0.2,
        spatial: None,
        nomination_seq_hint: 2,
    };
    vp.claim_slot_for_event(&ev).slot_id().expect("phot slot")
}

/// #11 live formation: the SAME call the body's 5 Hz binding ticker makes (NOT
/// curriculum) must form a binding when a heard sound co-occurs with a seen
/// object and a real earned pair exists — and must NOT inflate it on repeat.
#[test]
fn live_cooccurrence_forms_one_binding_and_does_not_run_away() {
    let vp = Arc::new(Viewport::new());
    let t = 2_000u64;

    let phot_slot = claim_phot_slot(&vp, t); // SEEN object
    let _vib_slot = claim_and_voice_vib_slot(&vp, t); // HEARD sound
    let _ = paired_write_and_window(&vp, t, 0.7, 0.68); // earned, recent pair

    let formed = vp.tick_form_binding_from_cooccurrence(t + 130);
    assert!(formed, "real heard+seen+paired co-occurrence must form a binding");
    let b = vp.read_language_binding();
    assert_eq!(b.n_bindings, 1);
    assert_eq!(
        b.traces[0].target,
        BindingTargetRef::ObjectSlot { slot_id: phot_slot as u8 }
    );
    assert_eq!(b.traces[0].pattern_kind, PatternKind::VibModality);
    let rc0 = b.traces[0].reinforce_count;

    // Runaway guard: re-fire the SAME pair many times (as the 5 Hz ticker would
    // while the stimulus persists). One earned pair = one event — neither the
    // trace count, the reinforce_count, nor the evidence set may inflate.
    for _ in 0..20 {
        vp.tick_form_binding_from_cooccurrence(t + 130);
    }
    let b2 = vp.read_language_binding();
    assert_eq!(b2.n_bindings, 1, "must not spawn extra traces");
    assert_eq!(
        b2.traces[0].reinforce_count, rc0,
        "a single earned pair must not inflate reinforce_count per tick"
    );
    assert_eq!(
        b2.traces[0].evidence_refs.len(),
        1,
        "one earned co-occurrence = one evidence ref"
    );
}

/// Honesty: a sound with NO seen object (no active PHOT slot) must not ground a
/// symbol, no matter how strong the heard sound or the earned pair.
#[test]
fn live_cooccurrence_forms_nothing_without_seen_object() {
    let vp = Arc::new(Viewport::new());
    let t = 2_000u64;
    let _vib_slot = claim_and_voice_vib_slot(&vp, t);
    let _ = paired_write_and_window(&vp, t, 0.7, 0.68);
    // NO PHOT slot claimed → no seen object.
    let formed = vp.tick_form_binding_from_cooccurrence(t + 130);
    assert!(!formed, "a sound with no seen object must not ground a symbol");
    assert_eq!(vp.read_language_binding().n_bindings, 0);
}
