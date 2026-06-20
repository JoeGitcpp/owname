//! B1 (VPC mechanism 5): cross-layer credit assignment — high-layer half.
//!
//! A `CrossLayerPredictor` learns to predict ONE target axis's low-layer error
//! from a SPARSE set of source axes. When it predicts that error well, it
//! writes a prior back to the viewport's prior channel so the low-layer
//! predictor can use it (frozen doc 机制5: "prior 写回 → 低层利用高阶信息 →
//! 间接降低 err").
//!
//! Compliance (no BP / no global loss / no cross-predictor direct comm):
//!   - It reads source-axis state and the target's error ONLY via the shared
//!     viewport (err_stream / observations). No direct wires to other predictors.
//!   - It updates ONLY its own `src_weights`, from ONLY its own local error
//!     (predicted target-err vs actual target-err). No chain rule, no回传.
//!   - It writes ONLY to the prior channel — never to any low-layer a/b.
//!   - Sources are a SPARSE explicit set, not a global all-axis matrix. Sources
//!     that don't correlate with the target's error decay toward zero weight
//!     (noise rejection emerges for free).
//!
//! Phase 3 cutover note: the per-slot SlotCrossLayer stack + recursive tier
//! residual tower that used to live here have been removed — `perturbation_graph`
//! relations now carry the object-granularity cross-layer credit. The axis-level
//! predictor below survives because it explains scalar axes (including
//! INTERO_PRESENCE) that have no slot equivalent.

use std::sync::Arc;

use crate::viewport::Viewport;

/// One source axis and its learned weight in the error-prediction.
#[derive(Clone, Copy, Debug)]
struct SrcWeight {
    axis: usize,
    w: f64,
}

pub struct CrossLayerPredictor {
    /// Axis whose low-layer error this predictor tries to explain.
    target_axis: usize,
    /// Sparse candidate sources, each with a learned weight.
    sources: Vec<SrcWeight>,
    /// Learned bias / intercept. Without it, sources are forced to fake any
    /// constant offset in the target error, which spuriously inflates an
    /// otherwise-uncorrelated source's weight (breaks noise rejection).
    bias: f64,
    /// Local learning rate for the weight update (Widrow-Hoff / LMS).
    base_lr: f64,
    /// Clamp on any single weight (keeps it bounded, like the AR-2 clip).
    clip: f64,
    /// EMA of squared error-prediction error (how well it predicts target err).
    err_pred_mse: f64,
    /// Previous-tick mse, for the learning-progress (d(mse)/dt) signal.
    prev_mse: f64,
    /// EMA of the DOWNWARD movement of err_pred_mse = learning progress.
    /// High when the predictor is actively getting better at explaining the
    /// target error; ≈0 when it has plateaued (mastered OR pure noise). This is
    /// the "blank/空白" drive signal.
    learning_progress: f64,
    /// EMA of the target error's own magnitude² (its variance proxy). The prior
    /// gate compares our residual mse against this: if we explain most of the
    /// target error (mse ≪ var) we are confident; if we explain none of it
    /// (mse ≈ var, i.e. pure noise) we gate the prior off. This makes noise
    /// rejection hold even after the predictor has stopped improving.
    target_err_var: f64,
    /// Strength of the prior written back, scaled by prediction confidence.
    prior_gain: f64,
    /// Warmup counter; no prior is written until it has seen enough.
    n_updates: u32,
}

const WARMUP_MIN: u32 = 8;

/// A2: readable / persistable snapshot of one axis-level cross-layer predictor
/// (pure numbers — no strings, no labels). Mirrors the learned fields so the
/// bank survives a restart instead of re-warming from zero (prior 短暂归 0 会
/// 破坏自洽连续性).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct AxisCrossLayerState {
    /// Axis whose low-layer error this predictor explains.
    pub target_axis: usize,
    /// Sparse explicit source axes, parallel to `source_weights`.
    pub source_axes: Vec<usize>,
    /// Learned weight per source axis.
    pub source_weights: Vec<f64>,
    /// Learned bias / intercept.
    pub bias: f64,
    /// EMA of squared error-prediction error.
    pub err_pred_mse: f64,
    /// Previous-tick mse (learning-progress bookkeeping).
    pub prev_mse: f64,
    /// EMA of downward mse movement.
    pub learning_progress: f64,
    /// EMA of the target error's own magnitude² (variance proxy).
    pub target_err_var: f64,
    /// Total ticks run (warmup gate state).
    pub n_updates: u32,
}

