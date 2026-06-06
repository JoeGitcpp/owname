//! Phase 1 (keystone ①): sparse anisotropic Gaussian visual field.
//!
//! A small pool of 2-D Gaussians that learns, online and with NO backprop, to
//! reconstruct a low-resolution view of the camera frame. Each Gaussian is a
//! "colored sand grain": a position μ, an (axis-aligned) spread σ, an RGB color,
//! and an opacity. The field is updated each frame by a local, closed-form
//! predictive-coding rule:
//!
//!   1. splat the current Gaussians into a reconstruction grid,
//!   2. residual = target − reconstruction,
//!   3. each Gaussian nudges its own params from the residual inside its OWN
//!      footprint (a Widrow-Hoff step in parameter space — the analytic gradient
//!      of the squared residual w.r.t. each param is a local quantity; no global
//!      chain rule, no autograd),
//!   4. birth a new Gaussian where residual is high and unexplained; kill any
//!      whose opacity decays below a floor.
//!
//! Compliance with the highest principle:
//!   * No backprop / no autograd — only local closed-form parameter deltas.
//!   * No semantic priors — μ/σ/color/opacity are physical quantities; locality,
//!     translation invariance, multi-scale, minimum-size are physical priors only.
//!   * One tier-0 — the field is a DERIVED layer (like the object-slot layer); it
//!     does not introduce a parallel truth. (Viewport field + axis mirroring +
//!     adapter wiring + body integration test land in the next increment.)
//!
//! Design decision (Phase 1): this layer AUGMENTS, it does not yet replace
//! `region_perception`. The 20 coarse VIS axes remain the raw "retina readout"
//! (AXIS_COUNT unchanged); the Gaussian field is the rich model that depth (②)
//! and the capability governor's encoder probe (⑤) will be re-founded on.
//!
//! Increment 1 (this file): core splat + color/opacity/μ/σ local updates +
//! birth/death + reconstruction-error metric, with unit tests.
//! PENDING (increment 2, tracked in the 2026-06-04 remediation plan): explicit
//! top-k sparsity, lateral inhibition, slow-feature objective, full-covariance
//! rotation, Viewport field + adapter wiring + body integration test.

use crate::phot_camera::{CameraFrame, CAM_H, CAM_W};

/// Reconstruction grid (low-res; far cheaper than the 320×240 frame). The field
/// learns to reconstruct the frame downsampled to this size.
pub const GAUSS_REC_W: usize = 320;
pub const GAUSS_REC_H: usize = 240;
pub const GAUSS_PIX: usize = GAUSS_REC_W * GAUSS_REC_H;

/// Pool capacity. A hard upper bound on simultaneously-active Gaussians — the
/// sparsity bound (a physical "few primitives explain the scene" prior).
pub const GAUSS_CAP: usize = 512;

// --- Engineering knobs (not theoretical constants) ---
// These are DAMPING factors on a normalized (Gauss-Newton) local step
// Δθ = lr·Σ(r·s)/Σ(s²); <1 keeps the step a fraction of the Newton step.
const LR_COLOR: f64 = 0.30;
const LR_OPACITY: f64 = 0.30;
const LR_MU: f64 = 0.30;
const LR_SIGMA: f64 = 0.30;

const SIGMA_INIT: f64 = 0.12; // normalized [0,1] std at birth
const SIGMA_MIN: f64 = 0.02; // physical prior: minimum primitive size
const SIGMA_MAX: f64 = 0.6;
const OPACITY_INIT: f64 = 0.4;
const OPACITY_DEATH: f64 = 0.02; // opacity below this → deactivate
const BIRTH_RESIDUAL_MIN: f64 = 0.08; // spawn only where peak residual exceeds this
const BIRTH_MIN_SEP: f64 = 0.10; // don't spawn within this (normalized) of an active center
const FOOTPRINT_SIGMAS: f64 = 3.0;

