// TDD tests for the action-consequence loop foundation.
//
// Plan: docs/superpowers/plans/2026-04-21-action-consequence-loop-foundation.md
// Spec: docs/superpowers/specs/2026-04-21-action-consequence-loop-and-imitation-laws-design.md
//
// Run order:
//   1. Tests written first → all fail (RED)
//   2. Implementation lands → all pass (GREEN)

use std::sync::Arc;

use bionic_v0::action_consequence::{
    ActionConsequenceWindowEvent, ConsequenceModalityKind, ConsequencePairStatus,
};
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

// ─── 1. Default state: no windows, no pairs ───────────────────────────────────

#[test]
fn action_consequence_viewport_exposes_empty_state_by_default() {
    let vp = Viewport::new();
    let st = vp.read_action_consequence();

    assert_eq!(st.n_windows, 0);
    assert!(st.windows.is_empty());
    assert_eq!(st.n_pairs, 0);
    assert!(st.pairs.is_empty());
}

// ─── 2. Pending window carries only numeric/time fields ───────────────────────

#[test]
fn action_consequence_records_pending_window_without_result_semantics() {
    let vp = Viewport::new();

    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 140,
        magnitude: 0.3,
    });

    let st = vp.read_action_consequence();
    assert_eq!(st.n_windows, 1);
    assert_eq!(st.windows[0].start_t_us, 100);
    assert_eq!(st.windows[0].end_t_us, 140);
}

// ─── 3. Pair links write_seq to window_id with delay ─────────────────────────

#[test]
fn action_consequence_pairs_a_write_epoch_with_a_window() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1000,
    });

    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1100,
        magnitude: 0.25,
    });

    let wid = vp.read_action_consequence().windows[0].window_id;
    vp.pair_action_consequence(write_seq, wid, 40);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1);
    assert_eq!(st.pairs[0].write_seq, write_seq);
    assert_eq!(st.pairs[0].window_id, wid);
    assert_eq!(st.pairs[0].status, ConsequencePairStatus::Paired);
    assert_eq!(st.pairs[0].delay_us, 40);
}

// ─── 4. Snapshot roundtrip preserves windows and pairs ───────────────────────

#[test]
fn snapshot_carries_action_consequence_windows_and_pairs() {
    let vp = Arc::new(Viewport::new());
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1100,
        magnitude: 0.25,
    });
    let wid = vp.read_action_consequence().windows[0].window_id;
    vp.pair_action_consequence(write_seq, wid, 40);

    let dir = std::env::temp_dir().join("bionic_v0_action_consequence_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let snap = read_snapshot(&paths.final_path).expect("read ok");

    assert_eq!(snap.action_consequence.n_windows, 1);
    assert_eq!(snap.action_consequence.n_pairs, 1);
    assert_eq!(snap.action_consequence.pairs[0].write_seq, write_seq);
}

// ─── 6. Pairing rejects a write_seq that was never issued ────────────────────
//
// Current code: no validation → pair is added with phantom write_seq.
// After patch:  pair must be rejected and n_pairs stays 0.

#[test]
fn pair_rejects_nonexistent_write_seq() {
    let vp = Viewport::new();
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 140,
        magnitude: 0.3,
    });

    // write_seq 9999 was never issued — no Begin event has occurred
    let wid = vp.read_action_consequence().windows[0].window_id;
    vp.pair_action_consequence(9999, wid, 40);

    let st = vp.read_action_consequence();
    assert_eq!(
        st.n_pairs, 0,
        "no pair must be recorded for a phantom write_seq"
    );
}

// ─── 8. Valid pairing records pair_t_us from viewport substrate clock ─────────
//
// pair_t_us must reflect the body's shared clock_us at the moment of pairing,
// not the host wall clock. We set clock_us to a known value and assert equality.

#[test]
fn pair_records_real_pair_t_us() {
    use std::sync::atomic::Ordering;
    let vp = Viewport::new();

    let expected_clock: u64 = 7_654_321;
    vp.clock_us.store(expected_clock, Ordering::Relaxed);

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1100,
        magnitude: 0.25,
    });

    let wid = vp.read_action_consequence().windows[0].window_id;
    let ok = vp.pair_action_consequence(write_seq, wid, 40);
    assert!(ok, "valid pairing must succeed");

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "valid pairing must be recorded");
    assert_eq!(
        st.pairs[0].pair_t_us, expected_clock,
        "pair_t_us must equal viewport clock_us, not host wall clock"
    );
}

// ─── 10. pair_t_us must come from viewport clock_us, not wall clock ──────────
//
// Current code: pair_t_us = SystemTime::now() → wall-clock value >> clock_us.
// After patch:  pair_t_us must equal vp.clock_us at the time of pairing.

