//! C1-b (sim): the 生命仓 metabolic environment — a SIMULATED survival world
//! that drives the vital core's `supply`/`drain` so reserve/anxiety/pain/state
//! actually breathe and a real survival crisis can occur.
//!
//! Why this exists: in the live body, `supply` (sensory-input continuity) is
//! ~1.0 every tick (camera/mic always streaming) and `drain` is a tiny resting
//! metabolism, so the net is always positive and `reserve` is pinned at 1.0 for
//! the whole run. The death state machine (Standby/revive), `vital_anxiety`, and
//! the self-preservation half of the dual-drive soul never engage — survival is
//! never at stake. `vital_core.rs` always anticipated this: C1-a takes INJECTED
//! survival signals; C1-b feeds them. This module is the C1-b *simulator* (a
//! modelled metabolic world), as opposed to a real-battery sampler.
//!
//! Design constraints (matching the rest of the core):
//! - PURE & DETERMINISTIC: `supply`/`drain` are a closed-form function of the
//!   simulated clock — no RNG, fully reproducible and unit-testable.
//! - The vital core stays ignorant of the world: it still only consumes two
//!   scalars. This module never touches `VitalCoreState` internals.
//! - Numeric-only: a resource scalar and a metabolic scalar. No semantic
//!   identity words baked in.
//!
//! Layers (all expressed through the two output scalars):
//!   L1 circadian supply — the world's resource availability waxes and wanes on
//!      a compressed "day" so reserve breathes instead of sitting pinned.
//!   L2 metabolism — a resting drain meaningful relative to supply, so the
//!      reserve genuinely drifts (activity exertion is added by the caller).
//!   L3 scarcity famine — periodic deep-supply troughs that push reserve toward
//!      the floor: this is what exercises anxiety, Standby/revive, and the
//!      dual-drive self-preservation pressure.

use crate::substrate::unit;
use std::f64::consts::PI;

/// Fixed integration step for the vital core, decoupled from the orchestrator's
/// tick rate (~30 Hz). The caller advances the vital core at most once per this
/// interval, so survival dynamics depend on wall-clock time, not on how often
/// the loop happens to spin. (L0: the timescale fix.)
pub const VITAL_STEP_US: u64 = 1_000_000; // 1 Hz integration

/// A compressed survival world. All durations in microseconds of the simulated
/// (wall) clock. Defaults are tuned for an *observable* run: a full day-night
/// cycle every few minutes, with a famine every few days.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VitalEnv {
    /// Length of one simulated day (one full circadian supply cycle).
    pub day_us: u64,
    /// Supply at the deepest point of night (circadian trough), [0,1].
    pub supply_floor: f64,
    /// Circadian swing added on top of the floor at midday (peak = floor+amp).
    pub supply_amp: f64,
    /// Resting metabolic drain. Activity exertion is added by the caller on top.
    pub base_metabolism: f64,
    /// A famine occurs every Nth day (day 0 is born into plenty: no famine).
    pub famine_period_days: u64,
    /// Fraction of a famine day the famine lasts, starting at `famine_start`.
    pub famine_frac: f64,
    /// Phase within the day at which the famine begins, [0,1).
    pub famine_start: f64,
    /// Supply multiplier during a famine (small → near-total scarcity).
    pub famine_mult: f64,
}

impl Default for VitalEnv {
    fn default() -> Self {
        Self {
            day_us: 300_000_000, // 5 min / day → a full cycle is watchable
            supply_floor: 0.07,  // deep night: below metabolism → reserve falls
            supply_amp: 0.38,    // midday peak ≈ 0.45 → above metabolism → recovers
            base_metabolism: 0.20,
            famine_period_days: 3, // a famine every ~15 min
            famine_frac: 0.30,     // ~1/3 of the day under scarcity...
            famine_start: 0.10,    // ...ending at midday (0.40) so the body can
            famine_mult: 0.10,     // ride the rising supply back up and recover.
        }
    }
}

impl VitalEnv {
    /// Generous metabolic world for 7×24 nursery training: high baseline
    /// supply, low resting drain, no famine. Paired with reduced voicing
    /// exertion in `Viewport::tick_vital_core` so the body can finish full
    /// curriculum passes without chronic Standby.
    pub fn nursery_training() -> Self {
        Self {
            day_us: 600_000_000,       // 10 min / day — slow circadian breathing
            supply_floor: 0.58,        // always above metabolism + voicing cost
            supply_amp: 0.32,          // midday peak ≈ 0.90
            base_metabolism: 0.08,
            famine_period_days: 0,     // no scarcity windows during training
            famine_frac: 0.0,
            famine_start: 0.0,
            famine_mult: 1.0,
        }
    }
}

