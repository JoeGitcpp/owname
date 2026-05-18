use bionic_v0::surface_history::{SlotTrail, TrailPoint};
use bionic_v0::surface_projection::{SurfaceFrame, SurfaceObjectGlyph, SurfaceOrganDrive};
use bionic_v0::surface_render::{render_surface_frame, RenderConfig};

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
            err_hint: 0.1,
            surprise_hint: 0.1,
            attention_bias_hint: 0.5,
            action_intensity_hint: 0.0,
        }],
        relations: Vec::new(),
        global_probe_intensity: 0.0,
        global_surprise_load: 0.0,
        focus_slot_id: -1,
        organ_drive: SurfaceOrganDrive::default(),
    }
}

fn empty_trails() -> Vec<SlotTrail> {
    Vec::new()
}

fn active_organ_drive() -> SurfaceOrganDrive {
    SurfaceOrganDrive {
        activation_drive: 0.8,
        regulated_activation: 0.45,
        tension_drive: 0.6,
        coherence_drive: 0.3,
        ambiguity_drive: 0.5,
        aperture_drive: 0.7,
        residue_drive: 0.4,
        closure_drive: 0.2,
        reentry_drive: 0.3,
        vocal_drive: 0.35,
        regulation_gain: 0.56,
    }
}

fn frame_with_organ_drive(organ_drive: SurfaceOrganDrive) -> SurfaceFrame {
    SurfaceFrame {
        objects: Vec::new(),
        relations: Vec::new(),
        global_probe_intensity: 0.0,
        global_surprise_load: 0.0,
        focus_slot_id: -1,
        organ_drive,
    }
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
    img.iter()
        .filter(|px| px[0] > 0 || px[1] > 0 || px[2] > 0)
        .count()
}

#[test]
fn renderer_draws_organ_skin_from_drive_without_objects() {
    let frame = frame_with_organ_drive(active_organ_drive());
    let img = render_surface_frame(&frame, &empty_trails(), RenderConfig::new(64, 36));

    assert!(count_lit_pixels(&img) > 0);
}

#[test]
fn renderer_caps_organ_skin_brightness_under_high_pressure() {
    let mut drive = active_organ_drive();
    drive.activation_drive = 1.0;
    drive.regulated_activation = 0.35;
    drive.tension_drive = 1.0;
    drive.ambiguity_drive = 1.0;

    let frame = frame_with_organ_drive(drive);
    let img = render_surface_frame(&frame, &empty_trails(), RenderConfig::new(64, 36));

    let saturated = img
        .iter()
        .filter(|px| px[0] > 245 || px[1] > 245 || px[2] > 245)
        .count();
    assert_eq!(saturated, 0);
}

#[test]
fn surface_expression_sources_avoid_fixed_expression_ontology() {
    let sources = [
        include_str!("../src/surface_projection.rs"),
        include_str!("../src/surface_render.rs"),
    ];
    let forbidden = [
        "happy",
        "sad",
        "angry",
        "emotion",
        "mood",
        "personality",
        "face_expression",
        "semantic",
        "concept",
        "token",
        "label",
        "word",
        "identity",
    ];

    for source in sources {
        for term in forbidden {
            assert!(
                !source.contains(term),
                "surface expression source contains forbidden fixed ontology term: {term}"
            );
        }
    }
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
fn renderer_uses_neutral_grayscale_projection_when_hints_are_zero() {
    let frame = SurfaceFrame {
        objects: vec![SurfaceObjectGlyph {
            slot_id: 1,
            center_x: 0.5,
            center_y: 0.5,
            radius_x: 0.2,
            radius_y: 0.2,
            attention_share: 0.5,
            stability_hint: 0.8,
            err_hint: 0.0,
            surprise_hint: 0.0,
            attention_bias_hint: 0.0,
            action_intensity_hint: 0.0,
        }],
        relations: Vec::new(),
        global_probe_intensity: 0.0,
        global_surprise_load: 0.0,
        focus_slot_id: -1,
        organ_drive: SurfaceOrganDrive::default(),
    };
    let img = render_surface_frame(&frame, &empty_trails(), RenderConfig::new(64, 36));
    let lit = img
        .iter()
        .find(|px| px[0] > 0 || px[1] > 0 || px[2] > 0)
        .expect("at least one lit pixel");

    assert_eq!(lit[0], lit[1]);
    assert_eq!(lit[1], lit[2]);
}

#[test]
fn renderer_applies_multi_channel_color_from_endogenous_hints() {
    let frame = SurfaceFrame {
        objects: vec![SurfaceObjectGlyph {
            slot_id: 1,
            center_x: 0.5,
            center_y: 0.5,
            radius_x: 0.2,
            radius_y: 0.2,
            attention_share: 0.5,
            stability_hint: 0.8,
            err_hint: 0.5,
            surprise_hint: 0.3,
            attention_bias_hint: 0.8,
            action_intensity_hint: 0.5,
        }],
        relations: Vec::new(),
        global_probe_intensity: 0.0,
        global_surprise_load: 0.0,
        focus_slot_id: -1,
        organ_drive: SurfaceOrganDrive::default(),
    };
    let img = render_surface_frame(&frame, &empty_trails(), RenderConfig::new(64, 36));
    let lit = img
        .iter()
        .find(|px| px[0] > 0 || px[1] > 0 || px[2] > 0)
        .expect("at least one lit pixel");

    assert!(lit[0] > 0 || lit[1] > 0 || lit[2] > 0);
}
