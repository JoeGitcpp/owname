//! vocal_servo — active acoustic servo (P3 Stage 3).
//!
//! Measurement showed: with perception fixed (AGC) and the learning gap driven by
//! real acoustic shape error, the body correctly PERCEIVES it is ~0.48 (shape)
//! away from the target — but it never gets CLOSER (shape flat over 20 min, 0
//! prototypes recorded). Because the motor drive only aims via the coarse
//! feedforward pitch/resonance hint; the acoustic error credited learning but did
//! not STEER the motor.
//!
//! This servo closes that gap: it hill-climbs a small correction (pitch_offset,
//! resonance_offset) added to the feedforward aim, to MINIMIZE the shape distance
//! the body actually hears. It's a (1+1) coordinate search with an explicit
//! baseline/probe alternation so each decision compares like-for-like:
//!   Baseline period: apply committed offsets, hear baseline_shape.
//!   Probe period:    apply offsets + a step in one of ±pitch/±resonance, hear it.
//!   If the probe beat the baseline → commit it (keep that direction); else rotate.
//! Bounded offsets, smoothed organ → no instability; wrong guesses just don't
//! commit. This is babbling-toward-a-heard-target — the substrate of imitation,
//! not a scripted behavior.

pub const STEP: f64 = 0.06; // probe magnitude per commit
pub const MAX_OFFSET: f64 = 0.45; // correction bound (feedforward still dominates)
pub const SERVO_INTERVAL_US: u64 = 90_000; // ~90 ms per period (organ smoothing settles)
const DIRS: [(f64, f64); 4] = [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)];
const IMPROVE_EPS: f64 = 1e-4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Baseline,
    Probe,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VocalServoState {
    pub pitch_offset: f64,
    pub resonance_offset: f64,
    dir_idx: u8,
    phase: Phase,
    baseline_shape: f64,
    /// Per-tick EMA of the (noisy) formant-shape error. The hill-climb compares
    /// baseline vs probe across a ~90 ms gap; feeding the raw single-tick error
    /// lets synth jitter/shimmer dominate the comparison and the search wanders.
    /// Smoothing the objective every tick gives a stable surface to descend.
    pub shape_ema: f64,
    pub last_update_t_us: u64,
}

impl Default for VocalServoState {
    fn default() -> Self {
        Self {
            pitch_offset: 0.0,
            resonance_offset: 0.0,
            dir_idx: 0,
            phase: Phase::Baseline,
            baseline_shape: 1.0,
            shape_ema: 1.0,
            last_update_t_us: 0,
        }
    }
}

impl VocalServoState {
    /// Per-tick: fold the latest heard formant error into the smoothed objective.
    pub fn observe_shape(&mut self, shape_dist: f64) {
        self.shape_ema = self.shape_ema * 0.85 + shape_dist * 0.15;
    }
}

#[inline]
fn clamp_off(x: f64) -> f64 {
    x.clamp(-MAX_OFFSET, MAX_OFFSET)
}

impl VocalServoState {
    /// Phase 2: warm-start the committed offsets from a recalled correction
    /// (content-addressed motor recall) and re-enter the Baseline phase so the
    /// next probe is evaluated against the recalled point, not a stale one. This
    /// is what lets a familiar target skip the cold hill-climb.
    pub fn warm_start(&mut self, off_a: f64, off_b: f64) {
        self.pitch_offset = clamp_off(off_a);
        self.resonance_offset = clamp_off(off_b);
        self.phase = Phase::Baseline;
        self.baseline_shape = 1.0;
    }

    /// Phase 2: clean cold start (novel target with no recall) — drop any
    /// committed offset chased from a previous, now-gone target.
    pub fn reset(&mut self) {
        self.pitch_offset = 0.0;
        self.resonance_offset = 0.0;
        self.dir_idx = 0;
        self.phase = Phase::Baseline;
        self.baseline_shape = 1.0;
    }

