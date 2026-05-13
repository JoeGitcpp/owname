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
    /// Prediction error hint → R channel. Range [0,1].
    pub err_hint: f64,
    /// Local surprise hint → G channel. Range [0,1].
    pub surprise_hint: f64,
    /// Attention bias hint → B channel. Range [0,1].
    pub attention_bias_hint: f64,
    /// Action output intensity hint → brightness multiplier. Range [0,1].
    pub action_intensity_hint: f64,
}

/// Projected relation edge glyph for visual surface.
#[derive(Clone, Copy, Debug)]
pub struct SurfaceRelationGlyph {
    pub source_slot_id: usize,
    pub target_slot_id: usize,
    pub strength: f64,
    pub surprise: f64,
}

/// The complete projected surface frame derived from a snapshot.
#[derive(Debug)]
pub struct SurfaceFrame {
    pub objects: Vec<SurfaceObjectGlyph>,
    pub relations: Vec<SurfaceRelationGlyph>,
    pub global_probe_intensity: f64,
    pub global_surprise_load: f64,
    pub focus_slot_id: i8,
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

    let relations = project_relations(&snap.global_relations, &snap.slots);

    let focus_slot_id = snap
        .global_reasoning
        .routing_focus_slot_ids
        .first()
        .copied()
        .unwrap_or(-1);

    let probe_intensity = snap.global_reasoning_memory.novelty_score;

    SurfaceFrame {
        objects,
        relations,
        global_probe_intensity: probe_intensity,
        global_surprise_load: snap.global_reasoning.global_surprise_load,
        focus_slot_id,
    }
}

fn project_relations(
    rels: &crate::viewport::GlobalRelationState,
    slots: &[SnapshotSlot],
) -> Vec<SurfaceRelationGlyph> {
    let mut glyphs = Vec::new();

    for edge in &rels.edges {
        if !edge.active {
            continue;
        }

        let src_id = edge.src_slot_id as usize;
        let dst_id = edge.dst_slot_id as usize;

        if src_id >= slots.len() || dst_id >= slots.len() {
            continue;
        }

        let source_active = slots[src_id].active;
        let target_active = slots[dst_id].active;

        if !source_active || !target_active {
            continue;
        }

        glyphs.push(SurfaceRelationGlyph {
            source_slot_id: src_id,
            target_slot_id: dst_id,
            strength: edge.relation_confidence.clamp(0.0, 1.0),
            surprise: edge.pred.relation_surprise.clamp(0.0, 1.0),
        });
    }

    glyphs
}

fn eligible_slot(slot: &SnapshotSlot) -> bool {
    slot.active
}

fn slot_to_glyph(slot_id: usize, slot: &SnapshotSlot, sp: SnapshotSpatial) -> SurfaceObjectGlyph {
    let stability_hint = compute_stability_hint(slot);
    let (err_hint, surprise_hint, attention_bias_hint) = compute_endogenous_hints(slot);

    SurfaceObjectGlyph {
        slot_id,
        center_x: sp.pos_x,
        center_y: sp.pos_y,
        radius_x: sp.span_x.max(0.0),
        radius_y: sp.span_y.max(0.0),
        attention_share: slot.attention_share,
        stability_hint,
        err_hint,
        surprise_hint,
        attention_bias_hint,
        action_intensity_hint: 0.0,
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

fn compute_endogenous_hints(slot: &SnapshotSlot) -> (f64, f64, f64) {
    let mut err_hint = 0.0;
    let mut surprise_hint = 0.0;
    let mut attention_bias_hint = 0.0;

    if let Some(ref hl) = slot.high_layer {
        err_hint = hl.object_err.clamp(0.0, 1.0);
        surprise_hint = hl.predicted_surprise.clamp(0.0, 1.0);
        attention_bias_hint = hl.attention_bias.clamp(0.0, 1.0);
    }

    if let Some(ref fusion) = slot.fusion {
        surprise_hint = surprise_hint.max(fusion.fused_surprise.clamp(0.0, 1.0));
    }

    (err_hint, surprise_hint, attention_bias_hint)
}
