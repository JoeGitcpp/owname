use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::evolution_priors::BiasAxis;
use bionic_v0::framed_byte_carrier::{tick_sandbox_framed_byte_loop, SandboxFramedByteCarrier};
use bionic_v0::live_io_evidence::summarize_live_io_evidence;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotData, SnapshotPaths};
use bionic_v0::substrate_pressure::PressureSample;
use bionic_v0::viewport::Viewport;
use std::sync::Arc;

fn tense_sample() -> PressureSample {
    PressureSample {
        err: 0.7,
        ambiguity: 0.5,
        deviation: 0.8,
        recurrence: 0.4,
        cross_channel: 0.5,
        ..Default::default()
    }
}

fn make_paired_window(
    vp: &Viewport,
    write_t_us: u64,
    window_start_t_us: u64,
    magnitude: f64,
) -> u64 {
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude,
        event_t_us: write_t_us,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: window_start_t_us,
        end_t_us: window_start_t_us + 100,
        magnitude,
    });
    let window_id = vp.read_action_consequence().windows[idx as usize].window_id;
    assert!(vp.pair_action_consequence(
        write_seq,
        window_id,
        window_start_t_us.saturating_sub(write_t_us),
    ));
    window_id
}

fn snapshot_from_viewport(vp: &Arc<Viewport>, tag: &str) -> SnapshotData {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_life_closure_{}_{}",
        tag,
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(vp, &paths).unwrap();
    read_snapshot(&paths.final_path).unwrap()
}

#[test]
fn pair_sequence_action_consequence_pairs_have_monotonic_pair_sequence() {
    let vp = Viewport::new();
    let first = make_paired_window(&vp, 1_000, 1_400, 0.8);
    let second = make_paired_window(&vp, 2_000, 2_500, 0.7);

    let st = vp.read_action_consequence();
    let a = st.pairs.iter().find(|p| p.window_id == first).unwrap();
    let b = st.pairs.iter().find(|p| p.window_id == second).unwrap();

    assert!(a.pair_seq > 0);
    assert_eq!(b.pair_seq, a.pair_seq + 1);
}

#[test]
fn pair_sequence_evidence_tension_consumes_pair_closure_by_pair_sequence_once() {
    let vp = Viewport::new();
    vp.tick_evidence_tension(tense_sample(), 900);
    make_paired_window(&vp, 1_000, 1_400, 0.9);

    vp.tick_evidence_tension_from_pairs(1_500);
    let after_first = vp.read_evidence_tension();
    vp.tick_evidence_tension_from_pairs(1_600);
    let after_second = vp.read_evidence_tension();

    assert_eq!(after_second.last_pair_cursor, after_first.last_pair_cursor);
    assert_eq!(
        after_second.unresolved_pressure,
        after_first.unresolved_pressure
    );
}

#[test]
fn live_io_evidence_summary_reaches_pairing_level_from_snapshot_only() {
    let vp = Arc::new(Viewport::new());
    vp.begin_voicing(1_000, 0.45, 0.45, 0.55);
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1_100,
        end_t_us: 1_300,
        magnitude: 0.45,
    });
    let window_id = vp.read_action_consequence().windows[idx as usize].window_id;
    assert!(vp.pair_action_consequence(write_seq, window_id, 100));

    let snap = snapshot_from_viewport(&vp, "live_io_evidence");
    let summary = summarize_live_io_evidence(&snap);

    assert!(summary.body_write_out_exists);
    assert!(summary.write_epoch_exists);
    assert!(summary.consequence_return_exists);
    assert!(summary.temporal_pairing_exists);
    assert_eq!(summary.latest_window_id, window_id);
    assert_eq!(summary.latest_pair_seq, 1);
}

#[test]
fn sandbox_framed_byte_carrier_closes_write_return_loop_without_shell_meaning() {
    let vp = Viewport::new();
    let mut carrier = SandboxFramedByteCarrier::default();
    carrier.queue_return(7, b"ok", 0.72, 0.10);

    tick_sandbox_framed_byte_loop(&vp, &mut carrier, 1_000, 7, b"hi", 0.6);

    let egress = vp.read_framed_byte_egress();
    let ac = vp.read_action_consequence();
    assert_eq!(egress.n_frames, 1);
    assert_eq!(egress.n_returns, 1);
    assert!(egress.projection_confidence > 0.5);
    assert_eq!(ac.pairs.len(), 1);
    assert_eq!(ac.pairs[0].pair_seq, 1);
}

#[test]
fn life_closure_producers_pair_samples_sediment_route_and_bias_once() {
    let vp = Viewport::new();
    make_paired_window(&vp, 1_000, 1_300, 0.9);

    vp.tick_life_closure_producers(1_400);
    let adaptive = vp.read_adaptive_stability();
    let learned = vp.read_learned_bias();
    assert!(adaptive.n_routes > 0);
    assert!(learned
        .recent_events
        .iter()
        .any(|ev| ev.axis == BiasAxis::AlignmentGain));

    let event_count = learned.n_recent_events;
    vp.tick_life_closure_producers(1_500);
    assert_eq!(vp.read_learned_bias().n_recent_events, event_count);
}

#[test]
fn life_closure_producers_framed_byte_return_sediments_code_bias() {
    let vp = Viewport::new();
    vp.begin_framed_byte_egress(1_000, 3, b"ab", 0.5);
    vp.observe_framed_byte_return(1_200, 3, b"ab", 0.7, 0.0);

    vp.tick_life_closure_producers(1_300);

    let learned = vp.read_learned_bias();
    assert!(learned
        .recent_events
        .iter()
        .any(|ev| ev.axis == BiasAxis::CrossChannelGain));
}

#[test]
fn life_closure_framed_byte_bias_direction_tracks_return_relative_to_write() {
    let low_return = Viewport::new();
    low_return.begin_framed_byte_egress(1_000, 3, b"ab", 0.8);
    low_return.observe_framed_byte_return(1_200, 3, b"ab", 0.4, 0.4);
    low_return.tick_life_closure_producers(1_300);
    let low_delta = low_return
        .read_learned_bias()
        .recent_events
        .iter()
        .find(|ev| ev.axis == BiasAxis::CrossChannelGain)
        .map(|ev| ev.requested_delta)
        .unwrap_or(0.0);

    let high_return = Viewport::new();
    high_return.begin_framed_byte_egress(1_000, 3, b"ab", 0.4);
    high_return.observe_framed_byte_return(1_200, 3, b"ab", 0.8, 0.4);
    high_return.tick_life_closure_producers(1_300);
    let high_delta = high_return
        .read_learned_bias()
        .recent_events
        .iter()
        .find(|ev| ev.axis == BiasAxis::CrossChannelGain)
        .map(|ev| ev.requested_delta)
        .unwrap_or(0.0);

    assert!(
        low_delta < 0.0,
        "return below write pressure should reduce the channel pressure curve"
    );
    assert!(
        high_delta > 0.0,
        "return above write pressure should raise the channel pressure curve"
    );
}

#[test]
fn life_closure_producer_ticker_is_wired_in_body_runtime() {
    let body_src = include_str!("../src/bin/body.rs");
    assert!(
        body_src.contains("tick_life_closure_producers"),
        "body runtime must tick life-closure producers from admitted substrate traces"
    );
}
