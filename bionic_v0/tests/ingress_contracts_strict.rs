use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use bionic_v0::adapter::{
    AdapterRegistry, ModalityAdapter, RealPhotAdapter, RegistryError, SyntheticVibAdapter,
};
use bionic_v0::code_ingress::{
    CodeIngressFrameInput, CodeIngressState, CODE_DECODER_BINARY_FRAGMENT, CODE_DECODER_BYTE_UNIT,
};
use bionic_v0::code_ingress_adapter::FramedByteCarrier;
use bionic_v0::live_byte_carrier::LiveByteFileCarrier;
use bionic_v0::phot_camera::{
    CameraFault, CameraFrameProvider, SynthCamStep, SyntheticCameraFrameProvider, CAM_PIX,
};
use bionic_v0::synthetic_vib::SyntheticVibSource;
use bionic_v0::viewport::{HealthState, Viewport};
use bionic_v0::{
    CODE_INGRESS_BYTE_SAMPLE_CAP, CODE_INGRESS_FRAME_HISTORY_CAP,
    CODE_INGRESS_PROJECTION_HISTORY_CAP, CODE_INGRESS_SEQUENCE_WINDOW_CAP, MOD_VIB, N_OBJECT_SLOTS,
};

struct CountingAdapter {
    bit: u8,
    hz: u32,
    ticks: Arc<AtomicUsize>,
}

impl CountingAdapter {
    fn new(bit: u8, hz: u32, ticks: Arc<AtomicUsize>) -> Self {
        Self { bit, hz, ticks }
    }
}

impl ModalityAdapter for CountingAdapter {
    fn modality_bit(&self) -> u8 {
        self.bit
    }

    fn writes_ambient(&self) -> bool {
        true
    }

    fn writes_slots(&self) -> bool {
        false
    }

    fn tick_hz(&self) -> u32 {
        self.hz
    }

    fn tick(&mut self, _vp: &Arc<Viewport>, _now_us: u64) {
        self.ticks.fetch_add(1, Ordering::Relaxed);
    }
}

#[test]
fn adapter_registry_rejects_duplicate_modalities_and_rate_limits_ticks() {
    let vp = Arc::new(Viewport::new());
    let mut registry = AdapterRegistry::new();
    let ticks = Arc::new(AtomicUsize::new(0));

    registry
        .register(Box::new(CountingAdapter::new(
            0b1000_0000,
            100,
            ticks.clone(),
        )))
        .expect("first adapter registers");
    assert!(matches!(
        registry.register(Box::new(CountingAdapter::new(
            0b1000_0000,
            200,
            Arc::new(AtomicUsize::new(0)),
        ))),
        Err(RegistryError::DuplicateModality(0b1000_0000))
    ));

    assert_eq!(registry.len(), 1);
    assert_eq!(registry.modality_mask(), 0b1000_0000);

    registry.tick_due(&vp, 9_999);
    assert_eq!(ticks.load(Ordering::Relaxed), 0);

    registry.tick_due(&vp, 10_000);
    assert_eq!(ticks.load(Ordering::Relaxed), 1);

    registry.tick_due(&vp, 19_999);
    assert_eq!(ticks.load(Ordering::Relaxed), 1);

    registry.tick_due(&vp, 20_000);
    assert_eq!(ticks.load(Ordering::Relaxed), 2);
}

#[test]
fn synthetic_vib_adapter_writes_only_vib_ambient_and_vib_slot_on_event() {
    let vp = Arc::new(Viewport::new());
    let source = SyntheticVibSource::new(0, 1_000_000, 500_000);
    let mut adapter = SyntheticVibAdapter::new(source);

    adapter.tick(&vp, 250_000);

    let ambient = vp.read_ambient();
    assert!(
        ambient.vib.event_energy > 0.0,
        "VIB event must write VIB ambient energy"
    );
    assert!(
        ambient.vib.candidate_flag,
        "VIB burst must nominate a VIB candidate"
    );
    assert_eq!(
        ambient.phot.event_energy, 0.0,
        "VIB adapter must not write PHOT ambient state"
    );

    let mut vib_slots = 0usize;
    for slot_id in 0..N_OBJECT_SLOTS {
        let slot = vp.read_slot(slot_id).unwrap();
        if slot.active && (slot.source_modality & MOD_VIB) != 0 {
            vib_slots += 1;
            assert!(slot.vib.is_some(), "VIB slot must carry VIB bands");
            assert!(
                slot.phot.is_none(),
                "VIB-only event must not populate PHOT family state"
            );
        }
    }
    assert!(vib_slots >= 1, "VIB burst must claim at least one VIB slot");
}