// --- Phase 2: per-Gaussian inverse-depth (ρ) via the SAME local parallax rule
// the region depth observer used, now re-founded per primitive. ---
/// Inverse-depth bounds (ρ = 1/z, higher = closer). ρ is recovered only up to a
/// GLOBAL SCALE (monocular scale ambiguity) — only the near/far RATIO is meaningful.
pub const DEPTH_RHO_MIN: f64 = 0.01;
pub const DEPTH_RHO_MAX: f64 = 100.0;
/// Learning rate for the local depth-from-parallax step.
pub const DEPTH_LR: f64 = 0.1;

#[inline]
fn idx(c: usize, px: usize, py: usize) -> usize {
    c * GAUSS_PIX + py * GAUSS_REC_W + px
}

/// One visual tier-0 primitive: a colored, anisotropic (axis-aligned) Gaussian.
#[derive(Clone, Copy, Debug)]
pub struct Gaussian {
    /// Center in normalized image coords [0,1].
    pub mu_x: f64,
    pub mu_y: f64,
    /// Axis-aligned standard deviations in normalized units (rotation/full Σ is
    /// increment 2).
    pub sx: f64,
    pub sy: f64,
    /// RGB color in [0,1].
    pub color: [f64; 3],
    /// Opacity (presence) in [0,1].
    pub opacity: f64,
    pub active: bool,
    /// Inverse depth ρ = 1/z (higher = closer), recovered up to a global scale
    /// by the local parallax rule under self-motion. Neutral prior 1.0.
    pub inv_depth: f64,
    /// Whether depth was observable on the most-recent depth tick (efference
    /// translation present). False under stillness / pure rotation.
    pub depth_observable: bool,
    /// μx at the previous depth tick — lets each primitive measure its OWN
    /// image-space displacement Δμx (the parallax observation).
    pub prev_mu_x: f64,
}

impl Default for Gaussian {
    fn default() -> Self {
        Self {
            mu_x: 0.5,
            mu_y: 0.5,
            sx: SIGMA_INIT,
            sy: SIGMA_INIT,
            color: [0.0; 3],
            opacity: 0.0,
            active: false,
            inv_depth: 1.0,
            depth_observable: false,
            prev_mu_x: 0.5,
        }
    }
}

/// The sparse Gaussian field plus its last reconstruction error.
#[derive(Clone, Debug)]
pub struct GaussianField {
    pub gaussians: Vec<Gaussian>, // length GAUSS_CAP; `active` marks live
    /// Mean squared reconstruction residual on the most recent tick (the
    /// predictive-coding error BEFORE this tick's correction). Falls as the
    /// field learns a stable scene. This is the signal ⑤'s encoder probe reads.
    pub recon_mse: f64,
    pub n_updates: u64,
}

impl Default for GaussianField {
    fn default() -> Self {
        Self {
            gaussians: vec![Gaussian::default(); GAUSS_CAP],
            recon_mse: 0.0,
            n_updates: 0,
        }
    }
}

impl GaussianField {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn active_count(&self) -> usize {
        self.gaussians.iter().filter(|g| g.active).count()
    }

    /// Box-downsample a full camera frame into a target grid (3·GAUSS_PIX, planar
    /// RGB). Returns values in [0,1].
    pub fn downsample(frame: &CameraFrame) -> Vec<f64> {
        let mut target = vec![0.0_f64; 3 * GAUSS_PIX];
        for ry in 0..GAUSS_REC_H {
            let sy0 = ry * CAM_H / GAUSS_REC_H;
            let sy1 = ((ry + 1) * CAM_H / GAUSS_REC_H).max(sy0 + 1).min(CAM_H);
            for rx in 0..GAUSS_REC_W {
                let sx0 = rx * CAM_W / GAUSS_REC_W;
                let sx1 = ((rx + 1) * CAM_W / GAUSS_REC_W).max(sx0 + 1).min(CAM_W);
                let (mut ar, mut ag, mut ab, mut n) = (0.0, 0.0, 0.0, 0usize);
                for sy in sy0..sy1 {
                    for sx in sx0..sx1 {
                        let i = sy * CAM_W + sx;
                        ar += frame.r[i];
                        ag += frame.g[i];
                        ab += frame.b[i];
                        n += 1;
                    }
                }
                let inv = 1.0 / n.max(1) as f64;
                target[idx(0, rx, ry)] = (ar * inv).clamp(0.0, 1.0);
                target[idx(1, rx, ry)] = (ag * inv).clamp(0.0, 1.0);
                target[idx(2, rx, ry)] = (ab * inv).clamp(0.0, 1.0);
            }
        }
        target
    }

