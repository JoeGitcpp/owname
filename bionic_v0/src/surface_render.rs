use crate::surface_history::SlotTrail;
use crate::surface_projection::{SurfaceFrame, SurfaceObjectGlyph, SurfaceRelationGlyph};

/// Configuration for the raster renderer.
#[derive(Clone, Copy, Debug)]
pub struct RenderConfig {
    pub width: usize,
    pub height: usize,
}

impl RenderConfig {
    pub fn new(width: usize, height: usize) -> Self {
        Self { width, height }
    }
}

/// Render a surface frame into a low-resolution RGBA pixel buffer.
///
/// Returns a flat Vec of [R, G, B, A] pixels in row-major order.
/// No fixed human-state color dictionary; intensities derive purely
/// from numeric hints produced by projection and history.
pub fn render_surface_frame(
    frame: &SurfaceFrame,
    trails: &[SlotTrail],
    config: RenderConfig,
) -> Vec<[u8; 4]> {
    let total_pixels = config.width * config.height;
    let mut buffer = vec![[0u8; 4]; total_pixels];

    // Global state background layer
    apply_global_state_background(&mut buffer, frame, config.width, config.height);

    // Observer-side expression layer driven by bounded numeric substrate.
    apply_organ_skin_background(&mut buffer, frame, config.width, config.height);

    // Draw trail energy (background layer)
    for trail in trails {
        for point in &trail.points {
            stamp_trail_point(&mut buffer, point, config.width, config.height);
        }
    }

    // Draw relation edges (middle layer)
    for rel in &frame.relations {
        render_relation_glyph(
            &mut buffer,
            rel,
            &frame.objects,
            config.width,
            config.height,
        );
    }

    // Draw object blobs on top
    for obj in &frame.objects {
        fill_object_blob(&mut buffer, obj, config.width, config.height);
    }

    buffer
}

fn apply_organ_skin_background(
    buffer: &mut [[u8; 4]],
    frame: &SurfaceFrame,
    width: usize,
    height: usize,
) {
    let drive = frame.organ_drive;
    if drive.regulated_activation <= 0.0
        && drive.tension_drive <= 0.0
        && drive.residue_drive <= 0.0
        && drive.reentry_drive <= 0.0
    {
        return;
    }

    let aspect = height as f64 / width.max(1) as f64;

    for y in 0..height {
        let ny = if height <= 1 {
            0.0
        } else {
            y as f64 / (height - 1) as f64
        };
        for x in 0..width {
            let nx = if width <= 1 {
                0.0
            } else {
                x as f64 / (width - 1) as f64
            };
            let dx = (nx - 0.5) * aspect.max(0.1);
            let dy = ny - 0.5;
            let dist = (dx * dx * 2.6 + dy * dy * 1.4).sqrt();
            let core = (1.0 - dist / 0.46).clamp(0.0, 1.0);
            let wide = (1.0 - dist / 0.82).clamp(0.0, 1.0);
            let ring = (1.0 - ((dist - 0.42).abs() / 0.08)).clamp(0.0, 1.0);
            let side = ((nx - 0.5).abs() * 2.0).clamp(0.0, 1.0) * wide;

            let base = drive.regulated_activation.clamp(0.0, 1.0);
            let residue = (drive.residue_drive * 0.65 + drive.reentry_drive * 0.35).clamp(0.0, 1.0);

            let r = base * core * 42.0
                + drive.tension_drive.clamp(0.0, 1.0) * side * 54.0
                + drive.ambiguity_drive.clamp(0.0, 1.0) * ring * 38.0;
            let g = base * core * 36.0
                + drive.coherence_drive.clamp(0.0, 1.0) * wide * 48.0
                + drive.closure_drive.clamp(0.0, 1.0) * core * 30.0;
            let b = base * core * 50.0
                + drive.aperture_drive.clamp(0.0, 1.0) * ring * 56.0
                + drive.vocal_drive.clamp(0.0, 1.0) * wide * 32.0
                + residue * wide * 28.0;

            let idx = y * width + x;
            buffer[idx][0] = buffer[idx][0].max(r.clamp(0.0, 180.0) as u8);
            buffer[idx][1] = buffer[idx][1].max(g.clamp(0.0, 180.0) as u8);
            buffer[idx][2] = buffer[idx][2].max(b.clamp(0.0, 180.0) as u8);
            if buffer[idx][0] > 0 || buffer[idx][1] > 0 || buffer[idx][2] > 0 {
                buffer[idx][3] = 255;
            }
        }
    }
}

fn apply_global_state_background(
    buffer: &mut [[u8; 4]],
    frame: &SurfaceFrame,
    _width: usize,
    _height: usize,
) {
    let bg_intensity = (frame.global_probe_intensity * 24.0) as u8;
    let bg_r = (frame.global_surprise_load * 48.0) as u8;

    for idx in 0..buffer.len() {
        buffer[idx][0] = buffer[idx][0].max(bg_r);
        buffer[idx][1] = buffer[idx][1].max(bg_intensity / 2);
        buffer[idx][2] = buffer[idx][2].max(bg_intensity / 2);
    }
}