#[test]
fn synthetic_vib_adapter_silent_source_stays_quiet_and_slotless() {
    let vp = Arc::new(Viewport::new());
    let source = SyntheticVibSource::new(0, 1_000_000, 500_000).silent();
    let mut adapter = SyntheticVibAdapter::new(source);

    for step in 0..8u64 {
        adapter.tick(&vp, 100_000 + step * 25_000);
    }

    let ambient = vp.read_ambient();
    assert!(
        !ambient.vib.candidate_flag,
        "silent VIB source must not nominate an event"
    );
    for slot_id in 0..N_OBJECT_SLOTS {
        let slot = vp.read_slot(slot_id).unwrap();
        assert!(
            !(slot.active && (slot.source_modality & MOD_VIB) != 0),
            "silent VIB source must not claim a VIB slot"
        );
    }
}

#[test]
fn synthetic_camera_provider_clamps_frames_and_faults_stickily() {
    let mut provider = SyntheticCameraFrameProvider::new(
        vec![
            SynthCamStep::NoFrame,
            SynthCamStep::Blob {
                cx: -1.0,
                cy: 2.0,
                r: 0.0,
                brightness: 4.0,
            },
            SynthCamStep::RuntimeError,
        ],
        100,
        10,
    );

    assert!(provider.is_healthy());
    assert!(provider.try_recv().unwrap().is_none());

    let frame = provider.try_recv().unwrap().expect("clamped blob frame");
    assert_eq!(frame.t_us, 120);
    assert_eq!(frame.luma.len(), CAM_PIX);
    assert!(
        frame
            .luma
            .iter()
            .all(|pixel| pixel.is_finite() && *pixel >= 0.0 && *pixel <= 1.0),
        "camera frame luma must stay normalized"
    );
    assert!(
        frame.luma.iter().any(|pixel| *pixel > 0.9),
        "clamped bright blob must still be visible"
    );

    assert!(matches!(
        provider.try_recv(),
        Err(CameraFault::RuntimeError)
    ));
    assert!(!provider.is_healthy());
    assert!(
        matches!(provider.try_recv(), Err(CameraFault::RuntimeError)),
        "provider fault must remain observable after the first runtime error"
    );
}

#[test]
fn real_phot_adapter_turns_provider_runtime_fault_into_sticky_health_fault() {
    let vp = Arc::new(Viewport::new());
    let provider = Box::new(SyntheticCameraFrameProvider::new(
        vec![SynthCamStep::Idle, SynthCamStep::RuntimeError],
        0,
        33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);

    adapter.tick(&vp, 1_000_000);
    assert_ne!(vp.read_phot_health(), HealthState::Fault);

    adapter.tick(&vp, 1_033_333);
    assert_eq!(vp.read_phot_health(), HealthState::Fault);

    adapter.tick(&vp, 1_066_666);
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Fault,
        "PHOT runtime fault must remain sticky"
    );
}

