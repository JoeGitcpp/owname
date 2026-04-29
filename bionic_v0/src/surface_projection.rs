use crate::snapshot::{SnapshotData, SnapshotSlot, SnapshotSpatial};

/// A single projected object glyph for the visual surface.
///
/// All fields are normalized [0,1] numeric hints. The renderer interprets
/// them as center, extent, and rendering intensity. No semantic meaning is
/// attached.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceObjectGlyph {
    pub slot_id: usize,
    pub center_x: f64,
    pub center_y: f64,
    pub radius_x: f64,
    pub radius_y: f64,
    pub attention_share: f64,
    pub stability_hint: f64,
}

/// The complete projected surface frame derived from a snapshot.
#[derive(Debug)]
pub struct SurfaceFrame {
    pub objects: Vec<SurfaceObjectGlyph>,
}

/// Project a snapshot into an observer-side surface frame.
///
/// Only active slots with valid spatial data become glyphs.
/// This function is purely a read-side projection and does not
/// modify the snapshot or write back to the body.
pub fn project_surface_frame(snap: &SnapshotData) -> SurfaceFrame {
    let mut objects = Vec::new();

    for (slot_id, slot) in snap.slots.iter().enumerate() {
        if !eligible_slot(slot) {
            continue;
        }

        if let Some(sp) = slot.spatial {
            if sp.valid {
                let glyph = slot_to_glyph(slot_id, slot, sp);
                objects.push(glyph);
            }
        }
    }

    SurfaceFrame { objects }
}

fn eligible_slot(slot: &SnapshotSlot) -> bool {
    slot.active
}

fn slot_to_glyph(slot_id: usize, slot: &SnapshotSlot, sp: SnapshotSpatial) -> SurfaceObjectGlyph {
    let stability_hint = compute_stability_hint(slot);

    SurfaceObjectGlyph {
        slot_id,
        center_x: sp.pos_x,
        center_y: sp.pos_y,
        radius_x: sp.span_x.max(0.0),
        radius_y: sp.span_y.max(0.0),
        attention_share: slot.attention_share,
        stability_hint,
    }
}

fn compute_stability_hint(slot: &SnapshotSlot) -> f64 {
    let mut hint = 1.0;

    if let Some(ref hl) = slot.high_layer {
        let surprise = hl.predicted_surprise;
        let err = hl.object_err;
        hint *= 1.0 - surprise.clamp(0.0, 1.0);
        hint *= 1.0 - err.clamp(0.0, 1.0);
    }

    if let Some(ref fusion) = slot.fusion {
        let fused_surprise = fusion.fused_surprise;
        hint *= 1.0 - fused_surprise.clamp(0.0, 1.0);
    }

    hint.clamp(0.0, 1.0)
}
