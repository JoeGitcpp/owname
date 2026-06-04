use bionic_v0::capability_curriculum_governor::{
    CapabilityGovernorState, CapabilityLayerConfig, LAYER_ATTENTION, LAYER_CURIOSITY,
    LAYER_DEPTH, LAYER_ENCODER, LAYER_RELATION, N_CAPABILITY_LAYERS,
    summarize_capability_nursery,
    test_helpers::{
        attention_snap, cold_snap, curiosity_snap, depth_snap, encoder_snap, relation_snap,
    },
};

// ---------------------------------------------------------------------------
// Cold-start: all probes fail, all layers closed
// ---------------------------------------------------------------------------

#[test]
fn cold_start_all_layers_closed() {
    let config = CapabilityLayerConfig::default();
    let mut gov = CapabilityGovernorState::new(config.encoder.stability_window);
    let ev = gov.update(&cold_snap(), &config);

    assert!(!ev.layers_open[LAYER_ENCODER], "encoder should be closed on cold start");
    assert!(!ev.layers_open[LAYER_DEPTH]);
    assert!(!ev.layers_open[LAYER_RELATION]);
    assert!(!ev.layers_open[LAYER_ATTENTION]);
    assert!(!ev.layers_open[LAYER_CURIOSITY]);
    assert_eq!(ev.highest_open_layer, -1);
    assert_eq!(ev.tick_count, 1);
}

// ---------------------------------------------------------------------------
// Encoder layer opens after warming the stability window
// ---------------------------------------------------------------------------

#[test]
fn encoder_opens_after_stability_window_filled() {
    let config = CapabilityLayerConfig::default();
    let mut gov = CapabilityGovernorState::new(config.encoder.stability_window);

    // Verify window not yet full: after (window-1) encoder snaps, not open.
    let snap = encoder_snap();
    let mut ev = gov.update(&snap, &config);
    for _ in 1..config.encoder.stability_window - 1 {
        ev = gov.update(&snap, &config);
    }
    assert!(
        !ev.layers_open[LAYER_ENCODER],
        "encoder should not open before stability window is filled"
    );

    // One more snap fills the window → encoder opens.
    ev = gov.update(&snap, &config);

    assert!(
        ev.layers_open[LAYER_ENCODER],
        "encoder should open after stability window is filled"
    );
    assert!(!ev.layers_open[LAYER_DEPTH], "depth should remain closed");
    assert!(!ev.layers_open[LAYER_RELATION]);
    assert!(!ev.layers_open[LAYER_ATTENTION]);
    assert!(!ev.layers_open[LAYER_CURIOSITY]);
    assert_eq!(ev.highest_open_layer, LAYER_ENCODER as i8);
}

// ---------------------------------------------------------------------------
// Sequential opening: layers open in order 1→2→3→4→5
// ---------------------------------------------------------------------------

#[test]
fn layers_open_in_order_across_staged_snapshots() {
    let config = CapabilityLayerConfig::default();
    let mut gov = CapabilityGovernorState::new(config.encoder.stability_window);
    let win = config.encoder.stability_window;

    // Stage 1: fill window with encoder-ready snaps → layer 1 opens.
    let enc_snap = encoder_snap();
    for _ in 0..win {
        gov.update(&enc_snap, &config);
    }
    let ev1 = gov.update(&enc_snap, &config);
    assert!(ev1.layers_open[LAYER_ENCODER]);
    assert!(!ev1.layers_open[LAYER_DEPTH]);

    // Stage 2: depth-ready snap (which satisfies encoder too) → layer 2 opens.
    // We need to re-fill the window with depth snaps so encoder stays stable.
    let dep_snap = depth_snap();
    for _ in 0..win {
        gov.update(&dep_snap, &config);
    }
    let ev2 = gov.update(&dep_snap, &config);
    assert!(ev2.layers_open[LAYER_ENCODER]);
    assert!(ev2.layers_open[LAYER_DEPTH]);
    assert!(!ev2.layers_open[LAYER_RELATION]);

    // Stage 3: relation snap → layer 3 opens.
    let rel_snap = relation_snap();
    for _ in 0..win {
        gov.update(&rel_snap, &config);
    }
    let ev3 = gov.update(&rel_snap, &config);
    assert!(ev3.layers_open[LAYER_ENCODER]);
    assert!(ev3.layers_open[LAYER_DEPTH]);
    assert!(ev3.layers_open[LAYER_RELATION]);
    assert!(!ev3.layers_open[LAYER_ATTENTION]);

    // Stage 4: attention snap → layer 4 opens.
    let att_snap = attention_snap();
    for _ in 0..win {
        gov.update(&att_snap, &config);
    }
    let ev4 = gov.update(&att_snap, &config);
    assert!(ev4.layers_open[LAYER_ENCODER]);
    assert!(ev4.layers_open[LAYER_DEPTH]);
    assert!(ev4.layers_open[LAYER_RELATION]);
    assert!(ev4.layers_open[LAYER_ATTENTION]);
    assert!(!ev4.layers_open[LAYER_CURIOSITY]);

    // Stage 5: curiosity snap → layer 5 opens.
    let cur_snap = curiosity_snap();
    for _ in 0..win {
        gov.update(&cur_snap, &config);
    }
    let ev5 = gov.update(&cur_snap, &config);
    for i in 0..N_CAPABILITY_LAYERS {
        assert!(ev5.layers_open[i], "layer {} should be open", i);
    }
    assert_eq!(ev5.highest_open_layer, (N_CAPABILITY_LAYERS - 1) as i8);
}