#[test]
fn pair_t_us_comes_from_viewport_clock_us() {
    use std::sync::atomic::Ordering;
    let vp = Viewport::new();

    // Set viewport shared substrate clock to a known, small value.
    // Wall clock is ~1.7e15 µs; substrate clock starts at 0.
    // If pair_t_us still comes from SystemTime::now() this assertion will fail.
    let substrate_time: u64 = 5_000_000;
    vp.clock_us.store(substrate_time, Ordering::Relaxed);

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1100,
        magnitude: 0.25,
    });

    let wid = vp.read_action_consequence().windows[0].window_id;
    let ok = vp.pair_action_consequence(write_seq, wid, 40);
    assert!(ok, "pairing must succeed");

    let st = vp.read_action_consequence();
    assert_eq!(
        st.pairs[0].pair_t_us, substrate_time,
        "pair_t_us must equal viewport clock_us ({substrate_time}), not wall-clock"
    );
}

// ─── 9. Unpaired window stays pending — no phantom pair record ────────────────

#[test]
fn unpaired_window_remains_pending_no_phantom_pair() {
    let vp = Viewport::new();
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 200,
        end_t_us: 300,
        magnitude: 0.5,
    });

    let st = vp.read_action_consequence();
    assert_eq!(st.n_windows, 1, "window must be recorded");
    assert_eq!(st.n_pairs, 0, "no pair must exist for an unpaired window");
    assert!(
        st.pairs.is_empty(),
        "pairs list must be empty for unpaired window"
    );
}

// ─── 11. Pair rejects write_seq in counter range but absent from epoch history ─
//
// Manipulate action_output_write_seq counter to 5 without any Begin events.
// Seqs 1–5 are all <= max_issued but never appear in write_epoch_history.
// pair_action_consequence must reject them all.

#[test]
fn pair_rejects_seq_in_counter_range_but_not_in_history() {
    use std::sync::atomic::Ordering;
    let vp = Viewport::new();
    // Advance counter without calling Begin — no epoch is recorded.
    vp.action_output_write_seq.store(5, Ordering::Relaxed);
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 200,
        magnitude: 0.4,
    });
    let wid = vp.read_action_consequence().windows[0].window_id;
    let r1 = vp.pair_action_consequence(1, wid, 10);
    let r3 = vp.pair_action_consequence(3, wid, 10);
    let r5 = vp.pair_action_consequence(5, wid, 10);
    assert!(!r1, "seq 1 must be rejected — never in epoch history");
    assert!(!r3, "seq 3 must be rejected — never in epoch history");
    assert!(!r5, "seq 5 must be rejected — never in epoch history");
    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 0, "no pairs must be recorded");
}

// ─── 5. Observer renders "acq:" with no result semantics ─────────────────────

#[test]
fn observer_renders_action_consequence_without_result_semantics() {
    let observer_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/observer.rs"))
            .expect("read observer.rs");

    assert!(observer_src.contains("acq:"));
    assert!(!observer_src.contains("success"));
    assert!(!observer_src.contains("target_object"));
    assert!(!observer_src.contains("imit"));
}

// ─── 12. begin_action_consequence_window creates an Open window ───────────────

#[test]
fn window_begins_in_open_phase() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;
    let vp = Viewport::new();

    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 150,
        magnitude: 0.4,
    });

    let st = vp.read_action_consequence();
    assert_eq!(st.n_windows, 1);
    assert_eq!(idx, 0, "first window must be at index 0");
    assert_eq!(
        st.windows[0].phase,
        ConsequenceWindowPhase::Open,
        "window must begin Open"
    );
}

// ─── 13. extend updates end_t_us and last_update_t_us ────────────────────────

#[test]
fn extend_window_updates_end_t_and_last_update_t() {
    let vp = Viewport::new();

    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 150,
        magnitude: 0.4,
    });

    let st = vp.read_action_consequence();
    let wid = st.windows[idx as usize].window_id;
    let ok = vp.extend_action_consequence_window(wid, 300, 280);
    assert!(ok, "extend must succeed for an Open window");

    let st = vp.read_action_consequence();
    assert_eq!(st.windows[0].end_t_us, 300, "end_t_us must be updated");
    assert_eq!(
        st.windows[0].last_update_t_us, 280,
        "last_update_t_us must be updated"
    );
}

// ─── 14. close transitions window to Closed phase ────────────────────────────

#[test]
fn close_window_transitions_to_closed() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;
    let vp = Viewport::new();

    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 150,
        magnitude: 0.4,
    });

    let st = vp.read_action_consequence();
    let wid = st.windows[idx as usize].window_id;
    let ok = vp.close_action_consequence_window(wid, 160);
    assert!(ok, "close must succeed");

    let st = vp.read_action_consequence();
    assert_eq!(
        st.windows[0].phase,
        ConsequenceWindowPhase::Closed,
        "window must be Closed"
    );
    assert_eq!(
        st.windows[0].last_update_t_us, 160,
        "last_update_t_us must equal close t_us"
    );
}

// ─── 15. Extending a Closed window is rejected ───────────────────────────────

#[test]
fn extending_closed_window_is_rejected() {
    let vp = Viewport::new();

    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 150,
        magnitude: 0.4,
    });
    let st = vp.read_action_consequence();
    let wid = st.windows[idx as usize].window_id;
    vp.close_action_consequence_window(wid, 155);

    let ok = vp.extend_action_consequence_window(wid, 400, 390);
    assert!(!ok, "extending a Closed window must be rejected");

    let st = vp.read_action_consequence();
    assert_eq!(
        st.windows[0].end_t_us, 150,
        "end_t_us must not change after failed extend"
    );
}

