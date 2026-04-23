use crate::surface_history::SlotTrail;
use crate::surface_projection::SurfaceFrame;

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
/// No fixed human-emotion color dictionary; intensities derive purely
/// from numeric hints produced by projection and history.
pub fn render_surface_frame(
    frame: &SurfaceFrame,
    trails: &[SlotTrail],
    config: RenderConfig,
) -> Vec<[u8; 4]> {
    let total_pixels = config.width * config.height;
    let mut buffer = vec![[0u8; 4]; total_pixels];

    // Draw trail energy first (background layer)
    for trail in trails {
        for point in &trail.points {
            stamp_trail_point(&mut buffer, point, config.width, config.height);
        }
    }

    // Draw object blobs on top
    for obj in &frame.objects {
        fill_object_blob(&mut buffer, obj, config.width, config.height);
    }

    buffer
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

fn fill_object_blob(
    buffer: &mut [[u8; 4]],
    obj: &crate::surface_projection::SurfaceObjectGlyph,
    width: usize,
    height: usize,
) {
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

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            // Check if pixel is within the ellipse
            let dx = (x as f64 - cx) / rx.max(1.0);
            let dy = (y as f64 - cy) / ry.max(1.0);
            if dx * dx + dy * dy <= 1.0 {
                let idx = y * width + x;
                if idx < buffer.len() {
                    buffer[idx][0] = buffer[idx][0].max(base_intensity);
                    buffer[idx][1] = buffer[idx][1].max(base_intensity);
                    buffer[idx][2] = buffer[idx][2].max(base_intensity);
                    buffer[idx][3] = 255;
                }
            }
        }
    }
}
