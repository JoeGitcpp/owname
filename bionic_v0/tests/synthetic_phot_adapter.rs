use std::sync::Arc;

use bionic_v0::adapter::{ModalityAdapter, SyntheticPhotAdapter};
use bionic_v0::phot::SyntheticPhotSource;
use bionic_v0::viewport::Viewport;
use bionic_v0::{MOD_PHOT, N_OBJECT_SLOTS, PHOT_BANDS, SLOT_IDLE_US};

#[test]
fn synthetic_phot_is_deterministic() {
    let src = SyntheticPhotSource::new(1_000_000);
    let f1 = src.sample(1_100_000);
    let f2 = src.sample(1_100_000);
    assert_eq!(f1.phot_bands, f2.phot_bands);
    assert_eq!(f1.pos_x, f2.pos_x);
    assert_eq!(f1.pos_y, f2.pos_y);
    assert_eq!(f1.span_x, f2.span_x);
    assert_eq!(f1.span_y, f2.span_y);
    assert_eq!(f1.spatial_precision, f2.spatial_precision);
    assert_eq!(f1.event_energy, f2.event_energy);
}

#[test]
fn synthetic_phot_frame_shape() {
    let src = SyntheticPhotSource::new(0);
    let frame = src.sample(100_000);
    assert_eq!(frame.phot_bands.len(), PHOT_BANDS);
    assert!(frame.pos_x >= 0.0 && frame.pos_x <= 1.0);
    assert!(frame.pos_y >= 0.0 && frame.pos_y <= 1.0);
    assert!(frame.span_x >= 0.0 && frame.span_x <= 1.0);
    assert!(frame.span_y >= 0.0 && frame.span_y <= 1.0);
    assert!(frame.spatial_precision > 0.0);
    assert!(frame.event_energy > 0.0);
}

#[test]
fn phot_writes_viewport() {
    let vp = Arc::new(Viewport::new());
    let mut adapter = SyntheticPhotAdapter::new(0);
    // Drive several ticks across ~300ms of simulated time so the energy
    // smoothing has a chance to cross the nomination threshold.
    for step in 0..20 {
        let t = 1_000_000 + (step as u64) * 16_666;
        adapter.tick(&vp, t);
    }

    let amb = vp.read_ambient();
    assert!(
        amb.phot.event_energy > 0.0,
        "phot ambient.event_energy must become non-zero under synthetic drive"
    );

    let mut has_phot_slot = false;
    let mut has_spatial = false;
    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & MOD_PHOT) != 0 {
            if s.phot.is_some() {
                has_phot_slot = true;
            }
            if s.spatial.is_some() {
                has_spatial = true;
            }
        }
    }
    assert!(has_phot_slot, "at least one slot should carry a phot family");
    assert!(has_spatial, "at least one slot should carry spatial state");
}

#[test]
fn phot_slot_ages_out() {
    let vp = Arc::new(Viewport::new());
    let mut adapter = SyntheticPhotAdapter::new(0);

    // Warm up: accumulate a PHOT slot while the synthetic drive is active.
    let mut last_t = 1_000_000;
    for step in 0..40 {
        last_t = 1_000_000 + (step as u64) * 16_666;
        adapter.tick(&vp, last_t);
    }
    let mut had_active = false;
    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & MOD_PHOT) != 0 {
            had_active = true;
            break;
        }
    }
    assert!(had_active, "pre-idle: a PHOT slot should be active");

    // Force spatial invalidation: the adapter claimed a target; simulate a
    // tick where no fresh estimate exists.
    for i in 0..N_OBJECT_SLOTS {
        vp.invalidate_slot_spatial(i, last_t + 1);
    }
    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if let Some(sp) = s.spatial {
            assert!(!sp.valid, "spatial should be marked invalid after explicit call");
        }
    }

    // Let the slot idle out. `sweep_slots` (invoked by any adapter tick or by
    // direct call) must clear the active flag once SLOT_IDLE_US has elapsed
    // with no new writes.
    let idle_t = last_t + SLOT_IDLE_US + 50_000;
    vp.sweep_slots(idle_t);
    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        assert!(
            !s.active,
            "slot {} should age out after SLOT_IDLE_US without updates",
            i
        );
    }
}