// ─── 16. Pairing an Open window is allowed ───────────────────────────────────

#[test]
fn pairing_open_window_is_allowed() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 500,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 540,
        end_t_us: 700,
        magnitude: 0.2,
    });

    let st_before = vp.read_action_consequence();
    assert_eq!(st_before.windows[0].phase, ConsequenceWindowPhase::Open);

    let wid = st_before.windows[idx as usize].window_id;
    let ok = vp.pair_action_consequence(write_seq, wid, 40);
    assert!(ok, "pairing an Open window must be allowed");

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1);
}

// ─── 17. Pairing a Closed window is also allowed ─────────────────────────────

#[test]
fn pairing_closed_window_is_allowed() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 500,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 540,
        end_t_us: 700,
        magnitude: 0.2,
    });
    let st_before = vp.read_action_consequence();
    let wid = st_before.windows[idx as usize].window_id;
    vp.close_action_consequence_window(wid, 710);

    let st_before = vp.read_action_consequence();
    assert_eq!(st_before.windows[0].phase, ConsequenceWindowPhase::Closed);

    let wid = st_before.windows[idx as usize].window_id;
    let ok = vp.pair_action_consequence(write_seq, wid, 40);
    assert!(ok, "pairing a Closed window must be allowed");

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1);
}

// ─── 18. Snapshot roundtrip preserves window phase and last_update_t_us ───────

#[test]
fn snapshot_carries_window_lifecycle_fields() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;
    let vp = Arc::new(Viewport::new());

    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 200,
        magnitude: 0.5,
    });
    let st = vp.read_action_consequence();
    let wid = st.windows[idx as usize].window_id;
    vp.extend_action_consequence_window(wid, 350, 320);
    vp.close_action_consequence_window(wid, 360);

    let dir = std::env::temp_dir().join("bionic_v0_lifecycle_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let snap = read_snapshot(&paths.final_path).expect("read ok");

    assert_eq!(snap.action_consequence.n_windows, 1);
    assert_eq!(
        snap.action_consequence.windows[0].end_t_us, 350,
        "extended end_t_us must survive snapshot"
    );
    assert_eq!(
        snap.action_consequence.windows[0].last_update_t_us, 360,
        "last_update_t_us must survive snapshot"
    );
    // phase encoding: 0 = Open, 1 = Closed
    assert_eq!(
        snap.action_consequence.windows[0].phase,
        ConsequenceWindowPhase::Closed.as_u64(),
        "Closed phase must survive snapshot"
    );
}

// ─── 19. accumulate_ingress_change begins a window when none is Open ──────────

#[test]
fn accumulate_ingress_begins_open_window_when_none_exists() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;
    let vp = Viewport::new();

    let wid = vp.accumulate_ingress_change(1000, 0.5);
    let st = vp.read_action_consequence();

    assert_eq!(st.n_windows, 1, "first ingress change must begin a window");
    assert!(wid > 0, "window_id must be non-zero");
    assert_eq!(
        st.windows[0].window_id, wid,
        "returned window_id must match the window"
    );
    assert_eq!(
        st.windows[0].phase,
        ConsequenceWindowPhase::Open,
        "new window must be Open"
    );
    assert_eq!(
        st.windows[0].start_t_us, 1000,
        "start_t_us must be set to change time"
    );
}

// ─── 20. accumulate_ingress_change extends the Open window on subsequent calls ─

#[test]
fn accumulate_ingress_extends_same_open_window_on_subsequent_changes() {
    let vp = Viewport::new();

    let wid0 = vp.accumulate_ingress_change(1000, 0.5);
    let wid1 = vp.accumulate_ingress_change(1200, 0.4);
    let wid2 = vp.accumulate_ingress_change(1400, 0.3);

    let st = vp.read_action_consequence();

    assert_eq!(
        st.n_windows, 1,
        "continuing changes must extend, not create, the window"
    );
    assert_eq!(wid0, wid1, "same window_id on second change");
    assert_eq!(wid1, wid2, "same window_id on third change");
    assert_eq!(
        st.windows[0].end_t_us, 1400,
        "end_t_us must advance to latest change time"
    );
}

// ─── 21. tick_ingress_accumulation closes quiescent Open windows ──────────────

#[test]
fn tick_closes_quiescent_open_window() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;
    let vp = Viewport::new();

    vp.accumulate_ingress_change(1000, 0.5);
    vp.accumulate_ingress_change(1100, 0.3);

    // last_update_t_us = 1100; tick at 1600 with quiescence_us = 400 → 1600 - 1100 = 500 ≥ 400
    vp.tick_ingress_accumulation(1600, 400);

    let st = vp.read_action_consequence();
    assert_eq!(
        st.windows[0].phase,
        ConsequenceWindowPhase::Closed,
        "quiescence must close the Open window"
    );
}

// ─── 22. tick does NOT close a window that is still within quiescence period ──

