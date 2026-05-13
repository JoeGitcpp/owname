use bionic_v0::surface_history::SurfaceHistory;
use bionic_v0::surface_projection::{SurfaceFrame, SurfaceObjectGlyph};

fn frame_at(cx: f64, cy: f64, slot_id: usize) -> SurfaceFrame {
    let glyph = SurfaceObjectGlyph {
        slot_id,
        center_x: cx,
        center_y: cy,
        radius_x: 0.1,
        radius_y: 0.1,
        attention_share: 0.5,
        stability_hint: 0.8,
        err_hint: 0.1,
        surprise_hint: 0.1,
        attention_bias_hint: 0.5,
        action_intensity_hint: 0.0,
    };
    SurfaceFrame {
        objects: vec![glyph],
        relations: Vec::new(),
        global_probe_intensity: 0.0,
        global_surprise_load: 0.0,
        focus_slot_id: -1,
    }
}

#[test]
fn history_accumulates_recent_positions_into_trail_points() {
    let mut hist = SurfaceHistory::new(8);
    hist.update(&frame_at(0.2, 0.3, 1));
    hist.update(&frame_at(0.3, 0.35, 1));

    let trails = hist.render_trails();
    assert_eq!(trails.len(), 1);
    assert!(trails[0].points.len() >= 2);
}

#[test]
fn history_eviction_drops_stale_points_after_capacity_limit() {
    let mut hist = SurfaceHistory::new(2);
    hist.update(&frame_at(0.1, 0.1, 1));
    hist.update(&frame_at(0.2, 0.2, 1));
    hist.update(&frame_at(0.3, 0.3, 1));

    let trails = hist.render_trails();
    assert_eq!(trails[0].points.len(), 2);
}

#[test]
fn history_tracks_stable_slot_identity_when_projection_order_changes() {
    let mut hist = SurfaceHistory::new(8);

    hist.update(&SurfaceFrame {
        objects: vec![
            SurfaceObjectGlyph {
                slot_id: 7,
                center_x: 0.2,
                center_y: 0.2,
                radius_x: 0.1,
                radius_y: 0.1,
                attention_share: 0.5,
                stability_hint: 0.8,
                err_hint: 0.1,
                surprise_hint: 0.1,
                attention_bias_hint: 0.5,
                action_intensity_hint: 0.0,
            },
            SurfaceObjectGlyph {
                slot_id: 9,
                center_x: 0.8,
                center_y: 0.8,
                radius_x: 0.1,
                radius_y: 0.1,
                attention_share: 0.5,
                stability_hint: 0.8,
                err_hint: 0.1,
                surprise_hint: 0.1,
                attention_bias_hint: 0.5,
                action_intensity_hint: 0.0,
            },
        ],
        relations: Vec::new(),
        global_probe_intensity: 0.0,
        global_surprise_load: 0.0,
        focus_slot_id: -1,
    });

    hist.update(&SurfaceFrame {
        objects: vec![
            SurfaceObjectGlyph {
                slot_id: 9,
                center_x: 0.75,
                center_y: 0.75,
                radius_x: 0.1,
                radius_y: 0.1,
                attention_share: 0.5,
                stability_hint: 0.8,
                err_hint: 0.1,
                surprise_hint: 0.1,
                attention_bias_hint: 0.5,
                action_intensity_hint: 0.0,
            },
            SurfaceObjectGlyph {
                slot_id: 7,
                center_x: 0.25,
                center_y: 0.25,
                radius_x: 0.1,
                radius_y: 0.1,
                attention_share: 0.5,
                stability_hint: 0.8,
                err_hint: 0.1,
                surprise_hint: 0.1,
                attention_bias_hint: 0.5,
                action_intensity_hint: 0.0,
            },
        ],
        relations: Vec::new(),
        global_probe_intensity: 0.0,
        global_surprise_load: 0.0,
        focus_slot_id: -1,
    });

    let trails = hist.render_trails();
    let slot7 = trails.iter().find(|trail| trail.slot_id == 7).unwrap();
    let slot9 = trails.iter().find(|trail| trail.slot_id == 9).unwrap();

    assert_eq!(slot7.points.len(), 2);
    assert_eq!(slot9.points.len(), 2);
    assert!(slot7.points[0].x < slot7.points[1].x);
    assert!(slot9.points[0].x > slot7.points[1].x);
}

#[test]
fn history_drops_stale_slots_after_consecutive_missing_frames() {
    let mut hist = SurfaceHistory::new(4);
    hist.update(&frame_at(0.2, 0.3, 5));

    for _ in 0..4 {
        hist.update(&SurfaceFrame {
            objects: Vec::new(),
            relations: Vec::new(),
            global_probe_intensity: 0.0,
            global_surprise_load: 0.0,
            focus_slot_id: -1,
        });
    }

    let trails = hist.render_trails();
    assert!(trails.is_empty());
}

#[test]
fn history_alpha_emphasizes_newer_points_over_older_points() {
    let mut hist = SurfaceHistory::new(4);
    hist.update(&frame_at(0.1, 0.1, 3));
    hist.update(&frame_at(0.2, 0.2, 3));
    hist.update(&frame_at(0.3, 0.3, 3));

    let trails = hist.render_trails();
    let trail = trails.iter().find(|trail| trail.slot_id == 3).unwrap();

    assert!(trail.points.first().unwrap().alpha < trail.points.last().unwrap().alpha);
}