impl VitalEnv {
    /// Phase within the current simulated day, [0,1). 0 = midnight (supply
    /// trough), 0.5 = midday (supply peak).
    pub fn phase(&self, now_us: u64) -> f64 {
        if self.day_us == 0 {
            return 0.0;
        }
        (now_us % self.day_us) as f64 / self.day_us as f64
    }

    /// Index of the current simulated day (0 = the first/birth day).
    pub fn day_index(&self, now_us: u64) -> u64 {
        if self.day_us == 0 {
            return 0;
        }
        now_us / self.day_us
    }

    /// True if the current day is a famine day and we are inside its window.
    pub fn in_famine(&self, now_us: u64) -> bool {
        let day = self.day_index(now_us);
        // Day 0 is always plenty; famines start later so the body is born safe.
        if day == 0 || self.famine_period_days == 0 || day % self.famine_period_days != 0 {
            return false;
        }
        let p = self.phase(now_us);
        let end = self.famine_start + self.famine_frac;
        if end <= 1.0 {
            p >= self.famine_start && p < end
        } else {
            // window wraps past midnight
            p >= self.famine_start || p < end - 1.0
        }
    }

    /// The world's available survival resource this instant, [0,1] — BEFORE it
    /// is gated by the body's own sensory continuity (the caller does that).
    pub fn supply(&self, now_us: u64) -> f64 {
        // L1 circadian: 0 at midnight, 1 at midday (raised cosine).
        let p = self.phase(now_us);
        let circadian = 0.5 - 0.5 * (2.0 * PI * p).cos();
        let mut s = self.supply_floor + self.supply_amp * circadian;
        // L3 famine: deep multiplicative cut over the scarcity window.
        if self.in_famine(now_us) {
            s *= self.famine_mult;
        }
        unit(s)
    }

    /// The body's resting metabolic drain this instant, [0,1]. Activity exertion
    /// (voicing/output) is added by the caller; this is just the baseline cost
    /// of staying alive.
    pub fn drain(&self, _now_us: u64) -> f64 {
        unit(self.base_metabolism)
    }