#[test]
fn tick_does_not_close_window_still_within_quiescence() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;
    let vp = Viewport::new();

    vp.accumulate_ingress_change(1000, 0.5);
    vp.accumulate_ingress_change(1100, 0.3);

    // last_update_t_us = 1100; tick at 1200 with quiescence_us = 400 → 1200 - 1100 = 100 < 400
    vp.tick_ingress_accumulation(1200, 400);

    let st = vp.read_action_consequence();
    assert_eq!(
        st.windows[0].phase,
        ConsequenceWindowPhase::Open,
        "window still within quiescence must remain Open"
    );
}

// ─── 23. new change after close opens a new window ───────────────────────────

#[test]
fn new_change_after_close_opens_new_window() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;
    let vp = Viewport::new();

    let wid_first = vp.accumulate_ingress_change(1000, 0.5);
    vp.tick_ingress_accumulation(2000, 400); // closes the first window

    let st_mid = vp.read_action_consequence();
    assert_eq!(
        st_mid.windows[0].phase,
        ConsequenceWindowPhase::Closed,
        "first window must be Closed"
    );

    let wid_new = vp.accumulate_ingress_change(3000, 0.6);
    let st = vp.read_action_consequence();

    assert_eq!(
        st.n_windows, 2,
        "new change after close must open a second window"
    );
    assert_ne!(
        wid_new, wid_first,
        "new window must have a different window_id"
    );
    assert_eq!(
        st.windows[1].window_id, wid_new,
        "returned window_id must match second window"
    );
    assert_eq!(
        st.windows[1].phase,
        ConsequenceWindowPhase::Open,
        "new window must be Open"
    );
    assert_eq!(
        st.windows[1].start_t_us, 3000,
        "new window starts at the new change time"
    );
}

// ─── 24. accumulation produces no consequence pairs or result semantics ────────

#[test]
fn accumulation_produces_no_pairs_or_result_semantics() {
    let vp = Viewport::new();

    vp.accumulate_ingress_change(1000, 0.5);
    vp.accumulate_ingress_change(1200, 0.4);
    vp.tick_ingress_accumulation(2000, 400);

    let st = vp.read_action_consequence();

    assert_eq!(
        st.n_pairs, 0,
        "ingress accumulation must not produce consequence pairs"
    );
    assert!(
        st.pairs.is_empty(),
        "pairs list must be empty after accumulation-only workflow"
    );
    // Window exists with only numeric/time substrate fields — no target, result, or success
    assert_eq!(st.n_windows, 1);
    let w = &st.windows[0];
    assert!(
        w.start_t_us <= w.end_t_us,
        "start must precede or equal end"
    );
    assert!(
        w.magnitude >= 0.0,
        "magnitude must be non-negative numeric value"
    );
}

// ─── Live temporal pairing bridge (Phase: live write-to-consequence) ──────────
//
// These tests cover the minimal auto-pairing helper that forms a substrate-time
// link between the most-recent write epoch and the most-recent consequence window.
// No result, success, target, or meaning semantics are introduced here.

// ─── 25. Auto-pair forms a link when epoch precedes window within time window ──

#[test]
fn live_auto_pair_forms_link_when_epoch_precedes_window_within_time_window() {
    use bionic_v0::TEMPORAL_PAIR_MAX_DELAY_US;

    let vp = Viewport::new();

    // Seed a real write epoch at t = 1_000_000.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 1_000_000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    // Consequence window arrives at t = 1_500_000 (500 ms after epoch, well within max delay).
    let window_t: u64 = 1_500_000;
    assert!(
        window_t - 1_000_000 <= TEMPORAL_PAIR_MAX_DELAY_US,
        "test premise: window must be within TEMPORAL_PAIR_MAX_DELAY_US of epoch"
    );
    vp.accumulate_ingress_change(window_t, 0.5);

    // Auto-pair: bridge forms the link automatically, no explicit seq/index needed.
    let paired = vp.try_auto_pair_latest_epoch_to_window(window_t);
    assert!(
        paired,
        "auto-pair must succeed when epoch precedes window within time window"
    );

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "exactly one pair must be formed");
    assert_eq!(
        st.pairs[0].write_seq, write_seq,
        "pair must reference the real write seq"
    );
    assert_eq!(
        st.pairs[0].window_id, st.windows[0].window_id,
        "pair must reference the first window by its stable window_id"
    );
    assert_eq!(
        st.pairs[0].delay_us,
        window_t - 1_000_000,
        "delay_us must be the time difference between epoch start and window start"
    );
}

// ─── 26. No write epoch → no phantom pair ─────────────────────────────────────

#[test]
fn live_auto_pair_no_epoch_means_no_phantom_pair() {
    let vp = Viewport::new();

    // Consequence window exists, but there is no write epoch.
    vp.accumulate_ingress_change(2_000_000, 0.4);

    let paired = vp.try_auto_pair_latest_epoch_to_window(2_000_000);
    assert!(
        !paired,
        "auto-pair must not form a pair when no write epoch exists"
    );

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 0, "n_pairs must remain 0 — no epoch, no pair");
}

// ─── 27. Stale epoch beyond engineering time window is not paired ──────────────

