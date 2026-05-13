use std::sync::Arc;

use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::{GlobalRelationState, RelationEdgeState, Viewport};
use bionic_v0::{MOD_PHOT, MOD_VIB, PHOT_BANDS, VIB_BANDS};

fn seed_attended_slot(vp: &Arc<Viewport>, slot_id: usize, t_us: u64) {
    let vib_obs = [0.1f64; VIB_BANDS];
    let vib_pred = [0.09f64; VIB_BANDS];
    let vib_err = [0.01f64; VIB_BANDS];
    let vib_prec = [0.5f64; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &vib_obs, &vib_pred, &vib_err, &vib_prec);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);

    let phot_obs = vec![0.2f64; PHOT_BANDS];
    let phot_pred = vec![0.18f64; PHOT_BANDS];
    let phot_err = vec![0.02f64; PHOT_BANDS];
    let phot_prec = vec![0.4f64; PHOT_BANDS];
    vp.write_slot_phot(slot_id, t_us, &phot_obs, &phot_pred, &phot_err, &phot_prec);
    vp.refresh_slot_fusion(slot_id, MOD_PHOT, t_us);

    vp.refresh_slot_high_layer(slot_id, t_us);
}

fn relation_edge(gr: &GlobalRelationState, src: usize, dst: usize) -> &RelationEdgeState {
    gr.edges
        .iter()
        .find(|e| e.src_slot_id as usize == src && e.dst_slot_id as usize == dst)
        .expect("relation edge")
}

#[test]
fn spatial_writes_form_slot_worldline_with_prediction_error() {
    let vp = Viewport::new();
    let slot = vp.allocate_slot(MOD_PHOT, 1_000, 1).expect("slot");

    vp.write_slot_spatial(slot, MOD_PHOT, 1_000, 0.10, 0.20, 0.10, 0.10, 0.7);
    vp.write_slot_spatial(slot, MOD_PHOT, 11_000, 0.20, 0.25, 0.10, 0.10, 0.8);
    vp.write_slot_spatial(slot, MOD_PHOT, 21_000, 0.31, 0.31, 0.10, 0.10, 0.9);

    let snap = vp.read_slot(slot).expect("slot snapshot");
    let wl = snap.spatial_worldline.expect("worldline");

    assert_eq!(wl.n_updates, 3);
    assert_eq!(wl.prev_t_us, 11_000);
    assert_eq!(wl.last_update_t_us, 21_000);
    assert!(wl.valid);
    assert!(wl.velocity_x_per_s > 0.0);
    assert!(wl.velocity_y_per_s > 0.0);
    assert!(wl.predicted_pos_x > 0.29 && wl.predicted_pos_x < 0.31);
    assert!(wl.predicted_pos_y > 0.29 && wl.predicted_pos_y < 0.31);
    assert!(wl.err_norm > 0.0);
}

#[test]
fn global_relations_project_spatial_distance_from_worldlines() {
    let vp = Arc::new(Viewport::new());
    let a = vp.allocate_slot(MOD_PHOT, 1_000, 1).expect("slot a");
    let b = vp.allocate_slot(MOD_PHOT, 1_000, 2).expect("slot b");

    seed_attended_slot(&vp, a, 10_000);
    seed_attended_slot(&vp, b, 10_000);
    vp.write_slot_spatial(a, MOD_PHOT, 10_000, 0.10, 0.10, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(b, MOD_PHOT, 10_000, 0.80, 0.80, 0.10, 0.10, 0.9);

    vp.refresh_global_attention(10_000);
    vp.refresh_global_relations(10_000);
    let far = vp.read_global_relations();
    let far_edge = relation_edge(&far, a, b);

    vp.write_slot_spatial(b, MOD_PHOT, 20_000, 0.18, 0.18, 0.10, 0.10, 0.9);
    seed_attended_slot(&vp, a, 20_000);
    seed_attended_slot(&vp, b, 20_000);
    vp.refresh_global_attention(20_000);
    vp.refresh_global_relations(20_000);
    let near = vp.read_global_relations();
    let near_edge = relation_edge(&near, a, b);

    assert!(near_edge.spatial_distance_score > far_edge.spatial_distance_score);
    assert!(near_edge.spatial_overlap_score > far_edge.spatial_overlap_score);
}

#[test]
fn snapshot_preserves_spatial_worldline_and_spatial_relation_projection() {
    let vp = Arc::new(Viewport::new());
    let a = vp.allocate_slot(MOD_PHOT, 1_000, 1).expect("slot a");
    let b = vp.allocate_slot(MOD_PHOT, 1_000, 2).expect("slot b");

    seed_attended_slot(&vp, a, 10_000);
    seed_attended_slot(&vp, b, 10_000);
    vp.write_slot_spatial(a, MOD_PHOT, 10_000, 0.10, 0.10, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(a, MOD_PHOT, 20_000, 0.20, 0.20, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(b, MOD_PHOT, 20_000, 0.24, 0.24, 0.20, 0.20, 0.9);
    vp.refresh_global_attention(20_000);
    vp.refresh_global_relations(20_000);

    let dir = std::env::temp_dir().join("bionic_v0_spacetime_worldline_snap");
    std::fs::create_dir_all(&dir).ok();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).expect("write ok");
    let snap = read_snapshot(&paths.final_path).expect("read ok");

    let wl = snap.slots[a].spatial_worldline.expect("worldline");
    assert_eq!(wl.n_updates, 2);
    assert!(wl.err_norm >= 0.0);

    let edge = relation_edge(&snap.global_relations, a, b);
    assert!(edge.spatial_distance_score > 0.0);
    assert!(edge.spatial_overlap_score > 0.0);
}

#[test]
fn spacetime_substrate_does_not_become_request_response_reasoner() {
    let vp_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/viewport.rs"))
        .expect("read viewport.rs");

    for forbidden in [
        "spatial_reasoner",
        "temporal_reasoner",
        "scene_understanding",
        "request_response",
        "answer_model",
        "SpatialReasoner",
        "TemporalReasoner",
    ] {
        assert!(
            !vp_src.contains(forbidden),
            "space-time sense must remain a continuous body substrate, not `{}`",
            forbidden
        );
    }
}
