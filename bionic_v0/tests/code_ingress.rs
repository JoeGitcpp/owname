use std::sync::Arc;

use bionic_v0::code_ingress::{
    CodeIngressFrameInput, CodeIngressState, CODE_DECODER_BINARY_FRAGMENT, CODE_DECODER_BYTE_UNIT,
};
use bionic_v0::code_ingress_adapter::{CodeIngressAdapter, ScriptedCodeIngressCarrier};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::{
    CODE_POS, CODE_SPAN, CODE_STRUCT_ORDER_ORIENT, CODE_STRUCT_ORDER_PERIOD,
    CODE_STRUCT_ORDER_POS,
};
use bionic_v0::viewport::Viewport;

#[test]
fn code_ingress_state_records_bounded_byte_disturbance() {
    let mut st = CodeIngressState::default();

    st.observe_frame(CodeIngressFrameInput {
        stream_context_id: 7,
        t_start_us: 1_000,
        t_end_us: 1_050,
        bytes: &[0x41, 0x42, 0x0a, 0xff],
        framing_validity: 1.0,
    });

    assert_eq!(st.stream_count, 1);
    assert_eq!(st.recent_frames.len(), 1);
    let frame = &st.recent_frames[0];
    assert_eq!(frame.frame_seq, 1);
    assert_eq!(frame.stream_context_id, 7);
    assert_eq!(frame.byte_len, 4);
    assert_eq!(frame.byte_sample_len, 4);
    assert!(frame.byte_hash != 0);
    assert!(frame.byte_transition_energy > 0.0);
    assert!(frame.evidence_ref != 0);

    assert!(st
        .projection_candidates
        .iter()
        .any(|p| p.decoder_kind_code == CODE_DECODER_BYTE_UNIT));
    assert!(st
        .projection_candidates
        .iter()
        .any(|p| p.decoder_kind_code == CODE_DECODER_BINARY_FRAGMENT));
    assert_eq!(st.sequence_windows.len(), 1);
}

#[test]
fn code_ingress_records_raw_and_binary_projection_only_until_separate_shape_evidence_exists() {
    let mut st = CodeIngressState::default();

    st.observe_frame(CodeIngressFrameInput {
        stream_context_id: 3,
        t_start_us: 1_000,
        t_end_us: 1_100,
        bytes: b"abc",
        framing_validity: 0.9,
    });

    let kinds: Vec<u8> = st
        .projection_candidates
        .iter()
        .map(|projection| projection.decoder_kind_code)
        .collect();

    assert_eq!(
        kinds,
        vec![CODE_DECODER_BYTE_UNIT, CODE_DECODER_BINARY_FRAGMENT],
        "text-shaped projection must not be created body-side without separate visual evidence"
    );
}

#[test]
fn code_ingress_adapter_drains_dedicated_framed_carrier_into_viewport() {
    let vp = Arc::new(Viewport::new());
    let carrier = ScriptedCodeIngressCarrier::new(9, vec![vec![0x7b, 0x7d]]);
    let mut adapter = CodeIngressAdapter::new(carrier);

    adapter.tick(&vp, 2_000);

    let st = vp.read_code_ingress();
    assert_eq!(st.recent_frames.len(), 1);
    assert_eq!(st.recent_frames[0].stream_context_id, 9);
    assert_eq!(st.recent_frames[0].byte_len, 2);
    assert_eq!(vp.read_language_binding().n_bindings, 0);
}

#[test]
fn snapshot_roundtrip_preserves_code_ingress_traces() {
    let dir = std::env::temp_dir().join("bionic_v0_code_ingress_snap");
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    let vp = Arc::new(Viewport::new());

    let carrier = ScriptedCodeIngressCarrier::new(3, vec![vec![0x61, 0x62, 0x63]]);
    let mut adapter = CodeIngressAdapter::new(carrier);
    adapter.tick(&vp, 3_000);

    write_snapshot(&vp, &paths).expect("write snapshot");
    let snap = read_snapshot(&paths.final_path).expect("read snapshot");

    assert_eq!(snap.code_ingress.recent_frames.len(), 1);
    assert_eq!(snap.code_ingress.recent_frames[0].stream_context_id, 3);
    assert_eq!(snap.code_ingress.recent_frames[0].byte_len, 3);
    assert!(snap.code_ingress.projection_candidates.len() >= 2);
    assert_eq!(snap.language_binding.n_bindings, 0);
}

#[test]
fn observer_source_renders_code_ingress_without_command_semantics() {
    let observer = include_str!("../src/bin/observer.rs");
    assert!(observer.contains("code:n"));
    for forbidden in [
        "command meaning",
        "shell state",
        "terminal prompt",
        "source identity",
        "owner",
    ] {
        assert!(
            !observer.contains(forbidden),
            "observer code should not render `{}` as body truth",
            forbidden
        );
    }
}

#[test]
fn code_ingress_sources_contain_no_rejected_architecture() {
    let files = [
        include_str!("../src/code_ingress.rs"),
        include_str!("../src/code_ingress_adapter.rs"),
    ];
    for src in files {
        for forbidden in [
            "semantic_readiness",
            "SymbolizerState",
            "concept_id",
            "token_id",
            "word_id",
            "object_name",
            "println!",
            "eprintln!",
        ] {
            assert!(
                !src.contains(forbidden),
                "code ingress source must not contain `{}`",
                forbidden
            );
        }
    }
}

// Guard: the predictable representation of digital objects lives on the
// CODE axes, not in the extraction buffer.  After observe_code_ingress_frame
// all five CODE axes must carry a fresh observation; CodeIngressState itself
// is a derived extraction buffer and must not be the downstream ground truth.
#[test]
fn digital_object_predictable_repr_lands_on_code_axes() {
    let vp = Arc::new(Viewport::new());

    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 5,
        t_start_us: 10_000,
        t_end_us: 10_500,
        bytes: &[0x10, 0x20, 0x30, 0x40, 0x50],
        framing_validity: 0.8,
    });

    // All five CODE axes must have received an observation.
    for &axis in &[
        CODE_POS,
        CODE_SPAN,
        CODE_STRUCT_ORDER_POS,
        CODE_STRUCT_ORDER_ORIENT,
        CODE_STRUCT_ORDER_PERIOD,
    ] {
        let obs = vp.latest_observation(axis);
        assert!(
            obs.is_some(),
            "CODE axis {} must carry an observation after observe_code_ingress_frame",
            axis
        );
        let s = obs.unwrap();
        assert!(
            s.value >= 0.0 && s.value <= 1.0,
            "CODE axis {} value {:.4} must be in [0,1]",
            axis,
            s.value
        );
        assert_eq!(
            s.t_us, 10_500,
            "CODE axis {} timestamp must match the frame t_end_us",
            axis
        );
    }

    // The extraction buffer still holds the raw trace (for egress pairing and
    // snapshot/observer display), but the axis observations are authoritative.
    let buf = vp.read_code_ingress();
    assert_eq!(buf.recent_frames.len(), 1, "extraction buffer retains frame trace");
}
