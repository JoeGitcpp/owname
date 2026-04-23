use bionic_v0::surface_history::SurfaceHistory;
use bionic_v0::surface_projection::{SurfaceFrame, SurfaceObjectGlyph};

fn frame_at(cx: f64, cy: f64, _slot_seq: u64) -> SurfaceFrame {
    let glyph = SurfaceObjectGlyph {
        center_x: cx,
        center_y: cy,
        radius_x: 0.1,
        radius_y: 0.1,
        attention_share: 0.5,
        stability_hint: 0.8,
    };
    SurfaceFrame {
        objects: vec![glyph],
    }
}

#[test]
fn history_accumulates_recent_positions_into_trail_points() {
    let mut hist = SurfaceHistory::new(8);
    hist.update(&frame_at(0.2, 0.3, 1));
    hist.update(&frame_at(0.3, 0.35, 2));

    let trails = hist.render_trails();
    assert_eq!(trails.len(), 1);
    assert!(trails[0].points.len() >= 2);
}

#[test]
fn history_eviction_drops_stale_points_after_capacity_limit() {
    let mut hist = SurfaceHistory::new(2);
    hist.update(&frame_at(0.1, 0.1, 1));
    hist.update(&frame_at(0.2, 0.2, 2));
    hist.update(&frame_at(0.3, 0.3, 3));

    let trails = hist.render_trails();
    assert_eq!(trails[0].points.len(), 2);
}
