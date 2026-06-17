// Virtual neck — controlled synthetic camera with known translation.
//
// Phase A: implement a purely deterministic, CPU-side virtual neck that
// translates a synthetic camera laterally (X axis) through a 3D scene.
// Every tick produces:
//   (CameraFrame, EfferenceCopy)
//
// The camera TRANSLATES — it does NOT rotate.  Translation-only motion is
// required for parallax: near objects appear to move more than far objects.
// Pure rotation produces zero parallax and no depth signal.
//
// Scene model (pinhole projection):
//   screen_x = (x_world − cam_x) · focal_px / z_world
//   screen_y = y_world · focal_px / z_world
//   px = CAM_W/2 + screen_x,  py = CAM_H/2 + screen_y
//
// Curriculum: amplitude grows from NECK_AMP_INIT to NECK_AMP_MAX over
// NECK_PHASE_TICKS ticks per stage.  Starts imperceptibly small so the
// predictor can warm up on a near-static world before seeing motion.
//
// Integration with Viewport:
//   1. Call tick() to get (frame, ec).
//   2. vp.push_efference_copy(ec)
//   3. vp.inject_efference_motion_priors()   ← before predictor ticks
//   4. Push region_perception(frame, prev) observations to vp
//   5. (per-Gaussian depth update now runs inside vp.tick_gaussian_field)
//   6. vp.apply_action_output_event(NeckTranslation Begin, magnitude=|tx|)
//   7. Optionally: vp.accumulate_ingress_change(t, motion_energy)

use std::f64::consts::PI;
use std::sync::Arc;

use crate::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use crate::efference_copy::EfferenceCopy;
use crate::phot_camera::{CameraFrame, CAM_H, CAM_PIX, CAM_W};
use crate::viewport::Viewport;

// ── Curriculum constants ────────────────────────────────────────────────────

/// Starting amplitude (normalized frame-width units) — imperceptibly small.
pub const NECK_AMP_INIT: f64 = 0.002;
/// Amplitude increment per curriculum phase.
pub const NECK_AMP_STEP: f64 = 0.003;
/// Maximum amplitude.
pub const NECK_AMP_MAX: f64 = 0.10;
/// How many ticks each curriculum phase lasts before amplitude steps up.
pub const NECK_PHASE_TICKS: u32 = 200;
/// Number of ticks per full sine-wave wobble cycle.
pub const NECK_WOBBLE_PERIOD: f64 = 60.0;

// ── Scene constants ─────────────────────────────────────────────────────────

/// Conceptual focal length in pixels.  320 px wide ÷ 2 rad FOV ≈ 160 px/rad.
pub const NECK_FOCAL_PX: f64 = 160.0;
/// Background brightness (near-black).
pub const NECK_BG: f64 = 0.02;

// ─────────────────────────────────────────────────────────────────────────────

/// One scene object (3D point light source rendered as a soft blob).
#[derive(Clone, Debug)]
pub struct SceneObject {
    /// World X position (normalised, 0 = image centre).
    pub x_world: f64,
    /// World Y position (normalised, 0 = image centre).
    pub y_world: f64,
    /// Depth from camera (positive; larger = farther away).
    /// Inverse depth ρ = 1/z_world.
    pub z_world: f64,
    /// Peak brightness of the blob [0,1].
    pub brightness: f64,
    /// Blob radius in normalised world units.
    pub radius_world: f64,
}

impl SceneObject {
    /// Inverse depth ρ = 1/z_world.
    #[inline]
    pub fn inv_depth(&self) -> f64 {
        1.0 / self.z_world.max(1e-6)
    }
}

/// Neck amplitude curriculum — steps up after each phase.
#[derive(Clone, Debug)]
pub struct NeckCurriculum {
    pub current_phase: u32,
    pub ticks_in_phase: u32,
    pub amplitude: f64,
}

impl Default for NeckCurriculum {
    fn default() -> Self {
        Self {
            current_phase: 0,
            ticks_in_phase: 0,
            amplitude: NECK_AMP_INIT,
        }
    }
}