/// A2: the body-resident axis-level cross-layer bank — the hard-coded SPARSE
/// prior table for VPC mechanism 5's prior write-back leg. Every (target,
/// sources) pair is explicit; each target reads at most a handful of same- or
/// related-modality axes, NEVER an all-axis fan-in (that would be a disguised
/// global coupling / BP). Coverage (intentionally restrained for the first
/// wiring):
///   - hearing: each VIB band's error predicted from its adjacent band(s);
///   - vision: per region, brightness error from the region's R/G/B, and
///     temporal-flux error from the region's motion/edge-energy;
///   - interoception: three substrate-coupled groups — and one DELIBERATE
///     cross-modal source (VIS_LOOMING_AXIS → INTERO_VITAL_ANXIETY) that lets the
///     bank learn "approach predicts dread" on top of the innate reflex.
pub fn axis_predictor_bank() -> Vec<CrossLayerPredictor> {
    let mut bank = Vec::new();

    // Hearing: adjacent frequency bands (same-modality neighbours).
    for k in 0..crate::VIB_BANDS {
        let target = crate::AXIS_VIB + k;
        let mut sources = Vec::with_capacity(2);
        if k > 0 {
            sources.push(crate::AXIS_VIB + k - 1);
        }
        if k + 1 < crate::VIB_BANDS {
            sources.push(crate::AXIS_VIB + k + 1);
        }
        bank.push(CrossLayerPredictor::new(target, &sources));
    }

    // Vision: within-region physical quantities (sparse, never cross-all).
    for region in 0..crate::VIS_REGIONS {
        bank.push(CrossLayerPredictor::new(
            crate::vis_axis(region, crate::vis_sub::BRIGHTNESS),
            &[
                crate::vis_axis(region, crate::vis_sub::R),
                crate::vis_axis(region, crate::vis_sub::G),
                crate::vis_axis(region, crate::vis_sub::B),
            ],
        ));
        bank.push(CrossLayerPredictor::new(
            crate::vis_axis(region, crate::vis_sub::TEMPORAL_FLUX),
            &[
                crate::vis_axis(region, crate::vis_sub::MOTION),
                crate::vis_axis(region, crate::vis_sub::EDGE_ENERGY),
            ],
        ));
    }

    // Interoception: a few substrate-coupled scalar pairs.
    // Vital anxiety (dread) is predicted from the body's own metabolism
    // (reserve, pain) AND — the prepared-learning leg of the perception→defense
    // link — from the born-in looming/approach detector. If an approaching
    // stimulus reliably co-occurs with rising dread (it does: tick_looming fires
    // the innate reflex), VIS_LOOMING_AXIS's weight grows and a prior is written
    // back; if not, LMS noise-rejection decays it to ~0. This is how "approach
    // predicts danger" is LEARNED on top of the unconditioned reflex.
    bank.push(CrossLayerPredictor::new(
        crate::INTERO_VITAL_ANXIETY,
        &[crate::INTERO_RESERVE, crate::INTERO_PAIN, crate::VIS_LOOMING_AXIS],
    ));
    bank.push(CrossLayerPredictor::new(
        crate::INTERO_GAP_PRESSURE,
        &[crate::INTERO_CLOSURE, crate::INTERO_LEARNING_PROGRESS],
    ));
    bank.push(CrossLayerPredictor::new(
        crate::INTERO_PRESENCE,
        &[crate::INTERO_ATTACHMENT, crate::INTERO_CLOSURE],
    ));

    bank
}

impl CrossLayerPredictor {
    pub fn new(target_axis: usize, source_axes: &[usize]) -> Self {
        let sources = source_axes
            .iter()
            .map(|&axis| SrcWeight { axis, w: 0.0 })
            .collect();
        Self {
            target_axis,
            sources,
            bias: 0.0,
            base_lr: 0.5,
            clip: 10.0,
            err_pred_mse: 0.0,
            prev_mse: 0.0,
            learning_progress: 0.0,
            target_err_var: 0.0,
            prior_gain: 1.0,
            n_updates: 0,
        }
    }

    /// EMA of squared error-prediction error (observer / test view).
    pub fn err_pred_mse(&self) -> f64 {
        self.err_pred_mse
    }

    /// Learning progress = EMA of the downward movement of err_pred_mse.
    /// >0 while actively learning the target's error structure; ≈0 on a plateau
    /// (mastered or pure noise). Gates the prior (noise rejection).
    pub fn learning_progress(&self) -> f64 {
        self.learning_progress
    }

    /// A2: pure-numeric snapshot of the learned state (observer / persistence).
    pub fn to_state(&self) -> AxisCrossLayerState {
        AxisCrossLayerState {
            target_axis: self.target_axis,
            source_axes: self.sources.iter().map(|s| s.axis).collect(),
            source_weights: self.sources.iter().map(|s| s.w).collect(),
            bias: self.bias,
            err_pred_mse: self.err_pred_mse,
            prev_mse: self.prev_mse,
            learning_progress: self.learning_progress,
            target_err_var: self.target_err_var,
            n_updates: self.n_updates,
        }
    }