    /// Additive RGB splat: reconstruction[c,p] = Σ_k opacity_k · K_k(p) · color_k[c].
    /// Returned grid is NOT clamped (the raw model output, for residual math).
    /// Rows are processed in parallel via Rayon; each row is independent.
    pub fn splat(&self) -> Vec<f64> {
        use rayon::prelude::*;
        let active: Vec<&Gaussian> = self.gaussians.iter().filter(|g| g.active).collect();
        if active.is_empty() {
            return vec![0.0f64; 3 * GAUSS_PIX];
        }

        // Process rows in parallel; each row is independent
        let rows: Vec<Vec<f64>> = (0..GAUSS_REC_H).into_par_iter().map(|py| {
            let yn = (py as f64 + 0.5) / GAUSS_REC_H as f64;
            let mut row_r = vec![0.0f64; GAUSS_REC_W];
            let mut row_g = vec![0.0f64; GAUSS_REC_W];
            let mut row_b = vec![0.0f64; GAUSS_REC_W];
            for g in &active {
                // Skip row if outside Gaussian footprint
                let y_lo = g.mu_y - 3.0 * g.sy;
                let y_hi = g.mu_y + 3.0 * g.sy;
                if yn < y_lo || yn > y_hi { continue; }
                let dy = yn - g.mu_y;
                let ky = (-0.5 * dy * dy / (g.sy * g.sy)).exp();
                let x_lo = ((g.mu_x - 3.0 * g.sx) * GAUSS_REC_W as f64) as usize;
                let x_hi = (((g.mu_x + 3.0 * g.sx) * GAUSS_REC_W as f64) as usize + 1).min(GAUSS_REC_W);
                for px in x_lo..x_hi {
                    let xn = (px as f64 + 0.5) / GAUSS_REC_W as f64;
                    let dx = xn - g.mu_x;
                    let kx = (-0.5 * dx * dx / (g.sx * g.sx)).exp();
                    let w = g.opacity * ky * kx;
                    row_r[px] += w * g.color[0];
                    row_g[px] += w * g.color[1];
                    row_b[px] += w * g.color[2];
                }
            }
            let mut combined = Vec::with_capacity(3 * GAUSS_REC_W);
            combined.extend_from_slice(&row_r);
            combined.extend_from_slice(&row_g);
            combined.extend_from_slice(&row_b);
            combined
        }).collect();

        // Assemble into planar format [R plane][G plane][B plane]
        let mut recon = vec![0.0f64; 3 * GAUSS_PIX];
        for (py, row) in rows.iter().enumerate() {
            let base = py * GAUSS_REC_W;
            recon[base..base + GAUSS_REC_W].copy_from_slice(&row[0..GAUSS_REC_W]);
            recon[GAUSS_PIX + base..GAUSS_PIX + base + GAUSS_REC_W].copy_from_slice(&row[GAUSS_REC_W..2*GAUSS_REC_W]);
            recon[2*GAUSS_PIX + base..2*GAUSS_PIX + base + GAUSS_REC_W].copy_from_slice(&row[2*GAUSS_REC_W..3*GAUSS_REC_W]);
        }
        recon
    }

    /// One predictive-coding tick against a pre-downsampled target grid
    /// (3·GAUSS_PIX planar RGB). Returns the reconstruction MSE (pre-update).
    pub fn tick_grid(&mut self, target: &[f64]) -> f64 {
        debug_assert_eq!(target.len(), 3 * GAUSS_PIX);
        let recon = self.splat();
        // Residual + pre-update MSE.
        let mut residual = vec![0.0_f64; 3 * GAUSS_PIX];
        let mut sse = 0.0;
        for i in 0..residual.len() {
            let r = target[i] - recon[i];
            residual[i] = r;
            sse += r * r;
        }
        let mse = sse / residual.len() as f64;
        self.recon_mse = mse;

        // Local closed-form parameter updates (no backprop).
        use rayon::prelude::*;
        let updated: Vec<Gaussian> = self.gaussians.par_iter()
            .map(|g| if g.active { local_update(*g, &residual) } else { *g })
            .collect();
        self.gaussians = updated;

        self.birth_death(target, &residual);
        self.n_updates += 1;
        mse
    }