impl NeckCurriculum {
    /// Advance one tick and return the current amplitude.
    pub fn tick(&mut self) -> f64 {
        self.ticks_in_phase += 1;
        if self.ticks_in_phase >= NECK_PHASE_TICKS {
            self.ticks_in_phase = 0;
            self.current_phase = self.current_phase.saturating_add(1);
            self.amplitude =
                (self.amplitude + NECK_AMP_STEP).min(NECK_AMP_MAX);
        }
        self.amplitude
    }
}

/// Virtual neck: a laterally-translating camera in a 3D scene.
pub struct VirtualNeck {
    pub scene: Vec<SceneObject>,
    pub curriculum: NeckCurriculum,
    /// Monotonic tick counter (used to derive sine-wave phase).
    pub tick_count: u64,
    /// Wobble period in ticks.
    pub wobble_period: f64,
    /// Focal length in pixels.
    pub focal_px: f64,
}

impl VirtualNeck {
    /// A default 4-object scene placing two near objects (z=1.0) and two far
    /// objects (z=5.0) in opposing 2×2 grid regions.  Near objects are large
    /// enough (radius_world=0.25, which renders to ~40 px at z=1.0) to
    /// produce clearly detectable region-level brightness changes.
    ///
    /// Layout (x_world → left/right, y_world → top/bottom):
    ///   Near (z=1.0):  top-left  (−0.25, −0.20)  and  bottom-left  (−0.25, +0.20)
    ///   Far  (z=5.0):  top-right (+0.25, −0.20)  and  bottom-right (+0.25, +0.20)
    ///
    /// With focal_px=160:
    ///   Near blob radius ≈ 0.25 × 160 / 1.0 = 40 px (≈π×40²=5027 px, 26 % of region)
    ///   Far  blob radius ≈ 0.25 × 160 / 5.0 =  8 px  (small, depth contrast)
    pub fn new_default() -> Self {
        let scene = vec![
            // Near objects — left half of frame
            SceneObject {
                x_world: -0.25,
                y_world: -0.20,
                z_world: 1.0,
                brightness: 0.80,
                radius_world: 0.25,
            },
            SceneObject {
                x_world: -0.25,
                y_world: 0.20,
                z_world: 1.0,
                brightness: 0.80,
                radius_world: 0.25,
            },
            // Far objects — right half of frame
            SceneObject {
                x_world: 0.25,
                y_world: -0.20,
                z_world: 5.0,
                brightness: 0.70,
                radius_world: 0.25,
            },
            SceneObject {
                x_world: 0.25,
                y_world: 0.20,
                z_world: 5.0,
                brightness: 0.70,
                radius_world: 0.25,
            },
        ];
        Self {
            scene,
            curriculum: NeckCurriculum::default(),
            tick_count: 0,
            wobble_period: NECK_WOBBLE_PERIOD,
            focal_px: NECK_FOCAL_PX,
        }
    }

    /// Construct with a custom scene (for tests).
    pub fn with_scene(scene: Vec<SceneObject>) -> Self {
        Self {
            scene,
            curriculum: NeckCurriculum::default(),
            tick_count: 0,
            wobble_period: NECK_WOBBLE_PERIOD,
            focal_px: NECK_FOCAL_PX,
        }
    }

    /// Advance one tick.  Returns the rendered `CameraFrame` and the
    /// `EfferenceCopy` (the translation commanded THIS tick = Δtx).
    ///
    /// Camera position follows a sine wave:
    ///   cam_x(n) = amplitude · sin(2π · n / T)
    ///
    /// The efference copy carries the DELTA tx from tick n-1 to tick n, which
    /// is what drives parallax-based depth estimation (not the absolute position).
    pub fn tick(&mut self, t_us: u64) -> (CameraFrame, EfferenceCopy) {
        let amplitude = self.curriculum.tick();
        let period = self.wobble_period;

        let phase_now = 2.0 * PI * self.tick_count as f64 / period;
        let phase_prev = if self.tick_count == 0 {
            0.0
        } else {
            2.0 * PI * (self.tick_count - 1) as f64 / period
        };

        let cam_x_now = amplitude * phase_now.sin();
        let cam_x_prev = amplitude * phase_prev.sin();
        let delta_tx = cam_x_now - cam_x_prev;

        self.tick_count = self.tick_count.saturating_add(1);

        let frame = self.render_frame(t_us, cam_x_now);
        let ec = EfferenceCopy {
            tx: delta_tx,
            ty: 0.0,
            tz: 0.0,
            t_us,
        };
        (frame, ec)
    }

