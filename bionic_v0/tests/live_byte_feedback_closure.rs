#[test]
fn closure_driver_writes_body_frame_from_fresh_code_pressure() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::live_byte_closure::LiveByteClosureDriver;
    use bionic_v0::viewport::Viewport;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_closure_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();
    let out_path = dir.join("live_bytes").join("out.frames");

    let vp = Viewport::new();
    let mut driver = LiveByteClosureDriver::new(out_path.clone());
    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 2,
        t_start_us: 1_000,
        t_end_us: 1_000,
        bytes: b"abcabc001",
        framing_validity: 1.0,
    });
    vp.tick_evidence_tension_from_code_ingress(1_100);

    assert!(driver.tick(&vp, 1_200));
    assert_eq!(vp.read_framed_byte_egress().n_frames, 1);
    assert!(std::fs::metadata(out_path).unwrap().len() > 0);
}

#[test]
fn closure_driver_pairs_later_code_frame_as_return_and_sediments_bias() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::evolution_priors::BiasAxis;
    use bionic_v0::live_byte_closure::LiveByteClosureDriver;
    use bionic_v0::viewport::Viewport;

    let dir =
        std::env::temp_dir().join(format!("bionic_v0_live_byte_return_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();

    let vp = Viewport::new();
    let mut driver = LiveByteClosureDriver::new(dir.join("live_bytes").join("out.frames"));
    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 2,
        t_start_us: 1_000,
        t_end_us: 1_000,
        bytes: b"abcabc001",
        framing_validity: 1.0,
    });
    vp.tick_evidence_tension_from_code_ingress(1_100);
    assert!(driver.tick(&vp, 1_200));

    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 2,
        t_start_us: 1_500,
        t_end_us: 1_500,
        bytes: b"001abcabc",
        framing_validity: 1.0,
    });
    assert!(driver.tick(&vp, 1_600));
    vp.tick_life_closure_producers(1_700);

    assert_eq!(vp.read_framed_byte_egress().n_returns, 1);
    assert!(vp.read_action_consequence().n_pairs > 0);
    assert!(vp
        .read_learned_bias()
        .recent_events
        .iter()
        .any(|event| event.axis == BiasAxis::CrossChannelGain));
}

#[test]
fn closure_pair_similarity_tracks_framed_byte_return_deviation() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::live_byte_closure::LiveByteClosureDriver;
    use bionic_v0::viewport::Viewport;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_quality_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();

    let vp = Viewport::new();
    let mut driver = LiveByteClosureDriver::new(dir.join("live_bytes").join("out.frames"));
    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 2,
        t_start_us: 1_000,
        t_end_us: 1_000,
        bytes: b"abcabc001",
        framing_validity: 1.0,
    });
    vp.tick_evidence_tension_from_code_ingress(1_100);
    assert!(driver.tick(&vp, 1_200));

    vp.observe_code_ingress_frame(CodeIngressFrameInput {
        stream_context_id: 2,
        t_start_us: 1_500,
        t_end_us: 1_500,
        bytes: b"001abcabc",
        framing_validity: 1.0,
    });
    assert!(driver.tick(&vp, 1_600));

    let deviation = vp.read_framed_byte_egress().return_traces[0].correction_deviation;
    let similarity = vp.read_action_consequence().pairs[0].curve_similarity;

    assert!(
        (similarity - (1.0 - deviation)).abs() < 1e-9,
        "framed-byte pair similarity must use the same numeric return curve as correction deviation"
    );
}

#[test]
fn closure_driver_applies_learned_channel_gain_to_output_pressure() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::evolution_priors::BiasAxis;
    use bionic_v0::learned_bias::{BiasLocator, LearnedBiasEventInput};
    use bionic_v0::live_byte_closure::{LiveByteClosureDriver, LIVE_BYTE_EGRESS_STREAM_ID};
    use bionic_v0::viewport::Viewport;

    fn prepare(vp: &Viewport) {
        vp.observe_code_ingress_frame(CodeIngressFrameInput {
            stream_context_id: 2,
            t_start_us: 1_000,
            t_end_us: 1_000,
            bytes: b"abcabc001",
            framing_validity: 1.0,
        });
        vp.tick_evidence_tension_from_code_ingress(1_100);
    }

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_learned_gain_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let baseline = Viewport::new();
    prepare(&baseline);
    let mut baseline_driver = LiveByteClosureDriver::new(dir.join("baseline.frames"));
    assert!(baseline_driver.tick(&baseline, 1_200));
    let baseline_pressure = baseline.read_framed_byte_egress().output_pressure;
    let baseline_bytes = std::fs::read(dir.join("baseline.frames")).unwrap();

    let biased = Viewport::new();
    prepare(&biased);
    for i in 0..8_u64 {
        biased.apply_learned_bias_event(LearnedBiasEventInput {
            axis: BiasAxis::CrossChannelGain,
            locator: BiasLocator::Channel(LIVE_BYTE_EGRESS_STREAM_ID as u16),
            requested_delta: 0.05,
            support: 1.0,
            deviation: 0.0,
            evidence_ref: i + 1,
            event_t_us: 2_000 + i,
        });
    }
    let mut biased_driver = LiveByteClosureDriver::new(dir.join("biased.frames"));
    assert!(biased_driver.tick(&biased, 1_200));
    let biased_pressure = biased.read_framed_byte_egress().output_pressure;
    let biased_bytes = std::fs::read(dir.join("biased.frames")).unwrap();

    assert!(
        biased_pressure > baseline_pressure,
        "learned channel gain must feed back into live byte output pressure"
    );
    assert_ne!(
        biased_bytes, baseline_bytes,
        "pressure feedback must reach the actual framed-byte write-out"
    );
}