    /// Convenience: downsample a camera frame and tick.
    pub fn tick_frame(&mut self, frame: &CameraFrame) -> f64 {
        let target = Self::downsample(frame);
        self.tick_grid(&target)
    }

    /// One per-Gaussian depth-from-parallax tick. For each active Gaussian the
    /// observed displacement is its OWN Δμx since the last depth tick; the
    /// expected displacement under self-motion is tt·ρ. The local rule
    ///   ρ ← ρ + lr·(|Δμx| − tt·ρ)·tt
    /// has fixed point ρ* = |Δμx|/tt, so a NEAR primitive (more parallax for the
    /// same tt) settles at a LARGER ρ than a FAR one — depth ordering, up to a
    /// global scale. `tt = None` (still / pure rotation) → unobservable, ρ is
    /// NOT updated (world-motion must never masquerade as self-motion).
    /// `prev_mu_x` is always advanced so the next tick measures a fresh delta.
    ///
    /// Call AFTER `tick_grid`/`tick_frame` has re-fit μ to this frame.
    pub fn update_depth(&mut self, tt: Option<f64>) {
        for g in self.gaussians.iter_mut() {
            if !g.active {
                continue;
            }
            match tt {
                Some(tt) if tt.abs() >= 1e-6 => {
                    let d_obs = (g.mu_x - g.prev_mu_x).abs();
                    g.inv_depth = depth_step(g.inv_depth, d_obs, tt.abs());
                    g.depth_observable = true;
                }
                _ => {
                    g.depth_observable = false;
                }
            }
            g.prev_mu_x = g.mu_x;
        }
    }

    /// Birth a Gaussian at the peak unexplained residual; kill faded ones.
    fn birth_death(&mut self, target: &[f64], residual: &[f64]) {
        // Death: deactivate any Gaussian whose presence has decayed away.
        for g in self.gaussians.iter_mut() {
            if g.active && g.opacity < OPACITY_DEATH {
                g.active = false;
                g.opacity = 0.0;
            }
        }
        // Birth: find the pixel with the largest residual magnitude (summed over
        // channels) and, if it is unexplained enough and a slot is free, seed a
        // Gaussian there colored like the target.
        let mut best_mag = BIRTH_RESIDUAL_MIN;
        let mut best_p: Option<usize> = None;
        for p in 0..GAUSS_PIX {
            let mag = residual[p].abs() + residual[GAUSS_PIX + p].abs() + residual[2 * GAUSS_PIX + p].abs();
            if mag > best_mag {
                best_mag = mag;
                best_p = Some(p);
            }
        }
        let Some(p) = best_p else { return };
        let px = p % GAUSS_REC_W;
        let py = p / GAUSS_REC_W;
        let bx = (px as f64 + 0.5) / GAUSS_REC_W as f64;
        let by = (py as f64 + 0.5) / GAUSS_REC_H as f64;
        // Spatial non-max suppression: do not spawn where an active Gaussian
        // already sits — prevents redundant additive stacking (a minimal
        // competition prior; full lateral inhibition is increment 2).
        if self.gaussians.iter().any(|g| {
            g.active && ((g.mu_x - bx).powi(2) + (g.mu_y - by).powi(2)) < BIRTH_MIN_SEP * BIRTH_MIN_SEP
        }) {
            return;
        }
        let Some(slot) = self.gaussians.iter().position(|g| !g.active) else {
            return; // pool full → sparsity bound holds, no birth
        };
        self.gaussians[slot] = Gaussian {
            mu_x: bx,
            mu_y: by,
            sx: SIGMA_INIT,
            sy: SIGMA_INIT,
            color: [
                target[idx(0, px, py)],
                target[idx(1, px, py)],
                target[idx(2, px, py)],
            ],
            opacity: OPACITY_INIT,
            active: true,
            inv_depth: 1.0,
            depth_observable: false,
            prev_mu_x: bx,
        };
    }
}