    /// Render a CameraFrame from camera position `cam_x` (world X offset).
    fn render_frame(&self, t_us: u64, cam_x: f64) -> CameraFrame {
        let mut luma = vec![NECK_BG; CAM_PIX];
        let mut r_ch = vec![NECK_BG; CAM_PIX];
        let mut g_ch = vec![NECK_BG; CAM_PIX];
        let mut b_ch = vec![NECK_BG; CAM_PIX];

        let cx = CAM_W as f64 / 2.0;
        let cy = CAM_H as f64 / 2.0;

        for obj in &self.scene {
            // Pinhole projection: world → screen (pixel) coordinates.
            let sx = (obj.x_world - cam_x) * self.focal_px / obj.z_world;
            let sy = obj.y_world * self.focal_px / obj.z_world;
            let px = cx + sx;
            let py = cy + sy;
            // Blob radius in pixels (perspective scaling).
            let rp = (obj.radius_world * self.focal_px / obj.z_world).max(1.0);
            let rp2 = rp * rp;

            // Bounding box (clamped to frame).
            let x0 = ((px - rp).floor().max(0.0) as usize).min(CAM_W);
            let x1 = ((px + rp).ceil().min(CAM_W as f64 - 1.0) as usize + 1).min(CAM_W);
            let y0 = ((py - rp).floor().max(0.0) as usize).min(CAM_H);
            let y1 = ((py + rp).ceil().min(CAM_H as f64 - 1.0) as usize + 1).min(CAM_H);

            for oy in y0..y1 {
                for ox in x0..x1 {
                    let dx = ox as f64 - px;
                    let dy = oy as f64 - py;
                    let d2 = dx * dx + dy * dy;
                    if d2 <= rp2 {
                        // Soft Gaussian-like falloff inside blob radius.
                        let t = 1.0 - d2 / rp2;
                        let v = NECK_BG + obj.brightness * t;
                        let v = v.clamp(0.0, 1.0);
                        let i = oy * CAM_W + ox;
                        if v > luma[i] {
                            luma[i] = v;
                            r_ch[i] = v;
                            g_ch[i] = v;
                            b_ch[i] = v;
                        }
                    }
                }
            }
        }

        CameraFrame {
            t_us,
            luma,
            r: r_ch,
            g: g_ch,
            b: b_ch,
        }
    }

    /// Project one world point through the camera at position `cam_x`.
    /// Returns pixel coordinates (px, py) — may be outside frame bounds.
    pub fn project(&self, x_world: f64, y_world: f64, z_world: f64, cam_x: f64) -> (f64, f64) {
        let cx = CAM_W as f64 / 2.0;
        let cy = CAM_H as f64 / 2.0;
        let sx = (x_world - cam_x) * self.focal_px / z_world;
        let sy = y_world * self.focal_px / z_world;
        (cx + sx, cy + sy)
    }
}

// ── Viewport integration helpers ────────────────────────────────────────────

/// Write a NeckTranslation ActionOutputEvent for the given delta_tx.
///
/// Begins a new write epoch when |delta_tx| ≥ EFFERENCE_MIN_TT so that
/// action-consequence temporal pairing can link the translation command to
/// its subsequent visual consequence.
pub fn write_neck_translation_event(vp: &Arc<Viewport>, delta_tx: f64, t_us: u64) {
    use crate::efference_copy::EFFERENCE_MIN_TT;
    let mag = delta_tx.abs().clamp(0.0, 1.0);
    if mag < EFFERENCE_MIN_TT {
        return;
    }
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::NeckTranslation,
        feedback: ActionOutputFeedback::Begin,
        magnitude: mag,
        event_t_us: t_us,
    });
}