// ---------------------------------------------------------------------------
// Regression: if a lower layer goes red, all upper layers close immediately
// ---------------------------------------------------------------------------

#[test]
fn lower_layer_turning_red_closes_upper_layers() {
    let config = CapabilityLayerConfig::default();
    let mut gov = CapabilityGovernorState::new(config.encoder.stability_window);
    let win = config.encoder.stability_window;

    // Open all layers by feeding curiosity snaps through the stability window.
    let cur_snap = curiosity_snap();
    for _ in 0..(win + 1) {
        gov.update(&cur_snap, &config);
    }
    let all_open = gov.update(&cur_snap, &config);
    assert!(
        all_open.layers_open[LAYER_CURIOSITY],
        "all layers should be open before regression"
    );

    // Now inject a cold snapshot: encoder fails (no hl metrics, no stable count).
    // All upper layers must immediately close.
    let ev = gov.update(&cold_snap(), &config);
    assert!(
        !ev.layers_open[LAYER_ENCODER],
        "encoder should close after cold snap injected"
    );
    assert!(!ev.layers_open[LAYER_DEPTH], "depth closes when encoder fails");
    assert!(!ev.layers_open[LAYER_RELATION], "relation closes when depth-predecessor fails");
    assert!(!ev.layers_open[LAYER_ATTENTION]);
    assert!(!ev.layers_open[LAYER_CURIOSITY]);
    assert_eq!(ev.highest_open_layer, -1);
}

// ---------------------------------------------------------------------------
// Depth probe fails when only 1 worldline slot exists
// ---------------------------------------------------------------------------

#[test]
fn depth_probe_fails_with_insufficient_worldline_slots() {
    let config = CapabilityLayerConfig::default();
    let mut gov = CapabilityGovernorState::new(config.encoder.stability_window);
    let win = config.encoder.stability_window;

    // Feed encoder snaps to open layer 1.
    let enc = encoder_snap();
    for _ in 0..win {
        gov.update(&enc, &config);
    }
    let ev = gov.update(&enc, &config);
    // Encoder open, depth should be closed (encoder_snap has no worldline slots).
    assert!(ev.layers_open[LAYER_ENCODER]);
    assert!(!ev.layers_open[LAYER_DEPTH]);
    assert_eq!(ev.depth.n_worldline_slots, 0);
}

// ---------------------------------------------------------------------------
// Depth probe fails when all slots move at same velocity (no parallax)
// ---------------------------------------------------------------------------