/// Pixel-coordinate footprint [x0,x1) × [y0,y1) covering ±FOOTPRINT_SIGMAS·σ.
fn footprint(g: &Gaussian) -> (usize, usize, usize, usize) {
    let rx = FOOTPRINT_SIGMAS * g.sx;
    let ry = FOOTPRINT_SIGMAS * g.sy;
    let x0 = (((g.mu_x - rx) * GAUSS_REC_W as f64).floor()).clamp(0.0, GAUSS_REC_W as f64) as usize;
    let x1 = (((g.mu_x + rx) * GAUSS_REC_W as f64).ceil()).clamp(0.0, GAUSS_REC_W as f64) as usize;
    let y0 = (((g.mu_y - ry) * GAUSS_REC_H as f64).floor()).clamp(0.0, GAUSS_REC_H as f64) as usize;
    let y1 = (((g.mu_y + ry) * GAUSS_REC_H as f64).ceil()).clamp(0.0, GAUSS_REC_H as f64) as usize;
    (x0, x1.max(x0), y0, y1.max(y0))
}

/// The local depth-from-parallax step, shared by the per-Gaussian update and
/// tests. ρ ← ρ + lr·(d_obs − tt·ρ)·tt, clamped to [DEPTH_RHO_MIN, DEPTH_RHO_MAX].
/// Fixed point ρ* = d_obs/tt. Pure-local, no backprop.
#[inline]
fn depth_step(rho: f64, d_obs: f64, tt: f64) -> f64 {
    (rho + DEPTH_LR * (d_obs - tt * rho) * tt).clamp(DEPTH_RHO_MIN, DEPTH_RHO_MAX)
}