    /// A2: rebuild a predictor from a persisted state. Returns None if the
    /// state refers to axes outside the current layout (e.g. a snapshot from
    /// an older axis map) or carries non-finite learned values — the caller
    /// then falls back to a fresh predictor instead of importing corruption.
    pub fn from_state(state: &AxisCrossLayerState) -> Option<Self> {
        if state.target_axis >= crate::AXIS_COUNT {
            return None;
        }
        if state.source_axes.len() != state.source_weights.len() {
            return None;
        }
        if state.source_axes.iter().any(|&a| a >= crate::AXIS_COUNT) {
            return None;
        }
        let finite = state.bias.is_finite()
            && state.err_pred_mse.is_finite()
            && state.prev_mse.is_finite()
            && state.learning_progress.is_finite()
            && state.target_err_var.is_finite()
            && state.source_weights.iter().all(|w| w.is_finite());
        if !finite {
            return None;
        }
        let mut p = Self::new(state.target_axis, &state.source_axes);
        let clip = p.clip;
        for (s, &w) in p.sources.iter_mut().zip(state.source_weights.iter()) {
            s.w = w.clamp(-clip, clip);
        }
        p.bias = state.bias.clamp(-clip, clip);
        p.err_pred_mse = state.err_pred_mse;
        p.prev_mse = state.prev_mse;
        p.learning_progress = state.learning_progress;
        p.target_err_var = state.target_err_var;
        p.n_updates = state.n_updates;
        Some(p)
    }

    /// Learned weight for a given source axis (0 if not a source).
    pub fn weight_for(&self, axis: usize) -> f64 {
        self.sources
            .iter()
            .find(|s| s.axis == axis)
            .map(|s| s.w)
            .unwrap_or(0.0)
    }

    /// One cross-layer tick. Reads source state + target error from the
    /// viewport, updates its own weights locally, and (after warmup) writes a
    /// prior for the target axis.
    pub fn tick(&mut self, vp: &Arc<Viewport>) {
        // Read the target's actual low-layer error (what we are explaining).
        let actual_err = match vp.latest_err(self.target_axis) {
            Some(e) => e,
            None => return, // no error published yet → nothing to learn from
        };

        // Read source-axis state (latest observation) and form the prediction
        // of the target error from the SPARSE source set only.
        let mut src_vals = Vec::with_capacity(self.sources.len());
        let mut pred_err = self.bias;
        for s in &self.sources {
            let x = vp.latest_observation(s.axis).map(|o| o.value).unwrap_or(0.0);
            pred_err += s.w * x;
            src_vals.push(x);
        }

        // Local error of THIS predictor: how wrong our error-prediction was.
        let meta_err = actual_err - pred_err;

        // Local Widrow-Hoff update of our own weights + bias (no chain rule, no
        // 回传). The bias absorbs any constant offset so source weights track
        // genuine correlation only.
        self.bias = (self.bias + self.base_lr * meta_err).clamp(-self.clip, self.clip);
        for (s, &x) in self.sources.iter_mut().zip(src_vals.iter()) {
            let dw = self.base_lr * meta_err * x;
            s.w = (s.w + dw).clamp(-self.clip, self.clip);
        }

        // Track how well we predict the target error (EMA of meta_err²) and the
        // target error's own variance proxy (EMA of actual_err²).
        self.prev_mse = self.err_pred_mse;
        self.err_pred_mse = 0.98 * self.err_pred_mse + 0.02 * meta_err * meta_err;
        self.target_err_var = 0.98 * self.target_err_var + 0.02 * actual_err * actual_err;

        // Learning progress = EMA of the DOWNWARD movement of mse (空白 being
        // filled). Negative moves (mse rising) don't count as progress.
        let down = (self.prev_mse - self.err_pred_mse).max(0.0);
        self.learning_progress = 0.9 * self.learning_progress + 0.1 * down;
        self.n_updates = self.n_updates.saturating_add(1);

        // Prior gate = explained fraction = how much of the target error's
        // variance we actually explain. ≈1 for a learnable (now-mastered)
        // target; ≈0 for pure noise (residual mse ≈ the error's own variance).
        // This is the noise-rejection gate: a noise target gets gate→0 → no
        // prior, not just a small weight. Fully local; never touches low layer.
        let explained = if self.target_err_var > 1e-12 {
            (1.0 - self.err_pred_mse / self.target_err_var).clamp(0.0, 1.0)
        } else {
            0.0
        };

        // After warmup, write a prior — scaled by the gate. Recompute the
        // prediction with the freshly updated weights.
        if self.n_updates >= WARMUP_MIN {
            let mut prior = self.bias;
            for (s, &x) in self.sources.iter().zip(src_vals.iter()) {
                prior += s.w * x;
            }
            // A3 developmental precision schedule: the write-back gain is more
            // tolerant when naive (m=0 → coarse early priors help the low
            // layer off its 0×x=0 dead point) and anneals to exactly the
            // existing prior_gain·explained gate at m=1. Scales only this
            // predictor's local scalar; push_prior still clamps the magnitude.
            let m = vp.developmental_maturity();
            vp.push_prior(
                self.target_axis,
                crate::vpc_prior_gain_scale(m) * self.prior_gain * explained * prior,
            );
        }
    }
}