#[test]
fn depth_probe_fails_without_parallax() {
    use bionic_v0::snapshot::{SnapshotSpatial, SnapshotSlot};
    use bionic_v0::viewport::SpatialWorldlineState;

    // Build a snap where two slots have same span AND same velocity → no parallax.
    let mut s = encoder_snap();
    let uniform_slot = |span: f64| SnapshotSlot {
        active: true,
        last_update_t_us: 1_000,
        source_modality: bionic_v0::MOD_PHOT,
        nomination_seq: 1,
        vib: None,
        phot: None,
        mech: None,
        spatial: Some(SnapshotSpatial {
            pos_x: 0.5,
            pos_y: 0.5,
            span_x: span,
            span_y: span,
            precision: 0.8,
            last_update_t_us: 1_000,
            valid: true,
        }),
        spatial_worldline: Some(SpatialWorldlineState {
            n_updates: 10,
            prev_t_us: 0,
            last_update_t_us: 1_000,
            prev_pos_x: 0.4,
            prev_pos_y: 0.5,
            predicted_pos_x: 0.5,
            predicted_pos_y: 0.5,
            velocity_x_per_s: 0.30,  // same velocity for both
            velocity_y_per_s: 0.0,
            err_x: 0.005,
            err_y: 0.002,
            err_norm: 0.005,
            last_dt_us: 33_000,
            valid: true,
        }),
        fusion: None,
        high_layer: None,
        attention_share: 0.5,
        precision_bias: 0.0,
    };
    s.slots.push(uniform_slot(0.30));
    s.slots.push(uniform_slot(0.05)); // far, same velocity as near → wrong
    s.global_attention.n_active = 2;

    let config = CapabilityLayerConfig::default();
    let mut gov = CapabilityGovernorState::new(config.encoder.stability_window);
    for _ in 0..config.encoder.stability_window {
        gov.update(&s, &config);
    }
    let ev = gov.update(&s, &config);

    // Depth probe should fail: near doesn't have higher velocity than far.
    // (Both have same velocity → depth_sort_accuracy = 0.0 for this pair)
    assert!(!ev.layers_open[LAYER_DEPTH]);
    assert!(ev.depth.depth_sort_accuracy < 1.0);
}

// ---------------------------------------------------------------------------
// Nursery integration: summarize_capability_nursery with all-stage snaps
// ---------------------------------------------------------------------------

#[test]
fn nursery_integration_reaches_all_layers_with_staged_snaps() {
    let config = CapabilityLayerConfig::default();
    let win = config.encoder.stability_window;

    // Provide enough curiosity snaps to fill the stability window and beyond.
    let snaps: Vec<_> = (0..win + 4).map(|_| curiosity_snap()).collect();
    let ev = summarize_capability_nursery(snaps.iter(), &config);

    assert!(ev.layers_open[LAYER_ENCODER]);
    assert!(ev.layers_open[LAYER_CURIOSITY]);
    assert_eq!(ev.highest_open_layer, (N_CAPABILITY_LAYERS - 1) as i8);
    assert!(ev.tick_count as usize >= win + 4);
}

// ---------------------------------------------------------------------------
// Observer fields are populated correctly
// ---------------------------------------------------------------------------

#[test]
fn evidence_fields_reflect_probe_metrics() {
    let config = CapabilityLayerConfig::default();
    let mut gov = CapabilityGovernorState::new(config.encoder.stability_window);
    let win = config.encoder.stability_window;

    let cur = curiosity_snap();
    let mut ev = gov.update(&cold_snap(), &config);
    for _ in 0..win {
        ev = gov.update(&cur, &config);
    }

    // Encoder metrics should be populated from the curiosity snap (which
    // inherits encoder conditions).
    assert!(ev.encoder.hl_avg_mse > 0.0);
    assert!(ev.encoder.pred_to_avg_ratio < 1.0);
    assert_eq!(ev.tick_count, win as u64 + 1);

    // Curiosity metrics should reflect the injected learning progress.
    assert!(ev.curiosity.probe_intensity > 0.0 || ev.curiosity.max_learning_progress > 0.0);

    // probe_green and layers_open should be consistent.
    for i in 0..N_CAPABILITY_LAYERS {
        if i == 0 {
            assert_eq!(ev.layers_open[i], ev.probe_green[i]);
        } else {
            assert_eq!(ev.layers_open[i], ev.layers_open[i - 1] && ev.probe_green[i]);
        }
    }
}

// ---------------------------------------------------------------------------
// Source-level principle guard: no semantic labels in the governor source
// ---------------------------------------------------------------------------

#[test]
fn capability_governor_source_has_no_semantic_labels() {
    let source = include_str!("../src/capability_curriculum_governor.rs");
    let forbidden = [
        "object_name",
        "concept_id",
        "semantic_label",
        "token_id",
        "word_id",
        "text_understanding",
        "image_label",
        "voice_label",
        "prompt_answer",
        "eprintln!",
    ];
    for term in &forbidden {
        assert!(
            !source.contains(term),
            "capability governor source contains rejected term: {term}"
        );
    }
}