/// Local NORMALIZED parameter step for one Gaussian against the residual field.
/// For each parameter θ with per-pixel sensitivity s = ∂recon/∂θ, the step is the
/// damped Gauss-Newton / normalized-LMS update Δθ = lr·Σ(r·s)/(Σ s² + ε), summed
/// over the Gaussian's OWN footprint. Normalizing by Σs² makes the step self-
/// scaling (independent of footprint size) — without it, the raw gradient sum
/// grows with footprint area and the Gaussian overshoots and collapses. Still
/// fully local: no global chain rule, no autograd.
fn local_update(mut g: Gaussian, residual: &[f64]) -> Gaussian {
    const EPS: f64 = 1e-6;
    let (x0, x1, y0, y1) = footprint(&g);
    let (sx2, sy2) = (g.sx * g.sx, g.sy * g.sy);
    let mut col_num = [0.0_f64; 3];
    let mut col_den = [0.0_f64; 3];
    let (mut op_n, mut op_d) = (0.0, 0.0);
    let (mut mx_n, mut mx_d) = (0.0, 0.0);
    let (mut my_n, mut my_d) = (0.0, 0.0);
    let (mut sx_n, mut sx_d) = (0.0, 0.0);
    let (mut sy_n, mut sy_d) = (0.0, 0.0);
    for py in y0..y1 {
        let yn = (py as f64 + 0.5) / GAUSS_REC_H as f64;
        let dy = yn - g.mu_y;
        for px in x0..x1 {
            let xn = (px as f64 + 0.5) / GAUSS_REC_W as f64;
            let dx = xn - g.mu_x;
            let k = (-0.5 * (dx * dx / sx2 + dy * dy / sy2)).exp();
            let w = g.opacity * k;
            for c in 0..3 {
                let r = residual[idx(c, px, py)];
                let cc = g.color[c];
                let s_col = w; // ∂recon_c/∂color_c
                col_num[c] += r * s_col;
                col_den[c] += s_col * s_col;
                let s_op = k * cc; // ∂recon_c/∂opacity
                op_n += r * s_op;
                op_d += s_op * s_op;
                let s_mx = w * cc * (dx / sx2); // ∂recon_c/∂μx
                mx_n += r * s_mx;
                mx_d += s_mx * s_mx;
                let s_my = w * cc * (dy / sy2);
                my_n += r * s_my;
                my_d += s_my * s_my;
                let s_sx = w * cc * (dx * dx / (sx2 * g.sx)); // ∂recon_c/∂σx
                sx_n += r * s_sx;
                sx_d += s_sx * s_sx;
                let s_sy = w * cc * (dy * dy / (sy2 * g.sy));
                sy_n += r * s_sy;
                sy_d += s_sy * s_sy;
            }
        }
    }
    for c in 0..3 {
        g.color[c] = (g.color[c] + LR_COLOR * col_num[c] / (col_den[c] + EPS)).clamp(0.0, 1.0);
    }
    g.opacity = (g.opacity + LR_OPACITY * op_n / (op_d + EPS)).clamp(0.0, 1.0);
    g.mu_x = (g.mu_x + LR_MU * mx_n / (mx_d + EPS)).clamp(0.0, 1.0);
    g.mu_y = (g.mu_y + LR_MU * my_n / (my_d + EPS)).clamp(0.0, 1.0);
    g.sx = (g.sx + LR_SIGMA * sx_n / (sx_d + EPS)).clamp(SIGMA_MIN, SIGMA_MAX);
    g.sy = (g.sy + LR_SIGMA * sy_n / (sy_d + EPS)).clamp(SIGMA_MIN, SIGMA_MAX);
    g
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a target grid with a single soft blob of `color` centered at
    /// normalized (cx,cy) with normalized radius `rad`, on a black background.
    fn blob_target(cx: f64, cy: f64, rad: f64, color: [f64; 3]) -> Vec<f64> {
        let mut t = vec![0.0_f64; 3 * GAUSS_PIX];
        for py in 0..GAUSS_REC_H {
            let yn = (py as f64 + 0.5) / GAUSS_REC_H as f64;
            for px in 0..GAUSS_REC_W {
                let xn = (px as f64 + 0.5) / GAUSS_REC_W as f64;
                let d2 = (xn - cx).powi(2) + (yn - cy).powi(2);
                let v = (-0.5 * d2 / (rad * rad)).exp();
                for c in 0..3 {
                    t[idx(c, px, py)] = (v * color[c]).clamp(0.0, 1.0);
                }
            }
        }
        t
    }

    fn mse_against(target: &[f64], field: &GaussianField) -> f64 {
        let recon = field.splat();
        let mut sse = 0.0;
        for i in 0..target.len() {
            let r = target[i] - recon[i];
            sse += r * r;
        }
        sse / target.len() as f64
    }

    #[test]
    fn single_blob_reconstruction_error_falls() {
        let target = blob_target(0.5, 0.5, 0.15, [0.9, 0.1, 0.1]);
        let mut field = GaussianField::new();
        let first = field.tick_grid(&target);
        for _ in 0..120 {
            field.tick_grid(&target);
        }
        let last = mse_against(&target, &field);
        assert!(field.active_count() >= 1, "a Gaussian must have been born");
        assert!(
            last < 0.1 * first,
            "a single blob must be reconstructed nearly exactly: first={first} last={last}"
        );
    }

    #[test]
    fn trained_field_beats_empty_field_baseline() {
        let target = blob_target(0.35, 0.6, 0.12, [0.2, 0.8, 0.3]);
        let empty = GaussianField::new();
        let baseline = mse_against(&target, &empty); // no Gaussians = predict all-zero
        let mut field = GaussianField::new();
        for _ in 0..150 {
            field.tick_grid(&target);
        }
        let trained = mse_against(&target, &field);
        assert!(
            trained < 0.5 * baseline,
            "trained field must beat the empty (copy-nothing) baseline: baseline={baseline} trained={trained}"
        );
    }

    #[test]
    fn sparsity_bound_is_never_exceeded() {
        // A noisy, busy target tempts many births; the pool cap must hold.
        let mut t = vec![0.0_f64; 3 * GAUSS_PIX];
        for (i, v) in t.iter_mut().enumerate() {
            // deterministic pseudo-pattern, no RNG
            *v = (((i * 2654435761) >> 13) & 0xff) as f64 / 255.0;
        }
        let mut field = GaussianField::new();
        for _ in 0..200 {
            field.tick_grid(&t);
            assert!(
                field.active_count() <= GAUSS_CAP,
                "active Gaussians must never exceed GAUSS_CAP"
            );
        }
    }

    #[test]
    fn faded_gaussian_dies() {
        let mut field = GaussianField::new();
        field.gaussians[0] = Gaussian {
            opacity: OPACITY_DEATH * 0.5,
            active: true,
            ..Default::default()
        };
        // A black target gives no positive residual to revive it; a tick must reap it.
        let black = vec![0.0_f64; 3 * GAUSS_PIX];
        field.tick_grid(&black);
        assert!(!field.gaussians[0].active, "faded Gaussian must deactivate");
    }

    #[test]
    fn empty_field_on_black_is_zero_error() {
        let black = vec![0.0_f64; 3 * GAUSS_PIX];
        let mut field = GaussianField::new();
        let mse = field.tick_grid(&black);
        assert_eq!(mse, 0.0, "no Gaussians + black target = exact");
        assert_eq!(field.active_count(), 0, "black target births nothing");
    }

    // ── Phase 2: per-Gaussian depth-from-parallax ──────────────────────────────

    #[test]
    fn depth_step_converges_to_disp_over_tt() {
        // Fixed point of the parallax rule is ρ* = d_obs / tt.
        let (tt, d_obs) = (0.5_f64, 0.2_f64);
        let mut rho = 1.0_f64;
        for _ in 0..2000 {
            rho = depth_step(rho, d_obs, tt);
        }
        let expected = d_obs / tt; // 0.4
        assert!(
            (rho - expected).abs() < 1e-3,
            "ρ should converge to d_obs/tt={expected}, got {rho}"
        );
    }

    #[test]
    fn near_gaussian_gets_larger_rho_than_far() {
        // Same self-motion tt; the near primitive shows more parallax (larger
        // Δμx) so its ρ must settle higher than the far one's.
        let mut field = GaussianField::new();
        field.gaussians[0] = Gaussian { active: true, ..Default::default() };
        field.gaussians[1] = Gaussian { active: true, ..Default::default() };
        let tt = 0.5_f64;
        let (disp_near, disp_far) = (0.30_f64, 0.10_f64);
        for _ in 0..2000 {
            // Re-impose each primitive's own constant per-tick displacement.
            field.gaussians[0].prev_mu_x = 0.5;
            field.gaussians[0].mu_x = 0.5 + disp_near;
            field.gaussians[1].prev_mu_x = 0.5;
            field.gaussians[1].mu_x = 0.5 + disp_far;
            field.update_depth(Some(tt));
        }
        let rho_near = field.gaussians[0].inv_depth;
        let rho_far = field.gaussians[1].inv_depth;
        assert!(
            field.gaussians[0].depth_observable && field.gaussians[1].depth_observable,
            "both must be observable under self-motion"
        );
        assert!(rho_near > rho_far, "near ρ={rho_near} must exceed far ρ={rho_far}");
        assert!((rho_near - disp_near / tt).abs() < 1e-2, "near ρ={rho_near} ≈ {}", disp_near / tt);
        assert!((rho_far - disp_far / tt).abs() < 1e-2, "far ρ={rho_far} ≈ {}", disp_far / tt);
    }

    #[test]
    fn still_camera_leaves_depth_unobservable_and_rho_fixed() {
        let mut field = GaussianField::new();
        field.gaussians[0] = Gaussian {
            active: true,
            inv_depth: 0.7,
            mu_x: 0.6,
            prev_mu_x: 0.4, // a would-be displacement, but no self-motion
            ..Default::default()
        };
        let before = field.gaussians[0].inv_depth;
        field.update_depth(None); // still / pure rotation → unobservable
        assert_eq!(
            field.gaussians[0].inv_depth, before,
            "ρ must not change when depth is unobservable"
        );
        assert!(
            !field.gaussians[0].depth_observable,
            "must be marked unobservable when tt = None"
        );
    }
}