#[test]
fn live_byte_file_carrier_normalizes_zero_frame_len_and_preserves_binary_bytes() {
    let dir =
        std::env::temp_dir().join(format!("bionic_v0_strict_live_byte_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("in.frames");
    std::fs::write(&path, [0x00, 0xff, b'\n']).unwrap();

    let mut carrier = LiveByteFileCarrier::new(path, 77, 0);

    let first = carrier.try_recv_frame().expect("first one-byte frame");
    let second = carrier.try_recv_frame().expect("second one-byte frame");
    let third = carrier.try_recv_frame().expect("third one-byte frame");
    assert!(carrier.try_recv_frame().is_none());

    assert_eq!(first.stream_context_id, 77);
    assert_eq!(first.bytes, vec![0x00]);
    assert_eq!(second.bytes, vec![0xff]);
    assert_eq!(third.bytes, vec![b'\n']);
    assert_eq!(first.framing_validity, 1.0);
}

#[test]
fn code_ingress_clamps_invalid_validity_and_keeps_binary_projection_valid() {
    let mut state = CodeIngressState::default();

    state.observe_frame(CodeIngressFrameInput {
        stream_context_id: 5,
        t_start_us: 1_000,
        t_end_us: 1_050,
        bytes: b"abc",
        framing_validity: f64::NAN,
    });

    let frame = state.recent_frames.last().expect("frame trace");
    assert_eq!(frame.framing_validity, 0.0);

    let byte_unit = state
        .projection_candidates
        .iter()
        .find(|candidate| candidate.decoder_kind_code == CODE_DECODER_BYTE_UNIT)
        .expect("byte-unit candidate");
    assert_eq!(byte_unit.validity, 0.0);
    assert_eq!(byte_unit.error_magnitude, 1.0);

    let binary = state
        .projection_candidates
        .iter()
        .find(|candidate| candidate.decoder_kind_code == CODE_DECODER_BINARY_FRAGMENT)
        .expect("binary-fragment candidate");
    assert_eq!(binary.validity, 1.0);
    assert_eq!(binary.error_magnitude, 0.0);
}

#[test]
fn code_ingress_retention_caps_frames_projections_windows_and_samples() {
    let mut state = CodeIngressState::default();
    let total = CODE_INGRESS_FRAME_HISTORY_CAP + 9;
    let bytes: Vec<u8> = (0..(CODE_INGRESS_BYTE_SAMPLE_CAP + 17))
        .map(|idx| idx as u8)
        .collect();

    for seq in 0..total {
        state.observe_frame(CodeIngressFrameInput {
            stream_context_id: (seq % 5) as u64,
            t_start_us: 10_000 + seq as u64,
            t_end_us: 20_000 + seq as u64,
            bytes: &bytes,
            framing_validity: if seq % 2 == 0 { 1.5 } else { -0.5 },
        });
    }

    assert_eq!(state.recent_frames.len(), CODE_INGRESS_FRAME_HISTORY_CAP);
    assert_eq!(
        state.projection_candidates.len(),
        CODE_INGRESS_PROJECTION_HISTORY_CAP
    );
    assert_eq!(
        state.sequence_windows.len(),
        CODE_INGRESS_SEQUENCE_WINDOW_CAP
    );
    assert_eq!(state.stream_count, 5);
    assert_eq!(state.last_update_t_us, 20_000 + (total - 1) as u64);

    let first_retained = state.recent_frames.first().expect("first retained frame");
    let last_retained = state.recent_frames.last().expect("last retained frame");
    assert_eq!(first_retained.frame_seq, 10);
    assert_eq!(last_retained.frame_seq, total as u64);
    assert_eq!(
        last_retained.byte_sample_len as usize,
        CODE_INGRESS_BYTE_SAMPLE_CAP
    );
    assert_eq!(
        &last_retained.byte_sample[..CODE_INGRESS_BYTE_SAMPLE_CAP],
        &bytes[..CODE_INGRESS_BYTE_SAMPLE_CAP]
    );
    assert!(
        state
            .recent_frames
            .iter()
            .all(|frame| frame.framing_validity == 0.0 || frame.framing_validity == 1.0),
        "frame validity must be clamped into [0, 1]"
    );
}

#[test]
fn targeted_ingress_sources_do_not_reintroduce_rejected_architecture_or_body_output() {
    let sources = [
        ("vib", include_str!("../src/vib.rs")),
        ("phot_camera", include_str!("../src/phot_camera.rs")),
        ("adapter", include_str!("../src/adapter.rs")),
        (
            "live_byte_carrier",
            include_str!("../src/live_byte_carrier.rs"),
        ),
        ("code_ingress", include_str!("../src/code_ingress.rs")),
    ];
    let forbidden = [
        "semantic_readiness",
        "SemanticReadinessState",
        "refresh_semantic_readiness",
        "symbolizer",
        "SymbolizerState",
        "refresh_symbolizer",
        "concept_id",
        "ObjectConceptBinding",
        "RelationConceptBinding",
        "println!",
        "eprintln!",
        "std::process::Command",
    ];

    for (name, source) in sources {
        for needle in forbidden {
            assert!(
                !source.contains(needle),
                "{name} must not contain rejected architecture or body output surface: {needle}"
            );
        }
    }
}