fn stamp_trail_point(
    buffer: &mut [[u8; 4]],
    point: &crate::surface_history::TrailPoint,
    width: usize,
    height: usize,
) {
    let px = (point.x * width as f64) as isize;
    let py = (point.y * height as f64) as isize;
    let intensity = (point.alpha * 96.0) as u8;

    if px >= 0 && px < width as isize && py >= 0 && py < height as isize {
        let idx = py as usize * width + px as usize;
        buffer[idx][0] = buffer[idx][0].max(intensity);
        buffer[idx][1] = buffer[idx][1].max(intensity);
        buffer[idx][2] = buffer[idx][2].max(intensity);
        buffer[idx][3] = 255;
    }
}

fn fill_object_blob(buffer: &mut [[u8; 4]], obj: &SurfaceObjectGlyph, width: usize, height: usize) {
    let cx = obj.center_x * width as f64;
    let cy = obj.center_y * height as f64;
    let rx = obj.radius_x * width as f64;
    let ry = obj.radius_y * height as f64;

    let min_x = (cx - rx).max(0.0) as usize;
    let max_x = (cx + rx).min(width as f64 - 1.0) as usize;
    let min_y = (cy - ry).max(0.0) as usize;
    let max_y = (cy + ry).min(height as f64 - 1.0) as usize;

    let base_intensity = ((0.35 + 0.65 * obj.stability_hint.clamp(0.0, 1.0))
        * (96.0 + 159.0 * obj.attention_share.clamp(0.0, 1.0))) as u8;

    // Multi-channel color synthesis
    // R = err_hint (prediction error) - high error → red tint
    // G = surprise_hint + attention_bias_hint * 0.5
    // B = attention_bias_hint
    let r_f = obj.err_hint.clamp(0.0, 1.0);
    let g_f = (obj.surprise_hint + obj.attention_bias_hint * 0.5).clamp(0.0, 1.0);
    let b_f = obj.attention_bias_hint.clamp(0.0, 1.0);

    let intensity_multiplier = 0.5 + 0.5 * obj.action_intensity_hint.clamp(0.0, 1.0);

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = (x as f64 - cx) / rx.max(1.0);
            let dy = (y as f64 - cy) / ry.max(1.0);
            if dx * dx + dy * dy <= 1.0 {
                let idx = y * width + x;
                if idx < buffer.len() {
                    let base = (base_intensity as f64 * intensity_multiplier) as u8;
                    let r = ((r_f * 2.0 / 3.0 + base as f64 / 3.0).clamp(0.0, 255.0)) as u8;
                    let g = ((g_f * 2.0 / 3.0 + base as f64 / 3.0).clamp(0.0, 255.0)) as u8;
                    let b = ((b_f * 2.0 / 3.0 + base as f64 / 3.0).clamp(0.0, 255.0)) as u8;
                    buffer[idx][0] = buffer[idx][0].max(r);
                    buffer[idx][1] = buffer[idx][1].max(g);
                    buffer[idx][2] = buffer[idx][2].max(b);
                    buffer[idx][3] = 255;
                }
            }
        }
    }
}

fn render_relation_glyph(
    buffer: &mut [[u8; 4]],
    rel: &SurfaceRelationGlyph,
    objects: &[SurfaceObjectGlyph],
    width: usize,
    height: usize,
) {
    let source = objects.iter().find(|o| o.slot_id == rel.source_slot_id);
    let target = objects.iter().find(|o| o.slot_id == rel.target_slot_id);

    let (Some(src), Some(tgt)) = (source, target) else {
        return;
    };

    let x0 = (src.center_x * width as f64) as isize;
    let y0 = (src.center_y * height as f64) as isize;
    let x1 = (tgt.center_x * width as f64) as isize;
    let y1 = (tgt.center_y * height as f64) as isize;

    let dx = (x1 - x0).abs();
    let dy = (y1 - y0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx - dy;

    let mut x = x0;
    let mut y = y0;
    let mut continue_drawing = true;

    while continue_drawing {
        if x >= 0 && x < width as isize && y >= 0 && y < height as isize {
            let idx = y as usize * width + x as usize;
            if idx < buffer.len() {
                let alpha = (rel.strength * 128.0) as u8;
                let surprise_r = (rel.surprise * 192.0) as u8;
                buffer[idx][0] = buffer[idx][0].max(surprise_r);
                buffer[idx][1] = buffer[idx][1].max(alpha / 2);
                buffer[idx][2] = buffer[idx][2].max(alpha / 2);
                buffer[idx][3] = buffer[idx][3].max(alpha);
            }
        }

        if x == x1 && y == y1 {
            continue_drawing = false;
        } else {
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
}