#[test]
fn closure_driver_keeps_sustained_channel_bias_below_pressure_saturation() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::evolution_priors::BiasAxis;
    use bionic_v0::learned_bias::{BiasLocator, LearnedBiasEventInput};
    use bionic_v0::live_byte_closure::{LiveByteClosureDriver, LIVE_BYTE_EGRESS_STREAM_ID};
    use bionic_v0::viewport::Viewport;

    fn prepare(vp: &Viewport) {
        vp.observe_code_ingress_frame(CodeIngressFrameInput {
            stream_context_id: 2,
            t_start_us: 1_000,
            t_end_us: 1_000,
            bytes: b"abcabc001",
            framing_validity: 1.0,
        });
        vp.tick_evidence_tension_from_code_ingress(1_100);
    }

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_unsaturated_gain_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let baseline = Viewport::new();
    prepare(&baseline);
    let mut baseline_driver = LiveByteClosureDriver::new(dir.join("baseline.frames"));
    assert!(baseline_driver.tick(&baseline, 1_200));
    let baseline_pressure = baseline.read_framed_byte_egress().output_pressure;

    let sustained = Viewport::new();
    prepare(&sustained);
    for i in 0..512_u64 {
        sustained.apply_learned_bias_event(LearnedBiasEventInput {
            axis: BiasAxis::CrossChannelGain,
            locator: BiasLocator::Channel(LIVE_BYTE_EGRESS_STREAM_ID as u16),
            requested_delta: 0.05,
            support: 1.0,
            deviation: 0.0,
            evidence_ref: i + 1,
            event_t_us: 2_000 + i,
        });
    }
    let mut sustained_driver = LiveByteClosureDriver::new(dir.join("sustained.frames"));
    assert!(sustained_driver.tick(&sustained, 1_200));
    let sustained_pressure = sustained.read_framed_byte_egress().output_pressure;

    assert!(
        sustained_pressure > baseline_pressure,
        "sustained support should still reach the write-out pressure"
    );
    assert!(
        sustained_pressure < baseline_pressure * 1.25,
        "sustained support must remain below actuator saturation so later closure evidence can still matter"
    );
}

#[test]
fn closure_driver_retains_bounded_historical_channel_gain() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::evolution_priors::BiasAxis;
    use bionic_v0::learned_bias::{BiasLocator, LearnedBiasEventInput};
    use bionic_v0::live_byte_closure::{LiveByteClosureDriver, LIVE_BYTE_EGRESS_STREAM_ID};
    use bionic_v0::viewport::Viewport;

    fn prepare(vp: &Viewport) {
        vp.observe_code_ingress_frame(CodeIngressFrameInput {
            stream_context_id: 2,
            t_start_us: 1_000,
            t_end_us: 1_000,
            bytes: b"abcabc001",
            framing_validity: 1.0,
        });
        vp.tick_evidence_tension_from_code_ingress(1_100);
    }

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_historical_gain_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let baseline = Viewport::new();
    prepare(&baseline);
    let mut baseline_driver = LiveByteClosureDriver::new(dir.join("baseline.frames"));
    assert!(baseline_driver.tick(&baseline, 1_200));
    let baseline_pressure = baseline.read_framed_byte_egress().output_pressure;

    let sedimented = Viewport::new();
    prepare(&sedimented);
    for i in 0..512_u64 {
        sedimented.apply_learned_bias_event(LearnedBiasEventInput {
            axis: BiasAxis::CrossChannelGain,
            locator: BiasLocator::Channel(LIVE_BYTE_EGRESS_STREAM_ID as u16),
            requested_delta: 0.05,
            support: 1.0,
            deviation: 0.0,
            evidence_ref: i + 1,
            event_t_us: 2_000 + i,
        });
    }
    let mut sedimented_driver = LiveByteClosureDriver::new(dir.join("sedimented.frames"));
    assert!(sedimented_driver.tick(&sedimented, 3_000));
    let sedimented_pressure = sedimented.read_framed_byte_egress().output_pressure;

    assert!(
        sedimented_pressure >= baseline_pressure * 1.08,
        "sustained numeric support must survive beyond short EMA-only gain"
    );
    assert!(
        sedimented_pressure < baseline_pressure * 1.25,
        "historical channel gain must remain below pressure saturation"
    );
}

