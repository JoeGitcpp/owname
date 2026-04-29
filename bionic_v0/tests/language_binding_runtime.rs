use std::sync::Arc;

use bionic_v0::adapter::{ModalityAdapter, RealPhotAdapter, SyntheticVibAdapter};
use bionic_v0::language_binding::{
    BindingEvent, BindingFeedback, BindingStatus, BindingTargetRef, PatternKind,
};
use bionic_v0::phot_camera::{SynthCamStep, SyntheticCameraFrameProvider};
use bionic_v0::synthetic_vib::SyntheticVibSource;
use bionic_v0::viewport::Viewport;

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