#[test]
fn live_auto_pair_rejects_stale_epoch_beyond_time_window() {
    use bionic_v0::TEMPORAL_PAIR_MAX_DELAY_US;

    let vp = Viewport::new();

    // Write epoch at t = 0.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 0,
    });

    // Window arrives well beyond the engineering time window.
    let stale_t: u64 = TEMPORAL_PAIR_MAX_DELAY_US + 500_000;
    vp.accumulate_ingress_change(stale_t, 0.4);

    let paired = vp.try_auto_pair_latest_epoch_to_window(stale_t);
    assert!(!paired, "auto-pair must not form a pair for a stale epoch");

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 0, "n_pairs must remain 0 for stale epoch");
}

// ─── 28. Auto-pair is idempotent — duplicate calls produce only one pair ───────

#[test]
fn live_auto_pair_idempotent_no_duplicate_pairs() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 1_000_000,
    });
    vp.accumulate_ingress_change(1_500_000, 0.5);

    let r1 = vp.try_auto_pair_latest_epoch_to_window(1_500_000);
    let r2 = vp.try_auto_pair_latest_epoch_to_window(1_500_000);

    assert!(r1, "first auto-pair call must succeed");
    assert!(
        !r2,
        "second auto-pair call for same (epoch, window) must be rejected"
    );

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "only one pair must exist — no duplicates");
}

// ─── Multi-epoch temporal pairing selection ───────────────────────────────────
//
// When the real write_epoch_history contains multiple legal predecessor epochs
// for a given consequence window, the live auto-pair bridge must pick the
// epoch with the smallest non-negative delay that still lies within the
// engineering time window. Selection uses only substrate time — no target,
// result, success/failure, or meaning semantics.

// ─── 29. Among multiple legal predecessors, select the smallest delay ─────────

#[test]
fn live_auto_pair_selects_nearest_legal_predecessor_among_multiple_epochs() {
    use bionic_v0::TEMPORAL_PAIR_MAX_DELAY_US;

    let vp = Viewport::new();

    // Three real write epochs, all strictly preceding the consequence window
    // and all within TEMPORAL_PAIR_MAX_DELAY_US of it. The middle-in-time
    // epoch (t=1_700_000) has the smallest delay to the window at 1_800_000.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_000_000,
    });
    let seq_oldest = vp.read_action_output().channels[0].current_write_seq;

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_400_000,
    });
    let seq_middle = vp.read_action_output().channels[0].current_write_seq;

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_700_000,
    });
    let seq_nearest = vp.read_action_output().channels[0].current_write_seq;

    let window_t: u64 = 1_800_000;
    assert!(
        window_t - 1_000_000 <= TEMPORAL_PAIR_MAX_DELAY_US,
        "test premise: all three epochs are within the engineering time window"
    );
    vp.accumulate_ingress_change(window_t, 0.4);

    let paired = vp.try_auto_pair_latest_epoch_to_window(window_t);
    assert!(
        paired,
        "auto-pair must form a link when legal predecessors exist"
    );

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "exactly one pair must be formed");
    assert_eq!(
        st.pairs[0].write_seq, seq_nearest,
        "must pick the epoch with the smallest delay (seq={}), not seq_middle={} or seq_oldest={}",
        seq_nearest, seq_middle, seq_oldest
    );
    assert_eq!(
        st.pairs[0].delay_us,
        window_t - 1_700_000,
        "delay must equal (window_start - nearest_epoch_start)"
    );
}

// ─── 30. An older in-history epoch must not steal a pair from a nearer one ─────

#[test]
fn live_auto_pair_older_legal_epoch_does_not_steal_from_nearer_one() {
    let vp = Viewport::new();

    // Two real write epochs. Both are within the engineering time window,
    // but epoch A is far older than epoch B.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 500_000,
    });
    let seq_old = vp.read_action_output().channels[0].current_write_seq;

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_900_000,
    });
    let seq_recent = vp.read_action_output().channels[0].current_write_seq;

    let window_t: u64 = 2_000_000;
    vp.accumulate_ingress_change(window_t, 0.4);

    let paired = vp.try_auto_pair_latest_epoch_to_window(window_t);
    assert!(
        paired,
        "auto-pair must form a link when a nearer legal predecessor exists"
    );

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "exactly one pair must be formed");
    assert_ne!(
        st.pairs[0].write_seq, seq_old,
        "older legal epoch (seq={}) must not steal the pair from the nearer one (seq={})",
        seq_old, seq_recent
    );
    assert_eq!(
        st.pairs[0].write_seq, seq_recent,
        "pair must reference the nearest legal predecessor"
    );
}

// ─── 31. All legal predecessors out of time window → no phantom pair ───────────
//
// When the ledger contains multiple epochs but none is within
// TEMPORAL_PAIR_MAX_DELAY_US of the window, no pair must be formed even though
// the "latest" epoch trivially exists.

