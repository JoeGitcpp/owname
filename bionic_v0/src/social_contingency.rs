//! Social-contingency credit: turn-taking → developmental clock.
//!
//! # Why this exists
//!
//! Before this module the developmental clock (`developmental_maturity`) was fed
//! ONLY by self-echo competence: the body matured by hearing its own voice and
//! getting closer to a target (`advance_maturity_by_progress` off `vocal_efficacy`'s
//! rise). The "I vocalised → an external voice answered" turn-taking signal was
//! detected (`phoneme_memory::observe_vib`) and stored (`social_response_rate`),
//! but — per its own doc — was "TELEMETRY ONLY ... Not yet wired". So a body
//! matured identically whether alone or in a real conversation: 对外理解和反馈
//! earned nothing. This wires that contingency into the SAME learning-progress
//! clock, as a disjoint credit axis (no double-count: external VIB is firewalled
//! from the efficacy path by the 150ms self-hearing gate).
//!
//! # Why ΔP, not the raw response rate
//!
//! The naive wiring — "external sound within 3 s of voicing ⇒ credit" — rewards
//! mere temporal CO-OCCURRENCE: a fan, HVAC hum, or continuous babble would
//! mature the body (the adversarial review's fatal flaw F3). Credit must be a
//! CONTINGENCY (prediction error), not co-occurrence. So we estimate Rescorla's
//!
//!   ΔP = P(external event | recently voiced) − P(external event | NOT recently voiced)
//!
//! Constant ambient noise raises BOTH conditionals equally → ΔP ≈ 0 → no credit.
//! Only sound that genuinely FOLLOWS the body's own utterance (and is otherwise
//! quiet) yields ΔP > 0. This is the honest "my voice elicited an answer" signal.
//!
//! # Honesty gates (from the review)
//!
//! - F2 (rectified-noise inflation): credit the HIGH-WATER-MARK RISE of ΔP, and
//!   only once both conditionals have ≥ CONSOLIDATE_MIN samples, so an early noisy
//!   EMA cannot ratchet maturity. The per-tick step is further capped downstream
//!   by `advance_maturity_by_progress`'s `MATURITY_PROGRESS_STEP_MAX`.
//! - F5 (cross-lineage contamination): this state is RUNTIME-ONLY — never
//!   serialized, never merged. `Default` (all zeros) is the lineage reset, so a
//!   fresh/merged body inherits no unearned social high-water-mark. The clock it
//!   feeds (`developmental_maturity`) is what persists.
//! - Bootstrap (0×x=0): social credit is ADDITIVE over the intact self-echo path;
//!   a newborn alone still matures. ΔP simply stays 0 with no caregiver.

/// EMA smoothing for each conditional response rate. Slow (one sample per vocal
/// tick ≈ 5 Hz) so the rate reflects a stable regime, not a single event.
pub const SOCIAL_CONTINGENCY_ALPHA: f64 = 0.02;

/// Minimum samples on EACH conditional before ΔP is allowed to credit maturity.
/// Below this the EMAs are too green to trust (F2 consolidation gate).
pub const SOCIAL_CONTINGENCY_CONSOLIDATE_MIN: u64 = 30;

/// Significance floor: a ΔP below this is not a real contingency and earns
/// nothing. Two effects, both honesty (F2/F3): (1) it discards the tiny transient
/// ΔP (≈ α·(1−α)^(CONSOLIDATE−1) ≈ 0.011 here) that constant ambient noise
/// produces purely from the post-rate being sampled one EMA step ahead of the
/// ambient-rate — without it, steady noise would ratchet a small but nonzero
/// maturity. (2) It demands a minimum effect size before "my voice elicits
/// answers" counts as learned, so a barely-above-chance coincidence is not
/// rubber-stamped. A genuine caregiver contingency (ΔP → ~1, or even a 30%
/// responder at ΔP ≈ 0.3) clears it comfortably.
pub const SOCIAL_CONTINGENCY_MIN: f64 = 0.05;

/// Runtime-only estimator of the social turn-taking contingency. NOT persisted
/// (see module docs: `Default` is the lineage reset). One `update` per vocal tick.
#[derive(Clone, Copy, Debug, Default)]
pub struct SocialContingencyState {
    /// EMA of P(external event present | body recently voiced).
    resp_rate_post: f64,
    /// EMA of P(external event present | body NOT recently voiced).
    resp_rate_ambient: f64,
    /// Samples accrued into each conditional (consolidation gate).
    n_post: u64,
    n_ambient: u64,
    /// High-water-mark of ΔP — maturity is credited only on a rise above this.
    contingency_hwm: f64,
}

impl SocialContingencyState {
    /// One observation per vocal tick.
    ///
    /// * `recently_voiced` — the body voiced within the social-response window.
    /// * `external_event` — a fresh NON-self (self_match < 0.7) VIB event is
    ///   present this tick.
    ///
    /// Returns the maturity IMPROVEMENT to credit (the rise of ΔP above its
    /// high-water-mark), or `0.0` when there is nothing honest to credit.
    pub fn update(&mut self, recently_voiced: bool, external_event: bool) -> f64 {
        let obs = if external_event { 1.0 } else { 0.0 };
        if recently_voiced {
            self.resp_rate_post += SOCIAL_CONTINGENCY_ALPHA * (obs - self.resp_rate_post);
            self.n_post = self.n_post.saturating_add(1);
        } else {
            self.resp_rate_ambient += SOCIAL_CONTINGENCY_ALPHA * (obs - self.resp_rate_ambient);
            self.n_ambient = self.n_ambient.saturating_add(1);
        }
        // Both conditionals need enough evidence before the difference is real.
        if self.n_post < SOCIAL_CONTINGENCY_CONSOLIDATE_MIN
            || self.n_ambient < SOCIAL_CONTINGENCY_CONSOLIDATE_MIN
        {
            return 0.0;
        }
        // Significance floor: only the part of ΔP ABOVE the minimum effect size
        // is real contingency. Constant noise's transient ΔP sits below it → 0.
        let signif = (self.contingency() - SOCIAL_CONTINGENCY_MIN).max(0.0);
        if signif > self.contingency_hwm {
            let improvement = signif - self.contingency_hwm;
            self.contingency_hwm = signif;
            improvement
        } else {
            0.0
        }
    }

    /// Current ΔP estimate, clamped to [0, 1]. Exposed for telemetry.
    pub fn contingency(&self) -> f64 {
        (self.resp_rate_post - self.resp_rate_ambient).clamp(0.0, 1.0)
    }
}
