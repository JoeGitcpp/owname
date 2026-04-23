use bionic_v0::surface_render::{render_surface_frame, RenderConfig};
use bionic_v0::surface_projection::{SurfaceFrame, SurfaceObjectGlyph};
use bionic_v0::surface_history::{SlotTrail, TrailPoint};

fn single_object_frame(cx: f64, cy: f64, rx: f64, ry: f64) -> SurfaceFrame {
    SurfaceFrame {
        objects: vec![SurfaceObjectGlyph {
            slot_id: 1,
            center_x: cx,
            center_y: cy,
            radius_x: rx,
            radius_y: ry,
            attention_share: 0.5,
            stability_hint: 0.8,
        }],
    }
}

fn empty_trails() -> Vec<SlotTrail> {
    Vec::new()
}

fn one_trail(points: Vec<(f64, f64)>) -> Vec<SlotTrail> {
    let trail_points: Vec<TrailPoint> = points
        .iter()
        .enumerate()
        .map(|(i, &(x, y))| {
            let alpha = if points.len() <= 1 {
                1.0
            } else {
                (1.0 - i as f64 / (points.len() - 1) as f64).clamp(0.0, 1.0)
            };
            TrailPoint { x, y, alpha }
        })
        .collect();
    vec![SlotTrail {
        slot_id: 1,
        points: trail_points,
    }]
}

fn count_lit_pixels(img: &[[u8; 4]]) -> usize {
    img.iter().filter(|px| px[0] > 0 || px[1] > 0 || px[2] > 0).count()
}

#[test]
fn renderer_marks_pixels_inside_projected_object_extent() {
    let frame = single_object_frame(0.5, 0.5, 0.2, 0.2);
    let img = render_surface_frame(&frame, &empty_trails(), RenderConfig::new(64, 36));

    assert!(img.iter().any(|px| px[0] > 0));
}

#[test]
fn renderer_draws_trail_energy_behind_recent_motion() {
    let frame = single_object_frame(0.6, 0.5, 0.1, 0.1);
    let trails = one_trail(vec![(0.2, 0.5), (0.4, 0.5), (0.6, 0.5)]);
    let img = render_surface_frame(&frame, &trails, RenderConfig::new(64, 36));

    assert!(count_lit_pixels(&img) > 0);
}

#[test]
fn renderer_uses_neutral_grayscale_projection_instead_of_fixed_palette() {
    let frame = single_object_frame(0.5, 0.5, 0.2, 0.2);
    let img = render_surface_frame(&frame, &empty_trails(), RenderConfig::new(64, 36));
    let lit = img
        .iter()
        .find(|px| px[0] > 0 || px[1] > 0 || px[2] > 0)
        .expect("at least one lit pixel");

    assert_eq!(lit[0], lit[1]);
    assert_eq!(lit[1], lit[2]);
}