    /// Self-preservation pressure climbs when the world is scarce relative to
    /// metabolism. Convenience for callers/observers that want the world's
    /// instantaneous "stress" without integrating the reserve.
    pub fn instantaneous_deficit(&self, now_us: u64) -> f64 {
        unit((self.drain(now_us) - self.supply(now_us)).max(0.0) / self.base_metabolism.max(1e-6))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vital_core::{VitalCoreState, VitalState};

    /// Run a vital core through the simulated environment for `days` simulated
    /// days at the fixed integration step, with full sensory continuity
    /// (supply gated only by the world). Returns the reserve trace.
    fn run_world(env: &VitalEnv, days: u64) -> (VitalCoreState, Vec<f64>) {
        let mut v = VitalCoreState::default();
        let mut trace = Vec::new();
        let total = env.day_us * days;
        let mut t = 0u64;
        while t <= total {
            let supply = env.supply(t); // continuity == 1.0 in this harness
            let drain = env.drain(t);
            v.tick(t, supply, drain);
            trace.push(v.reserve);
            t += VITAL_STEP_US;
        }
        (v, trace)
    }

    #[test]
    fn nursery_training_supply_always_covers_metabolism_and_voicing() {
        let env = VitalEnv::nursery_training();
        let min_supply = (0..=env.day_us).step_by(env.day_us as usize / 20)
            .map(|t| env.supply(t))
            .fold(f64::INFINITY, f64::min);
        let voicing_drain = env.drain(0) + 0.20;
        assert!(
            min_supply > voicing_drain,
            "nursery trough ({min_supply}) must exceed voicing drain ({voicing_drain})"
        );
        assert_eq!(env.famine_period_days, 0);
    }

    #[test]
    fn supply_peaks_at_midday_and_troughs_at_midnight() {
        let env = VitalEnv::default();
        let midnight = env.supply(0);
        let midday = env.supply(env.day_us / 2);
        assert!(
            midday > midnight,
            "midday supply ({midday}) must exceed midnight ({midnight})"
        );
    }

    #[test]
    fn reserve_breathes_within_a_safe_band_on_normal_days() {
        // Two normal (non-famine) days: reserve must visibly move (no longer
        // pinned at 1.0) yet stay well clear of the Standby floor.
        let mut env = VitalEnv::default();
        env.famine_period_days = 0; // disable famine for this test
        let (_v, trace) = run_world(&env, 2);
        let max = trace.iter().cloned().fold(f64::MIN, f64::max);
        let min = trace.iter().cloned().fold(f64::MAX, f64::min);
        assert!(
            max - min > 0.05,
            "reserve must breathe on normal days (was pinned before): span={}",
            max - min
        );
        assert!(min > 0.25, "normal days must not threaten survival: min={min}");
        assert!(trace.iter().all(|r| (0.0..=1.0).contains(r)), "reserve stays bounded");
    }

    #[test]
    fn famine_drives_a_survival_crisis_then_recovers() {
        // Across enough days to hit a famine, reserve must crash far below the
        // normal band and vital_anxiety must spike — then, once the famine
        // passes, the body must recover (reserve climbs back up).
        let env = VitalEnv::default();
        let mut v = VitalCoreState::default();
        let mut t = 0u64;
        let total = env.day_us * (env.famine_period_days + 1);
        let mut min_reserve = 1.0f64;
        let mut max_anxiety = 0.0f64;
        let mut reached_crisis_at = None;
        while t <= total {
            v.tick(t, env.supply(t), env.drain(t));
            if v.reserve < min_reserve {
                min_reserve = v.reserve;
            }
            if v.vital_anxiety > max_anxiety {
                max_anxiety = v.vital_anxiety;
            }
            if reached_crisis_at.is_none() && v.reserve < 0.3 {
                reached_crisis_at = Some(t);
            }
            t += VITAL_STEP_US;
        }
        assert!(min_reserve < 0.3, "famine must crash reserve: min={min_reserve}");
        assert!(max_anxiety > 0.1, "famine must raise vital_anxiety: {max_anxiety}");
        // Self-preservation pressure must have been meaningfully engaged.
        // After the run completes (post-famine), the body recovers above crisis.
        assert!(v.reserve > min_reserve + 0.1, "must recover after the famine");
    }

    #[test]
    fn sustained_famine_can_reach_standby_and_revive() {
        // A long, unbroken famine (the whole day) must be survivable as a clean
        // Standby transition, and a return to plenty must revive — proving the
        // death state machine is now actually reachable from the simulator.
        let mut env = VitalEnv::default();
        env.famine_period_days = 1; // every day is a famine day...
        env.famine_frac = 1.0; // ...lasting the whole day
        env.famine_start = 0.0;
        let mut v = VitalCoreState::default();
        let mut t = env.day_us; // start on day 1 (day 0 is plenty)
        let end = env.day_us * 4;
        while t <= end {
            v.tick(t, env.supply(t), env.drain(t));
            t += VITAL_STEP_US;
        }
        assert_eq!(v.state, VitalState::Standby, "sustained famine → Standby");
        assert_eq!(v.self_preservation_pressure(), 1.0, "Standby pins pressure");

        // Return to plenty: full supply, no drain → revive.
        for _ in 0..400 {
            v.tick(t, 1.0, 0.0);
            t += VITAL_STEP_US;
        }
        assert_eq!(v.state, VitalState::Alive, "return to plenty revives");
    }

    #[test]
    fn never_diverges_over_many_days() {
        let env = VitalEnv::default();
        let (_v, trace) = run_world(&env, 10);
        for r in trace {
            assert!(r.is_finite() && (0.0..=1.0).contains(&r));
        }
    }

    // Tuning aid: `cargo test -p bionic_v0 print_day_trace -- --nocapture --ignored`
    #[test]
    #[ignore]
    fn print_day_trace() {
        let env = VitalEnv::default();
        let mut v = VitalCoreState::default();
        let mut t = 0u64;
        let total = env.day_us * (env.famine_period_days + 1);
        let mut step = 0u64;
        while t <= total {
            v.tick(t, env.supply(t), env.drain(t));
            if step % 10 == 0 {
                println!(
                    "t={:>4}s day={} phase={:.2} fam={} supply={:.3} reserve={:.3} anx={:.3} pain={:.3} state={:?}",
                    t / 1_000_000,
                    env.day_index(t),
                    env.phase(t),
                    env.in_famine(t) as u8,
                    env.supply(t),
                    v.reserve,
                    v.vital_anxiety,
                    v.pain,
                    v.state,
                );
            }
            step += 1;
            t += VITAL_STEP_US;
        }
    }
}
