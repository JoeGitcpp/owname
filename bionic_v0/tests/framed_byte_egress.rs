use std::sync::Arc;

use bionic_v0::action_consequence::{ConsequenceModalityKind, ConsequencePairStatus};
use bionic_v0::action_output::{ActionOutputPhase, OutputChannelKind};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;

#[test]
fn framed_byte_begin_writes_frame_and_action_epoch() {
    let vp = Viewport::new();

    let frame = vp.begin_framed_byte_egress(1_000_000, 7, &[0, 255, 10], 0.4);

    let st = vp.read_framed_byte_egress();
    assert_eq!(st.n_frames, 1);
    assert_eq!(st.phase.as_u8(), 1);
    assert_eq!(st.active_stream_id, 7);
    assert_eq!(st.current_write_seq, frame.write_seq);
    assert_eq!(frame.frame_seq, 1);
    assert_eq!(frame.stream_id, 7);
    assert_eq!(frame.byte_len, 3);
    assert_eq!(frame.start_t_us, 1_000_000);
    assert_eq!(frame.end_t_us, 1_000_000);
    assert!(frame.byte_checksum > 0);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(
        out.write_epoch_history[0].channel_kind,
        OutputChannelKind::FramedByteEgress
    );
    assert_eq!(out.write_epoch_history[0].write_seq, frame.write_seq);
    assert_eq!(out.write_epoch_history[0].start_t_us, 1_000_000);
    assert_eq!(
        out.write_epoch_history[0].last_phase,
        ActionOutputPhase::Active
    );
}

#[test]
fn framed_byte_adjust_extends_same_epoch_and_stop_idles_it() {
    let vp = Viewport::new();
    let first = vp.begin_framed_byte_egress(1_000_000, 7, &[1, 2], 0.3);

    let second = vp.adjust_framed_byte_egress(1_010_000, &[3, 4, 5], 0.6);
    assert_eq!(second.write_seq, first.write_seq);
    assert_eq!(second.frame_seq, 2);
    assert_eq!(second.byte_len, 3);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.channels[0].current_write_seq, first.write_seq);
    assert_eq!(out.channels[0].reinforce_count, 1);
    assert_eq!(out.write_epoch_history[0].last_update_t_us, 1_010_000);

    vp.stop_framed_byte_egress(1_020_000);
    let stopped = vp.read_framed_byte_egress();
    let out = vp.read_action_output();
    assert_eq!(stopped.phase.as_u8(), 0);
    assert_eq!(out.channels[0].phase, ActionOutputPhase::Idle);
    assert_eq!(
        out.write_epoch_history[0].last_phase,
        ActionOutputPhase::Idle
    );
}

#[test]
fn framed_byte_return_forms_numeric_consequence_pair() {
    let vp = Viewport::new();
    let frame = vp.begin_framed_byte_egress(1_000_000, 7, &[1, 2, 3], 0.4);

    let window_id = vp.observe_framed_byte_return(1_020_000, 7, &[1, 2, 4], 0.3, 0.2);

    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_windows, 1);
    assert_eq!(ac.windows[0].window_id, window_id);
    assert_eq!(
        ac.windows[0].modality_kind,
        ConsequenceModalityKind::FramedByteReturn
    );
    assert_eq!(ac.n_pairs, 1);
    assert_eq!(ac.pairs[0].write_seq, frame.write_seq);
    assert_eq!(ac.pairs[0].window_id, window_id);
    assert_eq!(ac.pairs[0].delay_us, 20_000);
    assert_eq!(ac.pairs[0].status, ConsequencePairStatus::Paired);

    let st = vp.read_framed_byte_egress();
    assert_eq!(st.n_returns, 1);
    assert_eq!(st.last_return_window_id, window_id);
    assert_eq!(st.return_traces[0].byte_len, 3);
    assert_eq!(st.return_traces[0].correction_deviation, 0.2);
}

#[test]
fn framed_byte_correction_revises_projection_confidence_numerically() {
    let vp = Viewport::new();
    vp.begin_framed_byte_egress(1_000_000, 7, &[1, 2, 3], 0.4);
    let before = vp.read_framed_byte_egress().projection_confidence;

    vp.observe_framed_byte_return(1_020_000, 7, &[9, 9, 9], 0.3, 0.9);
    let after_high_deviation = vp.read_framed_byte_egress().projection_confidence;
    assert!(after_high_deviation < before);

    vp.observe_framed_byte_return(1_040_000, 7, &[1, 2, 3], 0.3, 0.0);
    let after_support = vp.read_framed_byte_egress().projection_confidence;
    assert!(after_support > after_high_deviation);

    let st = vp.read_framed_byte_egress();
    assert_eq!(st.n_returns, 2);
    assert_eq!(st.correction_count, 2);
    assert_eq!(st.correction_deviation, 0.0);
}

#[test]
fn snapshot_roundtrip_preserves_framed_byte_egress_trace() {
    let vp = Arc::new(Viewport::new());
    let frame = vp.begin_framed_byte_egress(1_000_000, 7, &[1, 2, 3], 0.4);
    vp.observe_framed_byte_return(1_020_000, 7, &[1, 2, 4], 0.3, 0.2);

    let dir = std::env::temp_dir().join("bionic_v0_framed_byte_egress_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let snap = read_snapshot(&paths.final_path).expect("read ok");

    assert_eq!(snap.framed_byte_egress.n_frames, 1);
    assert_eq!(snap.framed_byte_egress.n_returns, 1);
    assert_eq!(snap.framed_byte_egress.current_write_seq, frame.write_seq);
    assert_eq!(snap.framed_byte_egress.last_frame_seq, 1);
    assert_eq!(snap.framed_byte_egress.last_return_window_id, 1);
    assert!(snap.framed_byte_egress.projection_confidence > 0.5);
    assert_eq!(snap.framed_byte_egress.correction_deviation, 0.2);
}

#[test]
fn boundary_hygiene_rejects_controller_and_discrete_language_drift() {
    let module_src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/framed_byte_egress.rs"
    ))
    .expect("read module");
    let observer_src =
        std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/observer.rs"))
            .expect("read observer");

    let forbidden_patterns: &[&[u8]] = &[
        &[
            67, 111, 100, 101, 69, 103, 114, 101, 115, 115, 79, 114, 103, 97, 110,
        ],
        &[
            115, 101, 109, 97, 110, 116, 105, 99, 95, 114, 101, 97, 100, 105, 110, 101, 115, 115,
        ],
        &[99, 111, 110, 99, 101, 112, 116, 95, 105, 100],
        &[115, 117, 99, 99, 101, 115, 115],
        &[102, 97, 105, 108, 117, 114, 101],
        &[
            116, 97, 114, 103, 101, 116, 95, 114, 101, 115, 117, 108, 116,
        ],
        &[
            115, 111, 117, 114, 99, 101, 95, 105, 100, 101, 110, 116, 105, 116, 121,
        ],
    ];

    for forbidden in forbidden_patterns {
        let forbidden = std::str::from_utf8(forbidden).expect("ascii pattern");
        assert!(
            !module_src.contains(forbidden),
            "framed-byte egress module must not contain {forbidden}"
        );
        assert!(
            !observer_src.contains(forbidden),
            "observer trace must not contain {forbidden}"
        );
    }
}
