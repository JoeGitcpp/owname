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

use std::sync::Arc;

use crate::viewport::Viewport;

/// Minimum relation confidence for a neighbour slot to be gated in as a source.
/// Below this, the relation is too weak to be a credible predictor of the
/// target's error (and noise/isolated objects never reach it).
const RELATION_GATE_CONFIDENCE: f64 = 0.15;

/// B1-3a: relation-gated source selection for slot→slot cross-layer prediction.
///
/// Returns the SPARSE set of source slots for a target slot: exactly those
/// slots that have an ACTIVE relation edge INTO the target with sufficient
/// confidence. This is the prime-directive guard at slot granularity: a target
/// reads only its relation neighbours, NEVER all slots (an all-slots fan-in
/// would be a disguised global-coupling / BP). A noise or isolated object grows
/// no active edge → it is never returned here (relation-gated noise rejection).
///
/// Pure read over the shared viewport's relation graph; mutates nothing.
pub fn gated_sources(vp: &Viewport, target_slot: usize) -> Vec<usize> {
    let relations = vp.read_global_relations();
    let mut srcs = Vec::new();
    for edge in relations.edges.iter() {
        if !edge.active {
            continue;
        }
        // Edge points src → dst; we want predictors of the TARGET, i.e. edges
        // whose dst is the target. Its src is a candidate source.
        if edge.dst_slot_id as usize != target_slot {
            continue;
        }
        if edge.relation_confidence < RELATION_GATE_CONFIDENCE {
            continue;
        }
        let src = edge.src_slot_id as usize;
        if src == target_slot {
            continue; // never self
        }
        if !srcs.contains(&src) {
            srcs.push(src);
        }
    }
    srcs
}

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
///   - interoception: three pairs whose dynamics are substrate-coupled.
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
    bank.push(CrossLayerPredictor::new(
        crate::INTERO_VITAL_ANXIETY,
        &[crate::INTERO_RESERVE, crate::INTERO_PAIN],
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

// ---------------------------------------------------------------------------
// B1-3b: per-slot cross-layer predictor (object→object granularity)
// ---------------------------------------------------------------------------

/// Readable snapshot of a slot's cross-layer predictor (observer / persistence).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlotCrossLayerState {
    /// Relation-gated source slots used this tick (sparse — never all slots).
    pub source_slots: Vec<usize>,
    /// Learned weight per source slot, parallel to `source_slots`.
    pub source_weights: Vec<f64>,
    /// Learned bias / intercept.
    pub bias: f64,
    /// EMA of squared error-prediction error.
    pub err_pred_mse: f64,
    /// Learning progress (downward movement of mse).
    pub learning_progress: f64,
    /// Total ticks run.
    pub n_updates: u32,
    /// This tick's residual (= meta_err = the part of the target error this unit
    /// could NOT explain). This is the signal a HIGHER tier tries to predict in
    /// turn — the recursive-depth substrate. Not persisted (recomputed at run).
    pub residual: f64,
    /// Explained fraction = 1 - err_pred_mse / target_err_var, in [0,1]. ≈1 when
    /// this unit explains the target's structure (learnable), ≈0 when the target
    /// is pure noise. This is the noise discriminator (B1-2) and the per-tier
    /// learnability gate that decides whether a higher tier is worth growing.
    pub explained_fraction: f64,
}

/// A cross-layer predictor for ONE target slot. It predicts the target slot's
/// `object_err` from the `object_err` of its RELATION-GATED neighbour slots.
///
/// Compliance (object granularity): sources are chosen by `gated_sources` (the
/// relation graph) — a SPARSE neighbour set, never all slots (no全连接/disguised
/// BP). It updates only its own weights from its own local error. It never
/// writes another slot's low-layer predictor coefficients.
#[derive(Clone, Debug, Default)]
pub struct SlotCrossLayerPredictor {
    target_slot: usize,
    source_slots: Vec<usize>,
    source_weights: Vec<f64>,
    bias: f64,
    base_lr: f64,
    clip: f64,
    err_pred_mse: f64,
    prev_mse: f64,
    learning_progress: f64,
    n_updates: u32,
    /// This tick's residual (meta_err) — exposed so a higher tier can predict it.
    residual: f64,
    /// EMA of the target error's own magnitude² (variance proxy). Used with
    /// err_pred_mse to derive explained_fraction (the noise / learnability gate).
    target_err_var: f64,
    /// Stage 2 (downward prior chain): a top-down prior written by the tier
    /// ABOVE this one. It enters this unit's prediction as a pure ADDITIVE INPUT
    /// (never into the weight update — mirrors the axis-level `external_prior`).
    /// Set ONLY via `set_prior_from_above` by the sweep; the tier above never
    /// touches this unit's weights/bias (the §2.2b red line: "可递 prior 当输入,
    /// 绝不伸手改 a/b"). It is one-sweep STALE by construction (the sweep is
    /// bottom-up, so the tier above writes it AFTER this unit ticked) → a slow
    /// contextual bias, not a within-step gradient (R3).
    prior_from_above: f64,
    /// This tick's prediction value (bias + Σw·x + prior_from_above). Cached so
    /// the tier can emit it DOWNWARD (gated by its own explained_fraction) as the
    /// below tier's prior_from_above — its estimate of the below tier's residual.
    last_pred: f64,
}

impl SlotCrossLayerPredictor {
    pub fn new(target_slot: usize) -> Self {
        Self {
            target_slot,
            source_slots: Vec::new(),
            source_weights: Vec::new(),
            bias: 0.0,
            base_lr: 0.3,
            clip: 10.0,
            ..Default::default()
        }
    }

    pub fn state(&self) -> SlotCrossLayerState {
        SlotCrossLayerState {
            source_slots: self.source_slots.clone(),
            source_weights: self.source_weights.clone(),
            bias: self.bias,
            err_pred_mse: self.err_pred_mse,
            learning_progress: self.learning_progress,
            n_updates: self.n_updates,
            residual: self.residual,
            explained_fraction: self.explained_fraction(),
        }
    }

    /// Explained fraction = 1 - mse/var, clamped to [0,1]. ≈1 = explains the
    /// target's structure (learnable); ≈0 = target is pure noise. The per-tier
    /// learnability / noise gate (mirrors the axis-level CrossLayerPredictor).
    pub fn explained_fraction(&self) -> f64 {
        if self.target_err_var > 1e-12 {
            (1.0 - self.err_pred_mse / self.target_err_var).clamp(0.0, 1.0)
        } else {
            0.0
        }
    }

    /// This tick's residual = the part of the target error this unit could not
    /// explain. A higher tier predicts THIS in turn (recursive depth).
    pub fn residual(&self) -> f64 {
        self.residual
    }

    pub fn n_updates(&self) -> u32 {
        self.n_updates
    }

    /// The top-down prior currently held (Stage 2). Observer / test view.
    pub fn prior_from_above(&self) -> f64 {
        self.prior_from_above
    }

    /// Stage 2: set the top-down prior from the tier ABOVE. This is the ONLY way
    /// a higher tier influences this one — and it touches NOTHING but this field
    /// (never weights/bias). That is the §2.2b red line made structural: a prior
    /// is an input, not a coefficient edit.
    pub fn set_prior_from_above(&mut self, prior: f64) {
        self.prior_from_above = prior;
    }

    /// Stage 2: the prior this tier emits DOWNWARD = its prediction of the below
    /// tier's residual, GATED by its OWN explained_fraction (R2: the gain depends
    /// only on this tier's own learnability, never on any lower/bottom error).
    /// Clamped for safety. ≈0 until this tier actually explains its target, so a
    /// non-learning (noise) tier sends no downward prior.
    pub fn prior_down(&self) -> f64 {
        (self.explained_fraction() * self.last_pred).clamp(-self.clip, self.clip)
    }

    /// One local learning step (PURE — no viewport). Given the actual target
    /// error and the CURRENT source values (parallel to `source_weights`), do the
    /// Widrow-Hoff update of THIS unit's own bias + weights, and track its own
    /// mse / variance / residual / learning-progress.
    ///
    /// Compliance (the no-BP heart): uses ONLY its own input × its own error to
    /// update ONLY its own weights. No chain rule, no other unit's coefficients,
    /// no global loss. Identical math whether the inputs are layer-0 object_err
    /// (tier 1) or a lower tier's residual (tier ≥2) — the recursion is uniform.
    ///
    /// Stage 2: the top-down `prior_from_above` enters the PREDICTION as a pure
    /// additive input (mirrors the axis-level `external_prior` in predictor.rs).
    /// It is NOT a parameter and no gradient flows back to the tier that wrote
    /// it — so this stays local Widrow-Hoff, never chain-rule BP.
    pub fn observe(&mut self, actual_err: f64, source_errs: &[f64]) {
        debug_assert_eq!(source_errs.len(), self.source_weights.len());
        let mut pred_err = self.bias + self.prior_from_above;
        for (w, &x) in self.source_weights.iter().zip(source_errs.iter()) {
            pred_err += w * x;
        }
        self.last_pred = pred_err;
        let meta_err = actual_err - pred_err;
        self.bias = (self.bias + self.base_lr * meta_err).clamp(-self.clip, self.clip);
        for (w, &x) in self.source_weights.iter_mut().zip(source_errs.iter()) {
            *w = (*w + self.base_lr * meta_err * x).clamp(-self.clip, self.clip);
        }
        self.prev_mse = self.err_pred_mse;
        self.err_pred_mse = 0.98 * self.err_pred_mse + 0.02 * meta_err * meta_err;
        let down = (self.prev_mse - self.err_pred_mse).max(0.0);
        self.learning_progress = 0.9 * self.learning_progress + 0.1 * down;
        self.target_err_var = 0.98 * self.target_err_var + 0.02 * actual_err * actual_err;
        self.residual = meta_err;
        self.n_updates = self.n_updates.saturating_add(1);
    }

    /// Refresh the gated source set from the relation graph, preserving learned
    /// weights for sources that persist and dropping those whose edge vanished.
    fn refresh_sources(&mut self, gated: Vec<usize>) {
        let mut new_weights = Vec::with_capacity(gated.len());
        for &s in &gated {
            let w = self
                .source_slots
                .iter()
                .position(|&old| old == s)
                .map(|i| self.source_weights[i])
                .unwrap_or(0.0);
            new_weights.push(w);
        }
        self.source_slots = gated;
        self.source_weights = new_weights;
    }
}

// ---------------------------------------------------------------------------
// Stage 1 (recursive PC depth): residual tower growth gate + upward sweep.
// READ-UP ONLY — no tier here ever writes a prior downward (Stage 2 territory).
// ---------------------------------------------------------------------------

/// Warmup ticks before a tier's explained_fraction is trusted for the gate.
const TIER_PROMOTE_MIN_UPDATES: u32 = 30;
/// A tier counts as a PERMANENT explainer (worth growing a tier above it) once
/// it explains at least this fraction of its target's variance. Below this, the
/// tier is not doing real explanatory work → no higher tier is born.
const TIER_PROMOTE_FRACTION: f64 = 0.5;
/// Runaway backstop ONLY (like the agent-count cap): the real depth limit is the
/// emergent noise gate (a tier that can't explain its target never promotes, so
/// nothing grows above it). This number is far above any plausible real depth;
/// if a tower ever hits it, that is a bug to investigate, not a designed ceiling.
pub const TOWER_DEPTH_CAP: usize = 16;

/// Pure growth gate: is the tier below a PERMANENT explainer, so a tier above it
/// is worth growing? Depends ONLY on that tier's OWN warmup + own explained
/// fraction (its own local learnability) — never on any lower layer's error.
/// This is the R4 invariant in pure form: depth growth reads only the
/// immediately-below tier's local state.
///
/// Note: `explained_fraction` is already 0 when the target had ~no variance, so
/// a constant / dead tier auto-fails here without a separate variance floor.
pub fn tier_is_permanent(n_updates: u32, explained_fraction: f64) -> bool {
    n_updates >= TIER_PROMOTE_MIN_UPDATES && explained_fraction >= TIER_PROMOTE_FRACTION
}

/// Cadence: tier `level` ticks once per 2^(level-2) sweeps, so higher tiers run
/// on a SLOWER timescale (timescale emergence; Stage-2 R3 groundwork). Pure and
/// MONOTONE — if a level is not due this sweep, no higher (slower) level is due
/// either, so the sweep can stop ascending the moment it hits a not-due level.
pub fn level_is_due(sweep_count: u64, level: usize) -> bool {
    debug_assert!(level >= 2);
    let period: u64 = 1u64 << (level - 2);
    sweep_count % period == 0
}

impl Viewport {
    /// B1-3b: drive the per-slot cross-layer predictor for `target_slot`.
    /// Reads the target's and its relation-neighbours' `object_err` via the
    /// shared viewport, learns locally, and stores updated state on the slot.
    pub fn tick_slot_cross_layer(&self, target_slot: usize, _now_us: u64) {
        if target_slot >= crate::N_OBJECT_SLOTS {
            return;
        }
        // Read the target's current object_err (what we explain). Requires a
        // high-layer state to exist on the target slot.
        let actual_err = match self.read_slot(target_slot).and_then(|s| s.high_layer) {
            Some(hl) => hl.object_err,
            None => return,
        };

        // Relation-gated sparse source set (this is the prime-directive guard).
        let gated = gated_sources(self, target_slot);

        // Read each source slot's object_err.
        let mut src_errs = Vec::with_capacity(gated.len());
        for &s in &gated {
            let e = self
                .read_slot(s)
                .and_then(|sl| sl.high_layer)
                .map(|hl| hl.object_err)
                .unwrap_or(0.0);
            src_errs.push(e);
        }

        // Take the predictor out, update it, put it back (slot-local state).
        let mut pred = self
            .take_slot_cross_layer(target_slot)
            .unwrap_or_else(|| SlotCrossLayerPredictor::new(target_slot));
        pred.refresh_sources(gated);
        // Local Widrow-Hoff update (pure; identical math to before — now shared
        // with the higher residual tiers via `observe`).
        pred.observe(actual_err, &src_errs);

        self.put_slot_cross_layer(target_slot, pred);
    }

    pub fn read_slot_cross_layer(&self, target_slot: usize) -> Option<SlotCrossLayerState> {
        if target_slot >= crate::N_OBJECT_SLOTS {
            return None;
        }
        self.slots[target_slot]
            .read()
            .unwrap()
            .xlayer
            .as_ref()
            .map(|p| p.state())
    }

    fn take_slot_cross_layer(&self, target_slot: usize) -> Option<SlotCrossLayerPredictor> {
        self.slots[target_slot].write().unwrap().xlayer.take()
    }

    fn put_slot_cross_layer(&self, target_slot: usize, pred: SlotCrossLayerPredictor) {
        self.slots[target_slot].write().unwrap().xlayer = Some(pred);
    }

    /// Read a tier's residual under a read lock. tier `level`: 1 = `xlayer`,
    /// ≥2 = `xstack[level-2]`. Returns 0.0 when the tier is absent.
    fn tier_residual_of(&self, slot: usize, level: usize) -> f64 {
        let g = self.slots[slot].read().unwrap();
        if level == 1 {
            g.xlayer.as_ref().map(|p| p.residual()).unwrap_or(0.0)
        } else {
            g.xstack.get(level - 2).map(|p| p.residual()).unwrap_or(0.0)
        }
    }

    /// Read a tier's gate inputs (n_updates, explained_fraction) under a read
    /// lock. None when the tier is absent.
    fn tier_gate_of(&self, slot: usize, level: usize) -> Option<(u32, f64)> {
        let g = self.slots[slot].read().unwrap();
        let p = if level == 1 {
            g.xlayer.as_ref()
        } else {
            g.xstack.get(level - 2)
        }?;
        Some((p.n_updates(), p.explained_fraction()))
    }

    /// Number of upper tiers (≥2) currently present on a slot. Observer-facing.
    /// Total depth = 1 (tier `xlayer`) + this, when `xlayer` is present.
    pub fn read_slot_tower_depth(&self, slot: usize) -> usize {
        if slot >= crate::N_OBJECT_SLOTS {
            return 0;
        }
        self.slots[slot].read().unwrap().xstack.len()
    }

    /// Read one upper tier's state (level ≥2). Observer / persistence.
    pub fn read_slot_tier(&self, slot: usize, level: usize) -> Option<SlotCrossLayerState> {
        if slot >= crate::N_OBJECT_SLOTS || level < 2 {
            return None;
        }
        self.slots[slot]
            .read()
            .unwrap()
            .xstack
            .get(level - 2)
            .map(|p| p.state())
    }

    /// Read the whole upper tower (tiers ≥2) of a slot, bottom-up. Observer /
    /// persistence. Empty when no tier above tier 1 has been born.
    pub fn read_slot_tower(&self, slot: usize) -> Vec<SlotCrossLayerState> {
        if slot >= crate::N_OBJECT_SLOTS {
            return Vec::new();
        }
        self.slots[slot]
            .read()
            .unwrap()
            .xstack
            .iter()
            .map(|p| p.state())
            .collect()
    }

    /// Upward sweep: grow + tick the residual tower across all slots, LEVEL BY
    /// LEVEL (so every tier's residual is fresh before the tier above reads it).
    /// A tier at `level` predicts the tier-below residual from relation-gated
    /// neighbours' SAME tier-below residual, learns locally, and exposes its own
    /// residual. A new tier is born above a tier ONLY when that tier is a
    /// permanent explainer (`tier_is_permanent`) — emergent, self-limiting depth.
    ///
    /// Stage 2 (downward prior chain): after a tier observes, it writes its
    /// gated prediction DOWN to the tier below's `prior_from_above` (the ONLY
    /// downward influence — never a coefficient edit). Because the sweep is
    /// bottom-up, the below tier already ticked this sweep, so it consumes that
    /// prior NEXT cycle: a slow contextual bias, not a within-step gradient (R3).
    ///
    /// Must run AFTER `tick_slot_cross_layer` for all slots (tier 1 ready).
    pub fn tick_cross_layer_tower_sweep(&self, _now_us: u64) {
        let count = self
            .tower_sweep_count
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut level = 2usize;
        while level <= TOWER_DEPTH_CAP {
            // Cadence: higher tiers run slower. Not due → no higher tier is due
            // either (monotone), so stop ascending.
            if !level_is_due(count, level) {
                break;
            }
            let mut ticked_any = false;
            for slot in 0..crate::N_OBJECT_SLOTS {
                if !self.slots[slot].read().unwrap().active {
                    continue;
                }
                // Gate: is the tier BELOW (level-1) a permanent explainer? (R4:
                // reads only the immediately-below tier's own local state.)
                let below = match self.tier_gate_of(slot, level - 1) {
                    Some(g) => g,
                    None => continue, // no tier below → nothing to predict
                };
                let below_permanent = tier_is_permanent(below.0, below.1);
                let has_unit = self.slots[slot].read().unwrap().xstack.len() >= level - 1;
                if !has_unit && !below_permanent {
                    continue; // not yet earned a tier here
                }

                // Gather inputs (read locks released before the write lock).
                let target = self.tier_residual_of(slot, level - 1);
                let gated = gated_sources(self, slot);
                let neighbour_res: Vec<f64> = gated
                    .iter()
                    .map(|&n| self.tier_residual_of(n, level - 1))
                    .collect();

                // Update (or birth) this slot's tier at `level`, then emit its
                // gated prediction DOWN to the tier below. observe() touches only
                // this unit's own weights; the downward write touches only the
                // below tier's prior_from_above field (never its weights/bias).
                {
                    let mut g = self.slots[slot].write().unwrap();
                    if g.xstack.len() < level - 1 {
                        g.xstack.push(SlotCrossLayerPredictor::new(slot)); // birth
                    }
                    let prior_down = {
                        let unit = &mut g.xstack[level - 2];
                        unit.refresh_sources(gated);
                        unit.observe(target, &neighbour_res);
                        unit.prior_down() // R2: gated by this unit's OWN explained
                    };
                    // Stage 2 downward chain: write prior to the tier BELOW. This
                    // is the WHOLE of the top-down influence — a prior input only,
                    // never a coefficient edit (§2.2b red line, structural).
                    if level == 2 {
                        if let Some(t1) = g.xlayer.as_mut() {
                            t1.set_prior_from_above(prior_down);
                        }
                    } else if let Some(below) = g.xstack.get_mut(level - 3) {
                        below.set_prior_from_above(prior_down);
                    }
                }
                ticked_any = true;
            }
            if !ticked_any {
                break; // no slot has structure at this level → tower stops
            }
            level += 1;
        }
    }
}

#[cfg(test)]
mod observe_tests {
    use super::*;

    /// Build a unit with one source wired in (bypassing the relation graph, so
    /// the PURE local-learning step can be tested without a viewport).
    fn unit_with_one_source() -> SlotCrossLayerPredictor {
        let mut p = SlotCrossLayerPredictor::new(0);
        p.source_slots = vec![1];
        p.source_weights = vec![0.0];
        p
    }

    /// A LEARNABLE target (actual_err = 2 × source) → the unit explains it:
    /// explained_fraction → high, residual → small. This is "the layer below
    /// left structure a higher tier could capture".
    #[test]
    fn observe_learns_a_linear_target() {
        let mut p = unit_with_one_source();
        // Deterministic ±1 source so the target has genuine variance.
        for i in 0..2000u64 {
            let src = if i % 2 == 0 { 1.0 } else { -1.0 };
            let actual = 2.0 * src;
            p.observe(actual, &[src]);
        }
        assert!(
            p.explained_fraction() > 0.8,
            "learnable target should be well-explained, got {}",
            p.explained_fraction()
        );
        assert!(
            p.residual().abs() < 0.2,
            "learnable residual should be small, got {}",
            p.residual()
        );
        assert!(
            (p.source_weights[0] - 2.0).abs() < 0.2,
            "weight should converge to the true gain 2.0, got {}",
            p.source_weights[0]
        );
    }

    /// A PURE-NOISE target (orthogonal to the source over its cycle) → the unit
    /// cannot explain it: explained_fraction stays ≈0. This is the per-tier noise
    /// gate that makes recursive depth SELF-LIMIT (no higher tier grows here).
    #[test]
    fn observe_cannot_explain_noise() {
        let mut p = unit_with_one_source();
        // src cycle [1,-1,1,-1], target cycle [1,1,-1,-1] → zero correlation.
        for i in 0..2000u64 {
            let src = if i % 2 == 0 { 1.0 } else { -1.0 };
            let actual = if i % 4 < 2 { 1.0 } else { -1.0 };
            p.observe(actual, &[src]);
        }
        assert!(
            p.explained_fraction() < 0.4,
            "noise target must stay poorly-explained (noise gate), got {}",
            p.explained_fraction()
        );
    }

    /// observe touches ONLY this unit's own weights/bias from its own error — the
    /// no-BP heart. A second, independent unit fed the SAME inputs is unaffected
    /// by the first (no cross-unit coupling, no shared state).
    #[test]
    fn observe_is_local_only() {
        let mut a = unit_with_one_source();
        let mut b = unit_with_one_source();
        for i in 0..50u64 {
            let src = if i % 2 == 0 { 1.0 } else { -1.0 };
            a.observe(2.0 * src, &[src]);
        }
        // b never observed → still pristine; a's learning did not leak into b.
        assert_eq!(b.source_weights[0], 0.0);
        assert_eq!(b.bias, 0.0);
        assert_eq!(b.n_updates(), 0);
        assert!(a.n_updates() == 50);
    }
}

#[cfg(test)]
mod tower_tests {
    use super::*;
    use crate::{MOD_VIB, VIB_BANDS};
    use std::sync::Arc;

    /// R4 in pure form: a tier above is born ONLY when the tier below is a
    /// permanent explainer — depends only on that tier's OWN warmup + explained.
    #[test]
    fn growth_gate_requires_permanent_below() {
        // not warmed up → never grows, however well it explains
        assert!(!tier_is_permanent(TIER_PROMOTE_MIN_UPDATES - 1, 0.99));
        // warmed up but explains nothing (noise) → never grows
        assert!(!tier_is_permanent(1000, 0.0));
        assert!(!tier_is_permanent(1000, TIER_PROMOTE_FRACTION - 0.01));
        // warmed up AND a real explainer → grow
        assert!(tier_is_permanent(TIER_PROMOTE_MIN_UPDATES, TIER_PROMOTE_FRACTION));
        assert!(tier_is_permanent(1000, 0.95));
    }

    /// Drive two related slots, then seed tier 1 to a permanent explainer state
    /// whose residuals are CROSS-SLOT CORRELATED (rA = 2·rB). The sweep should
    /// grow a tier 2 and that tier 2 should LEARN the cross-slot residual
    /// structure (explained_fraction rises) — depth emerging from real structure.
    fn warmup_two_related_slots() -> (Arc<Viewport>, usize, usize, u64) {
        let vp = Arc::new(Viewport::new());
        let t0 = 12_000_000u64;
        let dt = 10_000u64;
        let a = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc a");
        let b = vp.allocate_slot(MOD_VIB, t0, 0).expect("alloc b");
        let drive = |slot: usize, t: u64, lvl: f64| {
            let obs = [lvl; VIB_BANDS];
            let pred = [lvl * 0.9; VIB_BANDS];
            let err = [lvl * 0.1; VIB_BANDS];
            let prec = [0.5; VIB_BANDS];
            vp.write_slot_vib(slot, t, &obs, &pred, &err, &prec);
            vp.refresh_slot_fusion(slot, MOD_VIB, t);
            vp.refresh_slot_high_layer(slot, t);
        };
        for i in 0..60u64 {
            let t = t0 + i * dt;
            drive(a, t, 0.6);
            drive(b, t, 0.55);
            vp.refresh_global_attention(t);
            vp.refresh_global_relations(t);
            vp.refresh_global_relation_predictor(t);
            vp.tick_slot_cross_layer(a, t);
            vp.tick_slot_cross_layer(b, t);
        }
        (vp, a, b, t0 + 60 * dt)
    }

    /// Force a slot's tier 1 into a permanent-explainer state with a chosen
    /// residual (the synthetic "structured residual" the tower must capture).
    fn seed_tier1(vp: &Viewport, slot: usize, residual: f64) {
        let mut g = vp.slots[slot].write().unwrap();
        let p = g
            .xlayer
            .get_or_insert_with(|| SlotCrossLayerPredictor::new(slot));
        p.n_updates = 100;
        p.err_pred_mse = 0.05;
        p.target_err_var = 1.0; // explained_fraction = 0.95 → permanent
        p.residual = residual;
    }

    #[test]
    fn tower_grows_and_learns_cross_slot_residual() {
        let (vp, a, b, t) = warmup_two_related_slots();
        // precondition: an active relation edge feeds A (so tier 2 has a source).
        assert!(
            !gated_sources(&vp, a).is_empty(),
            "warmup must leave A with a relation-gated source"
        );

        // Seed correlated residuals (rA = 2·rB) and sweep repeatedly.
        for i in 0..600u64 {
            let rb = if i % 2 == 0 { 0.3 } else { -0.3 };
            seed_tier1(&vp, a, 2.0 * rb);
            seed_tier1(&vp, b, rb);
            vp.tick_cross_layer_tower_sweep(t + i);
        }

        assert!(
            vp.read_slot_tower_depth(a) >= 1,
            "a tier 2 should be born above a permanent tier 1, depth={}",
            vp.read_slot_tower_depth(a)
        );
        let tier2 = vp.read_slot_tier(a, 2).expect("tier 2 should exist");
        assert!(
            tier2.explained_fraction > 0.5,
            "tier 2 should LEARN the cross-slot residual structure, explained={}",
            tier2.explained_fraction
        );
    }

    #[test]
    fn higher_tiers_tick_on_a_slower_cadence() {
        // Pure schedule: level 2 every sweep, level 3 every 2nd, level 4 every 4th.
        assert!(level_is_due(0, 2) && level_is_due(1, 2) && level_is_due(3, 2));
        assert!(level_is_due(2, 3) && !level_is_due(1, 3) && !level_is_due(3, 3));
        assert!(level_is_due(4, 4) && !level_is_due(2, 4));
        // Monotone: a not-due level implies no higher (slower) level is due.
        for c in 0..32u64 {
            for l in 2..6usize {
                if !level_is_due(c, l) {
                    assert!(
                        !level_is_due(c, l + 1),
                        "monotone cadence violated at count {c} level {l}"
                    );
                }
            }
        }
        // End-to-end: in a growing tower, a higher tier accrues FEWER updates.
        let (vp, a, b, t) = warmup_two_related_slots();
        for i in 0..600u64 {
            let rb = if i % 2 == 0 { 0.3 } else { -0.3 };
            seed_tier1(&vp, a, 2.0 * rb);
            seed_tier1(&vp, b, rb);
            vp.tick_cross_layer_tower_sweep(t + i);
        }
        assert!(
            vp.read_slot_tower_depth(a) >= 2,
            "tower should reach depth 2 (tier 2 promoted, tier 3 probing), got {}",
            vp.read_slot_tower_depth(a)
        );
        let t2 = vp.read_slot_tier(a, 2).unwrap();
        let t3 = vp.read_slot_tier(a, 3).unwrap();
        assert!(
            t2.n_updates > t3.n_updates,
            "tier 3 must tick slower than tier 2: {} vs {}",
            t2.n_updates,
            t3.n_updates
        );
    }

    #[test]
    fn tower_self_limits_on_noise_residual() {
        let (vp, a, b, t) = warmup_two_related_slots();
        assert!(!gated_sources(&vp, a).is_empty());

        // Orthogonal (zero-correlation) residuals: rA cycle [1,-1,1,-1],
        // rB cycle [1,1,-1,-1] → tier 2 cannot explain rA from rB.
        for i in 0..600u64 {
            let ra = if i % 2 == 0 { 0.3 } else { -0.3 };
            let rb = if i % 4 < 2 { 0.3 } else { -0.3 };
            seed_tier1(&vp, a, ra);
            seed_tier1(&vp, b, rb);
            vp.tick_cross_layer_tower_sweep(t + i);
        }

        // One prober tier 2 is born, but it never promotes (noise) → the tower
        // does NOT keep growing. Self-limit via the same noise gate.
        assert!(
            vp.read_slot_tower_depth(a) <= 1,
            "noise residual must NOT grow an unbounded tower, depth={}",
            vp.read_slot_tower_depth(a)
        );
        if let Some(tier2) = vp.read_slot_tier(a, 2) {
            assert!(
                tier2.explained_fraction < 0.4,
                "tier 2 must fail to explain noise, explained={}",
                tier2.explained_fraction
            );
        }
    }

    // -----------------------------------------------------------------------
    // Stage 2 (downward prior chain) — the four red-line invariants, each with
    // a test that actively tries to VIOLATE it, plus no-BP and the mechanism.
    // -----------------------------------------------------------------------

    /// Force tier 1 into a warmed-up but NON-explaining (noise) state.
    fn seed_tier1_unlearnable(vp: &Viewport, slot: usize, residual: f64) {
        let mut g = vp.slots[slot].write().unwrap();
        let p = g
            .xlayer
            .get_or_insert_with(|| SlotCrossLayerPredictor::new(slot));
        p.n_updates = 100;
        p.err_pred_mse = 1.0;
        p.target_err_var = 1.0; // explained_fraction = 0 → NOT permanent
        p.residual = residual;
    }

    fn set_object_err(vp: &Viewport, slot: usize, v: f64) {
        let mut g = vp.slots[slot].write().unwrap();
        if let Some(hl) = g.high_layer.as_mut() {
            hl.object_err = v;
        }
    }

    /// R1: a tier's TARGET is the immediately-below tier's RESIDUAL — never the
    /// bottom (object_err). We make the residual learnable-from-neighbour but the
    /// object_err UNpredictable-from-neighbour; tier 2 ends up explaining well,
    /// which is only possible if it targeted the residual (not object_err).
    #[test]
    fn r1_tier_target_is_below_residual_not_bottom_error() {
        let (vp, a, b, t) = warmup_two_related_slots();
        assert!(!gated_sources(&vp, a).is_empty());
        for i in 0..600u64 {
            let s = if i % 2 == 0 { 0.3 } else { -0.3 }; // shared residual signal
            seed_tier1(&vp, a, s);
            seed_tier1(&vp, b, s); // B's residual == A's residual → predictable
                                   // object_err made UNpredictable-from-B (different cycle):
            set_object_err(&vp, a, if i % 3 == 0 { 0.9 } else { -0.9 });
            vp.tick_cross_layer_tower_sweep(t + i);
        }
        let tier2 = vp.read_slot_tier(a, 2).expect("tier 2 exists");
        assert!(
            tier2.explained_fraction > 0.5,
            "tier 2 explains the RESIDUAL (target=residual). If it had targeted \
             object_err (unpredictable from B) it could not explain it. explained={}",
            tier2.explained_fraction
        );
    }

    /// R2: the downward prior's GAIN is this tier's OWN explained_fraction only —
    /// nothing about any lower/bottom signal. Two units with identical prediction
    /// but different OWN mse emit priors scaled purely by their own explained.
    #[test]
    fn r2_prior_down_gain_is_only_own_explained() {
        let mut good = SlotCrossLayerPredictor::new(0);
        let mut bad = SlotCrossLayerPredictor::new(0);
        // Same cached prediction value...
        good.last_pred = 1.0;
        bad.last_pred = 1.0;
        // ...but good explains its own target (var≫mse), bad does not (var≈mse).
        good.err_pred_mse = 0.0;
        good.target_err_var = 1.0; // explained = 1.0
        bad.err_pred_mse = 1.0;
        bad.target_err_var = 1.0; // explained = 0.0
        assert!((good.prior_down() - 1.0).abs() < 1e-9, "explainer emits full prior");
        assert!(bad.prior_down().abs() < 1e-9, "non-explainer emits ~no prior");
        // And the gain ignores everything external: changing nothing but the
        // unit's OWN mse changes the emitted prior.
        let before = good.prior_down();
        good.err_pred_mse = 0.5; // explained drops to 0.5 → prior halves
        assert!((good.prior_down() - 0.5 * before).abs() < 1e-9);
    }

    /// R3: the prior a unit consumes is a PRE-SET bias (from a previous sweep),
    /// independent of THIS step's own error. observe() uses whatever
    /// prior_from_above was last set — a sudden actual_err spike does not
    /// retroactively change the prior that shaped the prediction.
    #[test]
    fn r3_prior_is_a_preset_bias_not_this_steps_error() {
        let mut p = SlotCrossLayerPredictor::new(0);
        p.set_prior_from_above(0.5);
        // Predict with a tiny actual: the prior (0.5) shapes the prediction.
        p.observe(0.0, &[]);
        let pred_small_actual = p.last_pred;
        // Reset and predict with a HUGE actual spike, same pre-set prior.
        let mut q = SlotCrossLayerPredictor::new(0);
        q.set_prior_from_above(0.5);
        q.observe(100.0, &[]);
        let pred_big_actual = q.last_pred;
        // The prediction (and thus the prior's contribution) is identical: the
        // prior is a pre-set contextual bias, NOT derived from this step's error.
        assert!((pred_small_actual - 0.5).abs() < 1e-9);
        assert!((pred_big_actual - 0.5).abs() < 1e-9);
        assert!((pred_small_actual - pred_big_actual).abs() < 1e-9);
    }

    /// R4: depth growth reads ONLY the immediately-below tier's own learnability,
    /// NEVER the bottom error magnitude. A tier 1 with a HUGE residual but which
    /// does NOT explain its target (explained=0) grows NO tier above it.
    #[test]
    fn r4_no_growth_when_below_does_not_explain_however_big_its_error() {
        let (vp, a, b, t) = warmup_two_related_slots();
        assert!(!gated_sources(&vp, a).is_empty());
        for i in 0..400u64 {
            // Big residual (big "error") but tier 1 is NOT a permanent explainer.
            seed_tier1_unlearnable(&vp, a, if i % 2 == 0 { 5.0 } else { -5.0 });
            seed_tier1_unlearnable(&vp, b, if i % 2 == 0 { 5.0 } else { -5.0 });
            vp.tick_cross_layer_tower_sweep(t + i);
        }
        assert_eq!(
            vp.read_slot_tower_depth(a),
            0,
            "no tier may grow above a non-explaining tier 1, however big its error"
        );
    }

    /// no-BP red line (§2.2b): the downward write touches ONLY the below tier's
    /// prior_from_above — NEVER its weights/bias. "可递 prior 当输入,绝不改 a/b."
    #[test]
    fn no_bp_downward_write_never_touches_below_weights() {
        let (vp, a, b, t) = warmup_two_related_slots();
        // Grow a tier 2 that will write a prior down to tier 1.
        for i in 0..400u64 {
            let rb = if i % 2 == 0 { 0.3 } else { -0.3 };
            seed_tier1(&vp, a, 2.0 * rb);
            seed_tier1(&vp, b, rb);
            vp.tick_cross_layer_tower_sweep(t + i);
        }
        assert!(vp.read_slot_tower_depth(a) >= 1, "need a tier 2 to write down");
        // Capture tier 1's coefficients, then do one more sweep (tier 2 writes
        // its prior DOWN to tier 1).
        let before = vp.read_slot_cross_layer(a).unwrap();
        vp.tick_cross_layer_tower_sweep(t + 1000);
        let after = vp.read_slot_cross_layer(a).unwrap();
        assert_eq!(
            before.source_weights, after.source_weights,
            "downward prior must NOT change tier 1 weights (no coefficient edit = no BP)"
        );
        assert_eq!(
            before.bias, after.bias,
            "downward prior must NOT change tier 1 bias (no coefficient edit = no BP)"
        );
    }

    /// Mechanism: an ACCURATE top-down prior reduces the prediction error — this
    /// is "大颗粒校正小颗粒" (top-down expectation improving the lower prediction).
    #[test]
    fn mechanism_accurate_prior_reduces_prediction_error() {
        // No sources, fresh bias=0, target = 1.0.
        let mut without = SlotCrossLayerPredictor::new(0);
        without.observe(1.0, &[]);
        let err_without = without.residual().abs(); // = 1.0 (pred 0)

        let mut with = SlotCrossLayerPredictor::new(0);
        with.set_prior_from_above(1.0); // accurate top-down estimate
        with.observe(1.0, &[]);
        let err_with = with.residual().abs(); // ≈ 0 (pred ≈ 1.0)

        assert!(
            err_with < err_without,
            "an accurate downward prior must lower the error: with={err_with} without={err_without}"
        );
        assert!(err_with < 1e-9, "perfectly accurate prior → ~zero error");
    }
}
