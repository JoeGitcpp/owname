// TDD tests for the action-output substrate (channel-based design).
//
// Plan: docs/superpowers/plans/2026-04-21-action-output-substrate-foundation.md
// Spec: docs/superpowers/specs/2026-04-21-action-output-substrate-minimal-contract-design.md
//
// Run order:
//   1. Tests written first → all fail (RED)
//   2. Implementation lands → all pass (GREEN)

use std::sync::Arc;

use bionic_v0::action_output::{
    ActionOutputEvent, ActionOutputFeedback, ActionOutputPhase, ActionOutputState,
    OutputChannelKind,
};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

// ─── 1. Default state: no channels (持续归属合同 baseline) ────────────────────

#[test]
fn action_output_viewport_exposes_empty_write_state_by_default() {
    let vp = Viewport::new();
    let st = vp.read_action_output();

    assert_eq!(st.n_channels, 0);
    assert!(st.channels.is_empty());
}

// ─── 2. No world content in debug representation (不携带世界内容合同) ──────────

#[test]
fn action_output_state_tracks_write_phase_without_targets_or_words() {
    let st = ActionOutputState::default();
    assert_eq!(st.n_channels, 0);
    let dbg = format!("{:?}", st);
    assert!(!dbg.contains("target"));
    assert!(!dbg.contains("word"));
}

// ─── 3. Begin/Adjust/Decay cycle (可施加差异 / 连续可调 / 可撤销与可衰减合同) ──

#[test]
fn action_output_events_are_small_step_and_reversible() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.1,
        event_t_us: 100,
    });
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Adjust,
        magnitude: 0.25,
        event_t_us: 120,
    });
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Decay,
        magnitude: 0.05,
        event_t_us: 180,
    });

    let st = vp.read_action_output();
    assert_eq!(st.n_channels, 1);
    assert_eq!(st.channels[0].phase, ActionOutputPhase::Decaying);
    assert!(st.channels[0].current_magnitude <= 0.25);
    assert_eq!(st.channels[0].start_t_us, 100);
    assert_eq!(st.channels[0].last_update_t_us, 180);
}

// ─── 4. Snapshot roundtrip carries channel state (时间嵌入 / 自身可见合同) ─────

#[test]
fn snapshot_carries_action_output_write_state() {
    let vp = Arc::new(Viewport::new());
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1000,
    });

    let dir = std::env::temp_dir().join(format!("bionic_v0_action_output_snap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let snap = read_snapshot(&paths.final_path).expect("read ok");

    assert_eq!(snap.action_output.n_channels, 1);
    assert_eq!(snap.action_output.channels[0].start_t_us, 1000);
}

// ─── 6. Begin records write epoch in ledger (最小 write epoch history) ─────────
//
// After a Begin event, read_action_output() must expose a write_epoch_history
// with exactly one entry carrying the correct write_seq, channel_kind,
// start_t_us, last_update_t_us, and last_phase=Active.

#[test]
fn begin_records_epoch_in_write_epoch_history() {
    use bionic_v0::action_output::ActionOutputPhase;
    let vp = Viewport::new();
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 2000,
    });
    let st = vp.read_action_output();
    assert_eq!(st.n_write_epochs, 1, "one epoch in ledger after Begin");
    assert_eq!(st.write_epoch_history.len(), 1);
    let ep = &st.write_epoch_history[0];
    assert_eq!(
        ep.write_seq, st.channels[0].current_write_seq,
        "epoch seq == channel seq"
    );
    assert_eq!(ep.channel_kind, OutputChannelKind::GenericWrite);
    assert_eq!(ep.start_t_us, 2000);
    assert_eq!(ep.last_update_t_us, 2000);
    assert_eq!(ep.last_phase, ActionOutputPhase::Active);
}

// ─── 7. Stop updates last_phase and last_update_t_us in epoch history ─────────

#[test]
fn stop_updates_last_phase_in_write_epoch_history() {
    use bionic_v0::action_output::ActionOutputPhase;
    let vp = Viewport::new();
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 2000,
    });
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Stop,
        magnitude: 0.0,
        event_t_us: 3000,
    });
    let st = vp.read_action_output();
    assert_eq!(st.n_write_epochs, 1);
    let ep = &st.write_epoch_history[0];
    assert_eq!(
        ep.last_phase,
        ActionOutputPhase::Idle,
        "Stop must set last_phase to Idle in history"
    );
    assert_eq!(
        ep.last_update_t_us, 3000,
        "Stop must update last_update_t_us in history"
    );
}

// ─── 5. Observer renders "out:" with no human semantics (不经语言裁决合同) ─────

#[test]
fn observer_renders_action_output_without_human_semantics() {
    let observer_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/observer.rs"))
            .expect("read observer.rs");

    assert!(observer_src.contains("out:"));
    assert!(!observer_src.contains("speak"));
    assert!(!observer_src.contains("target_object"));
    assert!(!observer_src.contains("word"));
}

// ─── 6. Write epoch history survives snapshot roundtrip ───────────────────────

#[test]
fn snapshot_roundtrip_carries_write_epoch_history() {
    use bionic_v0::action_output::ActionOutputPhase;
    let vp = Arc::new(Viewport::new());

    // First epoch: Begin → Decay → its history entry should reflect Decaying.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1000,
    });
    let seq1 = vp.read_action_output().channels[0].current_write_seq;

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Decay,
        magnitude: 0.1,
        event_t_us: 3000,
    });

    // Second epoch: Begin → stays Active.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: 5000,
    });
    let seq2 = vp.read_action_output().channels[0].current_write_seq;

    let dir = std::env::temp_dir().join(format!("bionic_v0_epoch_history_snap_{}", std::process::id()));
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let snap = read_snapshot(&paths.final_path).expect("read ok");

    assert_eq!(
        snap.action_output.n_write_epochs, 2,
        "two epochs must survive snapshot"
    );
    assert_eq!(snap.action_output.write_epoch_history.len(), 2);

    let ep0 = &snap.action_output.write_epoch_history[0];
    assert_eq!(ep0.write_seq, seq1);
    assert_eq!(ep0.start_t_us, 1000);
    assert_eq!(
        ep0.last_update_t_us, 3000,
        "Decay must update last_update_t_us in history"
    );
    assert_eq!(
        ep0.last_phase as u64,
        ActionOutputPhase::Decaying as u64,
        "Decay must update last_phase in history"
    );

    let ep1 = &snap.action_output.write_epoch_history[1];
    assert_eq!(ep1.write_seq, seq2);
    assert_eq!(ep1.start_t_us, 5000);
    assert_eq!(ep1.last_phase as u64, ActionOutputPhase::Active as u64);
}

// ─── 7. Observer renders write epoch history ──────────────────────────────────

#[test]
fn observer_renders_write_epoch_history() {
    let observer_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/observer.rs"))
            .expect("read observer.rs");

    assert!(
        observer_src.contains("ep:"),
        "observer must render write epoch history as ep: column"
    );
    assert!(!observer_src.contains("success"));
    assert!(!observer_src.contains("target_object"));
}