#[test]
fn live_auto_pair_no_phantom_when_all_epochs_beyond_time_window() {
    use bionic_v0::TEMPORAL_PAIR_MAX_DELAY_US;

    let vp = Viewport::new();

    // Two epochs, both strictly preceding the window but both beyond the
    // engineering time window.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 0,
    });
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 100_000,
    });

    let window_t: u64 = TEMPORAL_PAIR_MAX_DELAY_US + 5_000_000;
    vp.accumulate_ingress_change(window_t, 0.4);

    let paired = vp.try_auto_pair_latest_epoch_to_window(window_t);
    assert!(
        !paired,
        "auto-pair must not form a phantom pair when no epoch is within the time window"
    );

    let st = vp.read_action_consequence();
    assert_eq!(
        st.n_pairs, 0,
        "no pair must be recorded when all epochs are stale"
    );
}

// ─── 32. A future epoch (start_t_us >= window_start) must not be paired ────────
//
// Temporal ordering is substrate-level and strict: any epoch whose start_t_us
// is at or after window.start_t_us is disqualified, even if it is the
// "latest" epoch in the ledger. The scan must fall back to an earlier legal
// predecessor when one exists.

#[test]
fn live_auto_pair_skips_future_epoch_and_falls_back_to_earlier_legal_one() {
    let vp = Viewport::new();

    // Older epoch strictly precedes the window within the time window.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_400_000,
    });
    let seq_pre = vp.read_action_output().channels[0].current_write_seq;

    // Consequence window arrives.
    let window_t: u64 = 1_500_000;
    vp.accumulate_ingress_change(window_t, 0.4);

    // A later epoch arrives AFTER the window starts — it is not a legal
    // predecessor for this window and must be skipped.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_600_000,
    });
    let seq_post = vp.read_action_output().channels[0].current_write_seq;

    let paired = vp.try_auto_pair_latest_epoch_to_window(window_t);
    assert!(
        paired,
        "auto-pair must still succeed via the earlier legal predecessor"
    );

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "exactly one pair must be formed");
    assert_eq!(
        st.pairs[0].write_seq, seq_pre,
        "must pair with earlier legal predecessor (seq={}), not future epoch (seq={})",
        seq_pre, seq_post
    );
}

// ─── 33b. Latest-appended epoch is stale; earlier-appended legal epoch wins ───
//
// The ledger is append-only, but selection must operate on substrate time
// (epoch.start_t_us), not on append order. Construct a case where the most
// recently *appended* epoch has a start_t_us that is already beyond the
// engineering time window, while an earlier-appended epoch has a nearer
// start_t_us that still precedes the window within the time window.
//
// The current "take last epoch" heuristic rejects in this case; the
// multi-epoch scan must succeed.

#[test]
fn live_auto_pair_latest_appended_stale_falls_back_to_earlier_legal_epoch() {
    use bionic_v0::TEMPORAL_PAIR_MAX_DELAY_US;

    let vp = Viewport::new();

    // First append: epoch_A at start_t_us = 10_000_000 — will be a legal
    // predecessor to a window at 10_200_000.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 10_000_000,
    });
    let seq_legal = vp.read_action_output().channels[0].current_write_seq;

    // Second append: epoch_B with start_t_us DEEP in the past — it is the
    // last ledger entry but its delay to the window exceeds
    // TEMPORAL_PAIR_MAX_DELAY_US. It must be skipped.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_000,
    });
    let seq_stale = vp.read_action_output().channels[0].current_write_seq;

    // Sanity: stale epoch is indeed beyond the time window; legal epoch is within it.
    let window_t: u64 = 10_200_000;
    assert!(
        window_t - 10_000_000 <= TEMPORAL_PAIR_MAX_DELAY_US,
        "premise: legal epoch is within time window"
    );
    assert!(
        window_t - 1_000 > TEMPORAL_PAIR_MAX_DELAY_US,
        "premise: stale epoch is beyond time window"
    );

    vp.accumulate_ingress_change(window_t, 0.4);

    let paired = vp.try_auto_pair_latest_epoch_to_window(window_t);
    assert!(
        paired,
        "multi-epoch scan must succeed via the earlier-appended legal epoch"
    );

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "exactly one pair must be formed");
    assert_eq!(
        st.pairs[0].write_seq, seq_legal,
        "must pair with earlier-appended but nearest legal predecessor (seq={}), not the stale last-appended one (seq={})",
        seq_legal, seq_stale
    );
}

// ─── 33. Re-pair call is idempotent when selection would pick the same epoch ───

#[test]
fn live_auto_pair_multi_epoch_is_idempotent_for_same_selection() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_000_000,
    });
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_300_000,
    });
    let seq_nearest = vp.read_action_output().channels[0].current_write_seq;

    let window_t: u64 = 1_400_000;
    vp.accumulate_ingress_change(window_t, 0.4);

    let r1 = vp.try_auto_pair_latest_epoch_to_window(window_t);
    let r2 = vp.try_auto_pair_latest_epoch_to_window(window_t);

    assert!(
        r1,
        "first call must select and record the nearest legal predecessor"
    );
    assert!(
        !r2,
        "second call must not duplicate the same (seq, window) pair"
    );

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "only one pair must exist");
    assert_eq!(st.pairs[0].write_seq, seq_nearest);
}