    /// The correction to ADD to the feedforward (pitch_hint, resonance_hint) this
    /// period. Includes the live probe step during the Probe phase.
    pub fn aim(&self) -> (f64, f64) {
        match self.phase {
            Phase::Baseline => (clamp_off(self.pitch_offset), clamp_off(self.resonance_offset)),
            Phase::Probe => {
                let (sp, sr) = DIRS[self.dir_idx as usize % 4];
                (
                    clamp_off(self.pitch_offset + sp * STEP),
                    clamp_off(self.resonance_offset + sr * STEP),
                )
            }
        }
    }

    /// Feed the shape distance just heard (result of the aim applied last period)
    /// and advance the baseline/probe search. Returns true if a probe was committed.
    pub fn step(&mut self, now_us: u64, heard_shape: f64) -> bool {
        self.last_update_t_us = now_us;
        match self.phase {
            Phase::Baseline => {
                self.baseline_shape = heard_shape;
                self.phase = Phase::Probe;
                false
            }
            Phase::Probe => {
                self.phase = Phase::Baseline;
                if heard_shape + IMPROVE_EPS < self.baseline_shape {
                    let (sp, sr) = DIRS[self.dir_idx as usize % 4];
                    self.pitch_offset = clamp_off(self.pitch_offset + sp * STEP);
                    self.resonance_offset = clamp_off(self.resonance_offset + sr * STEP);
                    true // keep dir_idx — ride the gradient
                } else {
                    self.dir_idx = (self.dir_idx + 1) % 4; // explore next direction
                    false
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Synthetic acoustic landscape: shape minimized at (po*, ro*); the servo only
    // sees shape(aim), never the optimum — it must find it by hill-climbing.
    fn run_to_optimum(opt_p: f64, opt_r: f64) -> (VocalServoState, f64) {
        let land = |po: f64, ro: f64| 0.10 + 0.40 * ((po - opt_p).abs() + (ro - opt_r).abs());
        let mut s = VocalServoState::default();
        let mut t = 0u64;
        for _ in 0..800 {
            let (ap, ar) = s.aim();
            let heard = land(ap, ar);
            s.step(t, heard);
            t += SERVO_INTERVAL_US;
        }
        let committed_shape = land(s.pitch_offset, s.resonance_offset);
        (s, committed_shape)
    }

    #[test]
    fn descends_toward_optimum() {
        let (s, shape) = run_to_optimum(0.2, -0.1);
        assert!(shape < 0.20, "servo failed to descend (got {shape}, floor 0.10)");
        assert!((s.pitch_offset - 0.2).abs() < 0.10, "pitch_offset off: {}", s.pitch_offset);
        assert!((s.resonance_offset + 0.1).abs() < 0.10, "resonance_offset off: {}", s.resonance_offset);
    }

    #[test]
    fn descends_for_other_optimum() {
        let (_s, shape) = run_to_optimum(-0.3, 0.25);
        assert!(shape < 0.20, "servo failed for second optimum (got {shape})");
    }

    #[test]
    fn offsets_stay_bounded() {
        // optimum far outside the reachable bound → offsets must saturate, not blow up
        let (s, _) = run_to_optimum(5.0, -5.0);
        assert!(s.pitch_offset.abs() <= MAX_OFFSET + 1e-9);
        assert!(s.resonance_offset.abs() <= MAX_OFFSET + 1e-9);
    }

    #[test]
    fn stable_at_optimum_no_runaway() {
        // start already at optimum (0,0): no probe should improve → offsets stay ~0
        let land = |po: f64, ro: f64| 0.10 + 0.40 * (po.abs() + ro.abs());
        let mut s = VocalServoState::default();
        let mut t = 0u64;
        for _ in 0..400 {
            let (ap, ar) = s.aim();
            s.step(t, land(ap, ar));
            t += SERVO_INTERVAL_US;
        }
        assert!(s.pitch_offset.abs() < STEP * 1.5, "drifted from optimum: {}", s.pitch_offset);
        assert!(s.resonance_offset.abs() < STEP * 1.5, "drifted: {}", s.resonance_offset);
    }

    #[test]
    fn aim_probe_differs_from_baseline() {
        let mut s = VocalServoState::default();
        s.step(0, 0.5); // Baseline → Probe
        let (ap, _ar) = s.aim();
        assert!((ap - 0.0).abs() > 1e-9, "probe aim should offset from committed 0");
    }
}
