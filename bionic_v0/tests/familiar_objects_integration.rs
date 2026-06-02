//! C5 integration: a re-appearing object (same VIB band-shape) is RE-RECOGNIZED
//! through the viewport, and a consistently-present object emerges as the
//! most-familiar (责任人), all from slots — no id table, no label.

use std::sync::Arc;

use bionic_v0::viewport::Viewport;
use bionic_v0::{MOD_VIB, VIB_BANDS};

fn write_object(vp: &Arc<Viewport>, slot: usize, t: u64, shape: [f64; VIB_BANDS]) {
    let pred = shape.map(|v| v * 0.9);
    let err = shape.map(|v| v * 0.1);
    let prec = [0.5; VIB_BANDS];
    vp.write_slot_vib(slot, t, &shape, &pred, &err, &prec);
}

#[test]
fn reappearing_object_is_re_recognized_through_viewport() {
    let vp = Arc::new(Viewport::new());
    let t0 = 1_000_000u64;
    let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc");

    // Same spectral contour appears repeatedly in the slot.
    let contour = [0.6, 0.2, 0.15, 0.05];
    let mut t = t0;
    for _ in 0..6 {
        write_object(&vp, a, t, contour);
        vp.tick_familiar_objects(t);
        t += 1_000_000;
    }

    let fam = vp.read_familiar_objects();
    assert_eq!(
        fam.objects.len(),
        1,
        "the same recurring contour must be ONE re-recognized object, not many"
    );
    assert!(
        fam.objects[0].recognition_count >= 6,
        "re-recognized each time it reappeared"
    );
    assert!(
        fam.objects[0].familiarity > 0.002,
        "familiarity sediments with repetition (slow: bond is earned over time): {}",
        fam.objects[0].familiarity
    );
}

#[test]
fn consistent_object_emerges_as_most_familiar() {
    let vp = Arc::new(Viewport::new());
    let t0 = 5_000_000u64;
    let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");

    let caregiver = [0.5, 0.3, 0.15, 0.05];
    let stranger = [0.05, 0.15, 0.3, 0.5];
    let mut t = t0;
    // caregiver present often; stranger briefly.
    for k in 0..10 {
        write_object(&vp, a, t, caregiver);
        vp.tick_familiar_objects(t);
        t += 1_000_000;
        if k == 5 {
            write_object(&vp, a, t, stranger);
            vp.tick_familiar_objects(t);
            t += 1_000_000;
        }
    }

    let fam = vp.read_familiar_objects();
    let top = fam.most_familiar().expect("someone most familiar");
    // most-familiar contour should be the caregiver's (band 0 dominant).
    assert!(
        top.fingerprint[0] > top.fingerprint[3],
        "the consistently-present object emerges as 责任人 (no label): {:?}",
        top.fingerprint
    );
}