// ─── Stable window_id ────────────────────────────────────────────────────────

// ─── W1. begin_action_consequence_window assigns a non-zero stable window_id ──

#[test]
fn window_id_is_assigned_on_begin() {
    let vp = Viewport::new();
    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 200,
        magnitude: 0.3,
    });
    let st = vp.read_action_consequence();
    assert_eq!(idx, 0, "first window is at Vec index 0");
    assert!(
        st.windows[0].window_id > 0,
        "window_id must be assigned (non-zero) by begin"
    );
}

// ─── W2. push_action_consequence_window also assigns a stable window_id ───────

#[test]
fn window_id_is_assigned_on_push() {
    let vp = Viewport::new();
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 200,
        magnitude: 0.3,
    });
    let st = vp.read_action_consequence();
    assert!(
        st.windows[0].window_id > 0,
        "push must also assign a non-zero window_id"
    );
}

// ─── W3. Multiple windows get strictly increasing window_ids ──────────────────

#[test]
fn window_ids_are_strictly_increasing() {
    let vp = Viewport::new();
    vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 200,
        magnitude: 0.3,
    });
    vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 300,
        end_t_us: 400,
        magnitude: 0.4,
    });
    let st = vp.read_action_consequence();
    let id0 = st.windows[0].window_id;
    let id1 = st.windows[1].window_id;
    assert!(id1 > id0, "window_ids must be strictly increasing");
}

// ─── W4. window_id is stable across multiple reads ────────────────────────────

#[test]
fn window_id_is_stable_across_reads() {
    let vp = Viewport::new();
    vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 100,
        end_t_us: 200,
        magnitude: 0.3,
    });
    let id_first = vp.read_action_consequence().windows[0].window_id;
    let id_second = vp.read_action_consequence().windows[0].window_id;
    assert_eq!(
        id_first, id_second,
        "window_id must be stable: re-reading the same window must return the same id"
    );
}

// ─── Pair references window by stable window_id, not Vec position ─────────────

// ─── WP1. pair.window_id matches the window's own window_id ───────────────────

#[test]
fn pair_references_window_by_stable_window_id() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1200,
        magnitude: 0.2,
    });

    // The explicit pair API now takes a window_id, not a Vec index.
    let wid = vp.read_action_consequence().windows[idx as usize].window_id;
    let ok = vp.pair_action_consequence(write_seq, wid, 40);
    assert!(ok, "pairing by stable window_id must succeed");

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 1, "one pair must be recorded");
    assert_eq!(
        st.pairs[0].window_id, wid,
        "pair.window_id must equal the window's stable window_id"
    );
}

// ─── WP2. pair rejects window_id 0 (null sentinel) ────────────────────────────

#[test]
fn pair_rejects_null_window_id() {
    let vp = Viewport::new();
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1040,
        end_t_us: 1200,
        magnitude: 0.2,
    });

    // window_id 0 is the null sentinel — no real window has id 0.
    let ok = vp.pair_action_consequence(write_seq, 0, 40);
    assert!(!ok, "window_id 0 must be rejected as null sentinel");

    let st = vp.read_action_consequence();
    assert_eq!(st.n_pairs, 0, "no pair must be recorded for null window_id");
}

// ─── WP3. pair rejects window_id that has never been assigned ─────────────────

#[test]
fn pair_rejects_unissued_window_id() {
    let vp = Viewport::new();
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 1000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    // No windows created yet — window_id 9999 is unissued.
    let ok = vp.pair_action_consequence(write_seq, 9999, 40);
    assert!(!ok, "unissued window_id must be rejected");

    let st = vp.read_action_consequence();
    assert_eq!(
        st.n_pairs, 0,
        "no pair must be recorded for unissued window_id"
    );
}

// ─── Epoch history cap ────────────────────────────────────────────────────────

// ─── EC1. write_epoch_history does not exceed WRITE_EPOCH_HISTORY_CAP ─────────

#[test]
fn epoch_history_stays_within_cap() {
    use bionic_v0::WRITE_EPOCH_HISTORY_CAP;

    let vp = Viewport::new();

    // Push WRITE_EPOCH_HISTORY_CAP + 10 epochs. Each Begin at a different t_us
    // so they are all within the time window (start_t_us are all the same to
    // avoid any temporal-window-based eviction; we just want to trigger cap).
    // Use very small t_us values so none exceed TEMPORAL_PAIR_MAX_DELAY_US from
    // each other.
    let n = WRITE_EPOCH_HISTORY_CAP + 10;
    for i in 0..n {
        vp.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Begin,
            magnitude: 0.1,
            event_t_us: i as u64 * 100,
        });
    }

    let st = vp.read_action_output();
    assert!(
        st.write_epoch_history.len() <= WRITE_EPOCH_HISTORY_CAP,
        "write_epoch_history len {} must not exceed WRITE_EPOCH_HISTORY_CAP {}",
        st.write_epoch_history.len(),
        WRITE_EPOCH_HISTORY_CAP
    );
    assert_eq!(
        st.n_write_epochs as usize,
        st.write_epoch_history.len(),
        "n_write_epochs must equal actual Vec length after eviction"
    );
}