#[test]
fn closure_driver_dampens_stale_channel_gain_on_reentry() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::evolution_priors::BiasAxis;
    use bionic_v0::learned_bias::{BiasLocator, LearnedBiasEventInput};
    use bionic_v0::live_byte_closure::{LiveByteClosureDriver, LIVE_BYTE_EGRESS_STREAM_ID};
    use bionic_v0::viewport::Viewport;

    fn prepare(vp: &Viewport, t_start_us: u64) {
        vp.observe_code_ingress_frame(CodeIngressFrameInput {
            stream_context_id: 2,
            t_start_us,
            t_end_us: t_start_us,
            bytes: b"abcabc001",
            framing_validity: 1.0,
        });
        vp.tick_evidence_tension_from_code_ingress(t_start_us + 100);
    }

    fn apply_gain(vp: &Viewport, first_event_t_us: u64) {
        for i in 0..8_u64 {
            vp.apply_learned_bias_event(LearnedBiasEventInput {
                axis: BiasAxis::CrossChannelGain,
                locator: BiasLocator::Channel(LIVE_BYTE_EGRESS_STREAM_ID as u16),
                requested_delta: 0.05,
                support: 1.0,
                deviation: 0.0,
                evidence_ref: i + 1,
                event_t_us: first_event_t_us + i,
            });
        }
    }

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_reentry_gain_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let reentry_t_us = 900_000_000;

    let baseline = Viewport::new();
    prepare(&baseline, reentry_t_us - 1_000);
    let mut baseline_driver = LiveByteClosureDriver::new(dir.join("baseline.frames"));
    assert!(baseline_driver.tick(&baseline, reentry_t_us));
    let baseline_pressure = baseline.read_framed_byte_egress().output_pressure;

    let recent = Viewport::new();
    prepare(&recent, reentry_t_us - 1_000);
    apply_gain(&recent, reentry_t_us - 2_000);
    let mut recent_driver = LiveByteClosureDriver::new(dir.join("recent.frames"));
    assert!(recent_driver.tick(&recent, reentry_t_us));
    let recent_pressure = recent.read_framed_byte_egress().output_pressure;

    let stale = Viewport::new();
    prepare(&stale, reentry_t_us - 1_000);
    apply_gain(&stale, 1_000);
    let mut stale_driver = LiveByteClosureDriver::new(dir.join("stale.frames"));
    assert!(stale_driver.tick(&stale, reentry_t_us));
    let stale_pressure = stale.read_framed_byte_egress().output_pressure;

    assert!(
        recent_pressure > baseline_pressure,
        "fresh learned gain must still reach output pressure"
    );
    assert!(
        stale_pressure > baseline_pressure,
        "stale learned gain should remain present, not be erased"
    );
    assert!(
        stale_pressure < baseline_pressure + (recent_pressure - baseline_pressure) * 0.5,
        "long-idle reentry must not apply stale channel gain at full strength"
    );
}

#[test]
fn loopback_bridge_appends_bounded_return_bytes() {
    use bionic_v0::live_byte_loopback::LoopbackFileBridge;
    use std::io::Write;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_byte_loopback_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();
    let out_path = dir.join("live_bytes").join("out.frames");
    let in_path = dir.join("live_bytes").join("in.frames");

    let mut out = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&out_path)
        .unwrap();
    out.write_all(&[1, 2, 3, 4]).unwrap();
    drop(out);

    let mut bridge = LoopbackFileBridge::new(out_path, in_path.clone(), 8);
    assert!(bridge.tick());
    let returned = std::fs::read(in_path).unwrap();

    assert_eq!(returned.len(), 4);
    assert_ne!(returned, vec![1, 2, 3, 4]);
}

#[test]
fn body_runtime_wires_live_byte_closure_without_stdio_surface() {
    let body = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/body.rs"))
        .expect("read body.rs");

    assert!(body.contains("LiveByteClosureDriver"));
    assert!(body.contains("out.frames"));

    for forbidden in ["stdin", "std::process::Command", "println!", "eprintln!"] {
        assert!(
            !body.contains(forbidden),
            "body must not expose live byte closure through {forbidden}"
        );
    }
}