/// Write a visual-consequence ingress record when motion energy is above
/// threshold.  Uses GenericIngress modality — the substrate treats it as
/// generic visual feedback from the neck translation.
pub fn accumulate_neck_visual_consequence(vp: &Arc<Viewport>, motion_energy: f64, t_us: u64) {
    use crate::INGRESS_ACCUMULATION_THRESHOLD;
    if motion_energy > INGRESS_ACCUMULATION_THRESHOLD {
        vp.accumulate_ingress_change(t_us, motion_energy.clamp(0.0, 1.0));
    }
}

/// Write the NeckTranslation action-output event for `ec` (action-consequence
/// pairing). The full per-tick pipeline is `tick_virtual_neck_into_viewport`.
pub fn tick_neck_action_output(vp: &Arc<Viewport>, ec: &EfferenceCopy) {
    write_neck_translation_event(vp, ec.tx, ec.t_us);
}

/// Body-side driver: advance the virtual neck one tick and run the FULL
/// efference → perception → per-Gaussian-depth pipeline into the viewport.
/// Returns the new per-region brightness baseline (for the next tick's motion
/// diff). This is the curriculum vision source that makes depth observable in
/// the live loop: the body commands its OWN lateral eye-movement (a genuine
/// efference copy), so per-Gaussian parallax depth can update — unlike the
/// passive real camera, where tt = None and depth stays unobservable.
///
/// Call order matches the design contract:
///   1. push efference copy (the body's record of the motion it commanded)
///   2. inject motion priors from current per-Gaussian depth (before predictors)
///   3. push this frame's region physical-quantity axes (same as RealPhotAdapter)
///   4. tick the Gaussian field (re-fits μ) + per-Gaussian depth update (uses ec)
///   5. write the NeckTranslation action-output event (action-consequence pairing)
pub fn tick_virtual_neck_into_viewport(
    vp: &Arc<Viewport>,
    neck: &mut VirtualNeck,
    prev_brightness: &[f64; crate::VIS_REGIONS],
    t_us: u64,
) -> [f64; crate::VIS_REGIONS] {
    let (frame, ec) = neck.tick(t_us);
    vp.push_efference_copy(ec);
    vp.inject_efference_motion_priors();
    let (vis_axes, new_brightness) =
        crate::phot_camera::region_perception(&frame, prev_brightness);
    for (i, &v) in vis_axes.iter().enumerate() {
        vp.push_observation(crate::AXIS_VIS_BASE + i, t_us, v);
    }
    vp.tick_gaussian_field(&frame, t_us);
    // Phase 3 Stage 2: tier-0 visual frontend, parallel to slot path.
    vp.tick_vision_tier0_from_frame(&frame, t_us);
    write_neck_translation_event(vp, ec.tx, t_us);
    new_brightness
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::phot_camera::region_perception;
    use crate::{VIS_REGIONS, VIS_SUB_BRIGHTNESS, VIS_SUB_MOTION};

    // ── Parallax sign tests ──────────────────────────────────────────────────

    /// Near objects must be displaced more than far objects for the same
    /// lateral translation.  This is the fundamental parallax requirement.
    #[test]
    fn near_far_pixel_displacement_sign() {
        let near_z = 1.5;
        let far_z = 6.0;
        let neck = VirtualNeck {
            scene: vec![],
            curriculum: NeckCurriculum::default(),
            tick_count: 0,
            wobble_period: NECK_WOBBLE_PERIOD,
            focal_px: NECK_FOCAL_PX,
        };

        let tx = 0.05;
        // Object at world origin viewed with camera at cam_x=0 vs cam_x=tx.
        let (px0_near, _) = neck.project(0.0, 0.0, near_z, 0.0);
        let (px1_near, _) = neck.project(0.0, 0.0, near_z, tx);
        let disp_near = (px1_near - px0_near).abs();

        let (px0_far, _) = neck.project(0.0, 0.0, far_z, 0.0);
        let (px1_far, _) = neck.project(0.0, 0.0, far_z, tx);
        let disp_far = (px1_far - px0_far).abs();

        assert!(
            disp_near > disp_far,
            "near ({near_z}) disp={disp_near:.4} must exceed far ({far_z}) disp={disp_far:.4}"
        );

        // Displacement is proportional to 1/z: check the ratio is ≈ far_z/near_z.
        let expected_ratio = far_z / near_z;
        let actual_ratio = disp_near / disp_far;
        assert!(
            (actual_ratio - expected_ratio).abs() < 0.01,
            "disp ratio {actual_ratio:.4} expected {expected_ratio:.4}"
        );
    }

    /// The efference copy's translation_term must be Some(|tx|) for motion
    /// above threshold and None when still.
    #[test]
    fn efference_copy_translation_term() {
        use crate::efference_copy::EFFERENCE_MIN_TT;
        let moving = EfferenceCopy {
            tx: 0.05,
            ty: 0.0,
            tz: 0.0,
            t_us: 0,
        };
        assert!(moving.translation_term().is_some());
        assert!((moving.translation_term().unwrap() - 0.05).abs() < 1e-12);

        let still = EfferenceCopy {
            tx: EFFERENCE_MIN_TT * 0.5,
            ty: 0.0,
            tz: 0.0,
            t_us: 0,
        };
        assert!(still.translation_term().is_none(), "sub-threshold tx should be None");
    }

    /// tick() output: two consecutive frames with a non-zero curriculum amplitude
    /// must yield a non-zero efference copy.
    #[test]
    fn tick_produces_efference_copy() {
        let mut neck = VirtualNeck::new_default();
        // Fast-forward through the first phase so amplitude is non-trivial.
        for _ in 0..NECK_PHASE_TICKS {
            neck.tick(0);
        }
        // The sine wave should produce a non-zero delta at non-zero phase.
        let mut got_nonzero = false;
        for i in 0..NECK_WOBBLE_PERIOD as u32 {
            let (_, ec) = neck.tick(i as u64 * 33_000);
            if ec.translation_term().is_some() {
                got_nonzero = true;
                break;
            }
        }
        assert!(got_nonzero, "should have at least one tick with observable translation");
    }

    /// Region-level brightness in each region must be clearly above background
    /// when an object is present.
    #[test]
    fn rendered_frame_has_non_background_regions() {
        let neck = VirtualNeck::new_default();
        let prev = [0.0_f64; VIS_REGIONS];
        let (frame, _) = {
            let mut n = neck;
            // Skip curriculum warm-up: force amplitude on next tick.
            for _ in 0..NECK_PHASE_TICKS {
                n.curriculum.tick();
            }
            n.tick(0)
        };
        let (axes, _) = region_perception(&frame, &prev);
        // At least two regions should have above-background brightness.
        let bright_regions = (0..VIS_REGIONS)
            .filter(|&r| axes[r * 5 + VIS_SUB_BRIGHTNESS] > NECK_BG + 0.05)
            .count();
        assert!(
            bright_regions >= 2,
            "expected ≥2 bright regions, got {bright_regions}"
        );
    }

    /// Two consecutive frames with a non-zero translation must produce non-zero
    /// motion in at least one region.
    #[test]
    fn consecutive_frames_produce_motion() {
        let mut neck = VirtualNeck::new_default();
        // Build up enough amplitude to make motion visible.
        for _ in 0..3 * NECK_PHASE_TICKS {
            neck.curriculum.tick();
        }
        let (frame0, _) = neck.tick(0);
        let (_, prev_brightness) = region_perception(&frame0, &[0.0; VIS_REGIONS]);
        let (frame1, ec) = neck.tick(33_000);
        let (axes1, _) = region_perception(&frame1, &prev_brightness);

        // Only check motion if there was actually a translation this tick.
        if ec.translation_term().is_some() {
            let max_motion = (0..VIS_REGIONS)
                .map(|r| axes1[r * 5 + VIS_SUB_MOTION])
                .fold(0.0_f64, f64::max);
            assert!(
                max_motion > 0.0,
                "expected non-zero motion after translation, got {max_motion}"
            );
        }
    }
}