// ─── EC2. Most-recent epochs are retained after eviction ──────────────────────

#[test]
fn epoch_history_retains_most_recent_after_eviction() {
    use bionic_v0::WRITE_EPOCH_HISTORY_CAP;

    let vp = Viewport::new();
    let n = WRITE_EPOCH_HISTORY_CAP + 5;
    for i in 0..n {
        vp.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Begin,
            magnitude: 0.1,
            event_t_us: i as u64 * 100,
        });
    }

    let st = vp.read_action_output();
    // The last epoch must be present (most recently appended).
    let last_seq = st.channels[0].current_write_seq;
    assert!(
        st.write_epoch_history
            .iter()
            .any(|r| r.write_seq == last_seq),
        "most recently appended epoch (seq={}) must be in history after eviction",
        last_seq
    );
    // Seq 1 (the very first) must have been evicted.
    assert!(
        !st.write_epoch_history.iter().any(|r| r.write_seq == 1),
        "seq=1 (oldest epoch) must have been evicted when cap was exceeded"
    );
}

// ─── Window and pair history cap ─────────────────────────────────────────────

// ─── WC1. Consequence windows Vec does not exceed CONSEQUENCE_WINDOW_CAP ──────

#[test]
fn consequence_windows_stay_within_cap() {
    use bionic_v0::CONSEQUENCE_WINDOW_CAP;

    let vp = Viewport::new();

    // Push CONSEQUENCE_WINDOW_CAP + 5 windows. Use accumulate + tick to get
    // Open→Closed cycles. Each window gets its own start_t_us.
    let n = CONSEQUENCE_WINDOW_CAP + 5;
    for i in 0..n {
        let base_t = i as u64 * 2_000;
        vp.accumulate_ingress_change(base_t, 0.3);
        // Close it immediately via quiescence tick.
        vp.tick_ingress_accumulation(base_t + 1_000, 500);
    }

    let st = vp.read_action_consequence();
    assert!(
        st.windows.len() <= CONSEQUENCE_WINDOW_CAP,
        "windows len {} must not exceed CONSEQUENCE_WINDOW_CAP {}",
        st.windows.len(),
        CONSEQUENCE_WINDOW_CAP
    );
    assert_eq!(
        st.n_windows as usize,
        st.windows.len(),
        "n_windows must equal actual Vec length after eviction"
    );
}

// ─── WC2. Pairs Vec does not exceed PAIR_HISTORY_CAP ─────────────────────────

#[test]
fn consequence_pairs_stay_within_cap() {
    use bionic_v0::PAIR_HISTORY_CAP;

    let vp = Viewport::new();

    // Produce PAIR_HISTORY_CAP + 5 pairs by repeatedly beginning an epoch
    // and accumulating a consequence window, then auto-pairing.
    let n = PAIR_HISTORY_CAP + 5;
    for i in 0..n {
        let t_epoch = i as u64 * 3_000;
        let t_window = t_epoch + 500_000; // within TEMPORAL_PAIR_MAX_DELAY_US
        vp.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Begin,
            magnitude: 0.2,
            event_t_us: t_epoch,
        });
        vp.accumulate_ingress_change(t_window, 0.3);
        vp.try_auto_pair_latest_epoch_to_window(t_window);
        // Close the window so the next accumulate opens a fresh one.
        vp.tick_ingress_accumulation(t_window + 400_000, 300_000);
    }

    let st = vp.read_action_consequence();
    assert!(
        st.pairs.len() <= PAIR_HISTORY_CAP,
        "pairs len {} must not exceed PAIR_HISTORY_CAP {}",
        st.pairs.len(),
        PAIR_HISTORY_CAP
    );
    assert_eq!(
        st.n_pairs as usize,
        st.pairs.len(),
        "n_pairs must equal actual Vec length after eviction"
    );
}

// ─── WC3. begin_action_consequence_window returns a valid index after cap saturation ──

#[test]
fn begin_window_returns_valid_index_past_cap() {
    use bionic_v0::CONSEQUENCE_WINDOW_CAP;

    let vp = Viewport::new();

    // Saturate the cap using begin.
    let n = CONSEQUENCE_WINDOW_CAP + 2;
    let mut last_idx = 0u32;
    for i in 0..n {
        last_idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
            modality_kind: ConsequenceModalityKind::GenericIngress,
            start_t_us: i as u64 * 1_000,
            end_t_us: i as u64 * 1_000 + 500,
            magnitude: 0.1,
        });
    }

    let st = vp.read_action_consequence();
    // The returned index must be a valid Vec position.
    assert!(
        (last_idx as usize) < st.windows.len(),
        "begin returned idx={} but windows.len()={}; index must be in bounds after cap eviction",
        last_idx,
        st.windows.len()
    );
    // The window at that index must have the most recently assigned window_id.
    let last_wid = st.windows[last_idx as usize].window_id;
    let max_wid = st.windows.iter().map(|w| w.window_id).max().unwrap_or(0);
    assert_eq!(
        last_wid, max_wid,
        "returned idx must point to the most recently created window"
    );
}