#[test]
fn snapshot_and_session_expose_cumulative_closure_counts_beyond_trace_caps() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::live_byte_closure::LiveByteClosureDriver;
    use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
    use bionic_v0::viewport::Viewport;
    use std::sync::Arc;

    let dir =
        std::env::temp_dir().join(format!("bionic_v0_live_byte_total_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();

    let vp = Arc::new(Viewport::new());
    let mut driver = LiveByteClosureDriver::new(dir.join("live_bytes").join("out.frames"));

    for i in 0..24_u64 {
        let input_t = 1_000 + i * 1_000;
        vp.observe_code_ingress_frame(CodeIngressFrameInput {
            stream_context_id: 2,
            t_start_us: input_t,
            t_end_us: input_t,
            bytes: &[i as u8, 0x61, 0x62, 0x63],
            framing_validity: 1.0,
        });
        vp.tick_evidence_tension_from_code_ingress(input_t + 10);
        assert!(driver.tick(&vp, input_t + 20));

        vp.observe_code_ingress_frame(CodeIngressFrameInput {
            stream_context_id: 2,
            t_start_us: input_t + 100,
            t_end_us: input_t + 100,
            bytes: &[0x63, 0x62, 0x61, i as u8],
            framing_validity: 1.0,
        });
        assert!(driver.tick(&vp, input_t + 120));
        vp.tick_life_closure_producers(input_t + 140);
    }

    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    assert_eq!(snap.framed_byte_egress.n_frames, 16);
    assert_eq!(snap.framed_byte_egress.n_returns, 16);
    assert!(snap.framed_byte_egress.total_frames >= 24);
    assert!(snap.framed_byte_egress.total_returns >= 24);
    assert!(snap.action_consequence.total_pairs >= 24);
    assert!(snap.learned_bias.total_events >= 24);

    let session_src = include_str!("../src/bin/observer_helpers/session.rs");
    assert!(session_src.contains("egress_total_frames"));
    assert!(session_src.contains("egress_total_returns"));
    assert!(session_src.contains("pair_total"));
    assert!(session_src.contains("learned_event_total"));
}

#[test]
fn snapshot_and_session_split_pair_totals_by_return_modality() {
    use bionic_v0::action_consequence::{ActionConsequenceWindowEvent, ConsequenceModalityKind};
    use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
    use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
    use bionic_v0::viewport::Viewport;
    use std::sync::Arc;

    let dir = std::env::temp_dir().join(format!("bionic_v0_pair_sources_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let vp = Arc::new(Viewport::new());
    vp.begin_framed_byte_egress(1_000, 3, b"ab", 0.5);
    vp.observe_framed_byte_return(1_100, 3, b"ba", 0.5, 0.1);

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.4,
        event_t_us: 2_000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 2_100,
        end_t_us: 2_200,
        magnitude: 0.4,
    });
    let window_id = vp.read_action_consequence().windows[idx as usize].window_id;
    assert!(vp.pair_action_consequence(write_seq, window_id, 100));

    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    assert_eq!(snap.action_consequence.total_pairs, 2);
    assert_eq!(snap.action_consequence.total_framed_byte_pairs, 1);
    assert_eq!(snap.action_consequence.total_other_pairs, 1);

    let session_src = include_str!("../src/bin/observer_helpers/session.rs");
    assert!(session_src.contains("pair_framed_byte_total"));
    assert!(session_src.contains("pair_other_total"));
}

#[test]
fn egress_bytes_are_constrained_to_valid_vm_opcode_range() {
    use bionic_v0::code_ingress::CodeIngressFrameInput;
    use bionic_v0::live_byte_closure::LiveByteClosureDriver;
    use bionic_v0::viewport::Viewport;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_egress_opcode_range_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("live_bytes")).unwrap();
    let out_path = dir.join("live_bytes").join("out.frames");

    let vp = Viewport::new();
    let mut driver = LiveByteClosureDriver::new(out_path.clone());

    // Feed ingress frames so live byte closure has material to derive egress from.
    for i in 0..16_u64 {
        let t = 1_000 + i * 100;
        let bytes: Vec<u8> = (0..8).map(|b| (b as u8).wrapping_add(i as u8)).collect();
        vp.observe_code_ingress_frame(CodeIngressFrameInput {
            stream_context_id: 2,
            t_start_us: t,
            t_end_us: t,
            bytes: &bytes,
            framing_validity: 1.0,
        });
        vp.tick_evidence_tension_from_code_ingress(t + 10);
        driver.tick(&vp, t + 20);
    }

    let written = std::fs::read(&out_path).expect("out.frames should exist");
    assert!(!written.is_empty(), "egress must write at least one frame");

    for (idx, &byte) in written.iter().enumerate() {
        assert!(
            byte <= 0x0F,
            "egress byte {idx} value 0x{byte:02x} exceeds valid opcode range 0x00-0x0F"
        );
    }
}

