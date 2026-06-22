//! Phoneme prototype memory — 4-layer vocal self-learning scaffold.
//!
//! # Why this module exists
//!
//! A silicon life form cannot speak before it knows **what it sounds like**.
//! This module implements the minimal substrate for that bootstrapping loop:
//!
//! ```text
//! L1  Self-exploration / babbling
//!       endogenous_baseline fires → vocal burst → speaker → mic
//!       The body hears itself.  Motor params (pitch/resonance/noise) are
//!       stamped against the acoustic outcome (spectral centre / resonance /
//!       envelope) in a PhonemeProto.  Pool grows up to PROTO_CAP entries.
//!
//! L2  Fast-path memory
//!       Repeatedly voiced motor states crystallise into prototypes via EMA.
//!       When exploration_motor() selects the least-recently-rehearsed stable
//!       proto, voicing converges on the body's existing repertoire rather than
//!       wandering randomly — this is "practicing" rather than "babbling".
//!
//! L3  Directed causal binding ("I spoke → something changed")
//!       observe_vib() scans recent_vocal for a vocalization within
//!       SOCIAL_RESPONSE_WINDOW_US. A low-self-match VIB arriving in that
//!       window is attributed as a response, not coincidence.
//!
//! L4  Social fast-path / naming seeds
//!       credit_response() increments social_response_rate on the proto that
//!       spoke.  social_response_rate is TELEMETRY ONLY today — intended as the
//!       future seed of a name / greeting (voiced again under attachment
//!       pressure), but no selection/naming/drive logic reads it yet (consumer
//!       de-scoped 2026-04-20). Not yet wired.
//! ```
//!
//! # Integration points (all in viewport.rs)
//!
//! | Call site | Method | Layer |
//! |---|---|---|
//! | `tick_vocal_drive_field` on `begin_voicing` | `exploration_motor` + `record_voiced` | L1/L2 |
//! | `tick_vocal_target_from_vib` on self-play (self_match>0.85) | `update_acoustic_latest` | L1/L2 |
//! | `tick_vocal_target_from_vib` on external VIB (self_match<0.7) | `observe_vib` + `credit_response` | L3/L4 |
//!
//! # Persistence
//!
//! `serialize()` / `deserialize()` produce a version-tagged flat binary that
//! body.rs checkpoints to `phoneme_memory.bin` in the runtime dir alongside
//! `learned_bias.bin`.  No serde dependency; layout documented below.
//!
//! # No external crates
//!
//! LCG RNG is self-contained (same pattern as `vocal_synthesis.rs`).

use std::collections::VecDeque;

// ─── Capacity constants ────────────────────────────────────────────────────────

/// Maximum simultaneously-active phoneme prototypes.
/// 32 slots comfortably covers the phoneme inventory of most languages plus
/// room for blends, filler sounds and novel explorations.
/// B2 (2026-06-22): shrunk 32 → 12. The live inventory was 32/32 protos tiling
/// the motor space at ~0.1 spacing — a discretized SMEAR, not a discrete phonemic
/// inventory. A smaller cap forces COARSE categories (fewer, better-separated
/// basins) and gives the inter-prototype repulsion room to actually separate
/// them. Loading a pool saved under a larger cap truncates to the first PROTO_CAP.
pub const PROTO_CAP: usize = 12;

/// Minimum rehearsal count before a proto is considered "stable" and eligible
/// for the exploit-vs-explore tradeoff in `exploration_motor`.
pub const PROTO_MIN_REHEARSALS: u64 = 6;

/// Sanity ceiling on rehearsal_count. The distributed merge used to SUM counts
/// (`+=`); because aborn seeds each ephemeral body's phoneme pool FROM master and
/// merges it back, the merge re-added master's own seeded counts to itself, so
/// rehearsal_count DOUBLED every ~10-min merge cycle and overflowed to ~1e19
/// within hours (observed live: 0/32 protos had a sane count). Garbage counts
/// broke eviction (evict least-rehearsed) and exploration (pick least-rehearsed),
/// so the pool churned randomly and never crystallized a stable inventory. The
/// fix is twofold: merge by MAX not SUM (idempotent under re-seed), and clamp to
/// this ceiling everywhere (increment, merge, deserialize-heal). Far above the
/// stable threshold and the maturity-score saturation, far below u64 overflow.
pub const REHEARSAL_MAX: u64 = 1_000_000;

/// Time window (µs) in which an external VIB event following own vocalization
/// counts as a potential social response (Layer 3).
pub const SOCIAL_RESPONSE_WINDOW_US: u64 = 3_000_000; // 3 s

/// Ring-buffer depth for recent vocal events (causal attribution window).
const RECENT_VOCAL_CAP: usize = 16;

// ─── Motor state ──────────────────────────────────────────────────────────────

/// Compact motor-space fingerprint: the three free articulation parameters of
/// the vocal organ.  `breath_drive` is excluded — it is loudness/amplitude, not
/// articulation identity.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct MotorState {
    pub pitch_tension: f64, // [0, 1] — controls F0 (180–700 Hz)
    pub resonance_pos: f64, // [0, 1] — shapes vowel formants
    pub noise_mix: f64,     // [0, 1] — voiced-to-noisy ratio
}

impl MotorState {
    /// Euclidean distance in motor space.
    #[inline]
    pub fn dist(&self, other: &MotorState) -> f64 {
        let dp = self.pitch_tension - other.pitch_tension;
        let dr = self.resonance_pos - other.resonance_pos;
        let dn = self.noise_mix - other.noise_mix;
        (dp * dp + dr * dr + dn * dn).sqrt()
    }

    #[inline]
    pub fn clamp01(self) -> Self {
        Self {
            pitch_tension: self.pitch_tension.clamp(0.0, 1.0),
            resonance_pos: self.resonance_pos.clamp(0.0, 1.0),
            noise_mix: self.noise_mix.clamp(0.0, 1.0),
        }
    }
}

// ─── Acoustic state ───────────────────────────────────────────────────────────

/// Three-dimensional acoustic footprint of a phoneme observed at the
/// microphone.  Initially estimated from motor params; refined by self-play.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AcousticState {
    pub spectral_center: f64, // [0, 1] — normalized frequency gravity
    pub resonance: f64,       // [0, 1] — high-band / total-band ratio
    pub envelope: f64,        // [0, 1] — loudness / attack strength
}

impl AcousticState {
    /// Euclidean distance in acoustic space.
    #[inline]
    pub fn dist(&self, other: &AcousticState) -> f64 {
        let dc = self.spectral_center - other.spectral_center;
        let dr = self.resonance - other.resonance;
        let de = self.envelope - other.envelope;
        (dc * dc + dr * dr + de * de).sqrt()
    }
}

// ─── Phoneme prototype ────────────────────────────────────────────────────────

/// One entry in the phoneme pool.  Accumulates evidence from both self-play
/// (motor-acoustic binding) and social interactions (social_response_rate).
#[derive(Clone, Debug)]
pub struct PhonemeProto {
    /// Motor-space centroid (EMA-updated at each rehearsal).
    pub motor: MotorState,
    /// Acoustic outcome centroid (EMA-updated from self-play observations).
    pub acoustic: AcousticState,
    /// How many times this proto has been actively voiced.
    pub rehearsal_count: u64,
    /// Wall-clock µs of last vocalization.
    pub last_voiced_us: u64,
    /// EMA fraction of vocalizations that received a social response within
    /// SOCIAL_RESPONSE_WINDOW_US.  0 = never elicits reply; 1 = always.
    pub social_response_rate: f64,
    /// Raw count of confirmed social responses (Layer 4 credit events).
    pub social_response_count: u32,
    /// Whether this slot is occupied.
    pub active: bool,
}

impl Default for PhonemeProto {
    fn default() -> Self {
        Self {
            motor: MotorState::default(),
            acoustic: AcousticState::default(),
            rehearsal_count: 0,
            last_voiced_us: 0,
            social_response_rate: 0.0,
            social_response_count: 0,
            active: false,
        }
    }
}

// ─── Recent-vocal ring-buffer entry ──────────────────────────────────────────

/// One entry in the causal attribution ring buffer.
/// Pushed by `record_voiced`; consumed by `observe_vib`.
#[derive(Clone, Debug)]
struct RecentVocal {
    t_us: u64,
    proto_idx: Option<usize>,
    #[allow(dead_code)]
    motor: MotorState,
}

// ─── LCG RNG (no external crate) ─────────────────────────────────────────────

struct LcgRng {
    state: u64,
}

impl LcgRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    #[inline]
    fn next_u64(&mut self) -> u64 {
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.state
    }

    /// Uniform in [0, 1).
    #[inline]
    fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 52) as f64
    }

    /// Uniform in [-1, 1).
    #[inline]
    fn next_f64_signed(&mut self) -> f64 {
        self.next_f64() * 2.0 - 1.0
    }
}

// ─── PhonemeMemory ────────────────────────────────────────────────────────────

pub struct PhonemeMemory {
    /// Fixed-size pool of phoneme prototypes.
    pub protos: Vec<PhonemeProto>,
    /// Ring buffer of recent vocal events for causal attribution.
    recent_vocal: VecDeque<RecentVocal>,
    rng: LcgRng,
}

impl Default for PhonemeMemory {
    fn default() -> Self {
        Self::new()
    }
}

impl PhonemeMemory {
    pub fn new() -> Self {
        Self {
            protos: vec![PhonemeProto::default(); PROTO_CAP],
            recent_vocal: VecDeque::with_capacity(RECENT_VOCAL_CAP),
            rng: LcgRng::new(0xdead_beef_cafe_babe),
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // L1/L2: record a voiced event
    // ──────────────────────────────────────────────────────────────────────────

    /// Called when the body starts a new vocal burst (`begin_voicing`).
    ///
    /// Finds or creates the nearest `PhonemeProto` and updates it via EMA.
    /// Pushes to the recent-vocal ring buffer so that a subsequent social
    /// response can be attributed to this proto.
    ///
    /// Returns the proto index for potential external use.
    pub fn record_voiced(
        &mut self,
        motor: MotorState,
        acoustic: AcousticState,
        now_us: u64,
    ) -> usize {
        /// Motor-space radius within which two productions are considered the
        /// "same phoneme" and merge rather than birth a new proto.
        const MATCH_RADIUS: f64 = 0.25;
        const EMA_ALPHA: f64 = 0.10;

        // Find nearest active prototype.
        let best_idx = self
            .protos
            .iter()
            .enumerate()
            .filter(|(_, p)| p.active)
            .min_by(|(_, a), (_, b)| {
                a.motor
                    .dist(&motor)
                    .partial_cmp(&b.motor.dist(&motor))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(i, _)| i);

        let idx = match best_idx {
            Some(i) if self.protos[i].motor.dist(&motor) < MATCH_RADIUS => {
                // ── Merge into existing proto via EMA ──────────────────────
                let p = &mut self.protos[i];
                p.motor.pitch_tension =
                    p.motor.pitch_tension * (1.0 - EMA_ALPHA) + motor.pitch_tension * EMA_ALPHA;
                p.motor.resonance_pos =
                    p.motor.resonance_pos * (1.0 - EMA_ALPHA) + motor.resonance_pos * EMA_ALPHA;
                p.motor.noise_mix =
                    p.motor.noise_mix * (1.0 - EMA_ALPHA) + motor.noise_mix * EMA_ALPHA;
                p.acoustic.spectral_center = p.acoustic.spectral_center * (1.0 - EMA_ALPHA)
                    + acoustic.spectral_center * EMA_ALPHA;
                p.acoustic.resonance =
                    p.acoustic.resonance * (1.0 - EMA_ALPHA) + acoustic.resonance * EMA_ALPHA;
                p.acoustic.envelope =
                    p.acoustic.envelope * (1.0 - EMA_ALPHA) + acoustic.envelope * EMA_ALPHA;
                p.rehearsal_count = (p.rehearsal_count + 1).min(REHEARSAL_MAX);
                p.last_voiced_us = now_us;
                i
            }
            _ => {
                // ── Birth or evict ─────────────────────────────────────────
                let slot = self.protos.iter().position(|p| !p.active);
                match slot {
                    Some(i) => {
                        self.protos[i] = PhonemeProto {
                            motor,
                            acoustic,
                            rehearsal_count: 1,
                            last_voiced_us: now_us,
                            social_response_rate: 0.0,
                            social_response_count: 0,
                            active: true,
                        };
                        i
                    }
                    None => {
                        // Pool full — evict the weakest (least rehearsed,
                        // fewest social responses).
                        let evict = self
                            .protos
                            .iter()
                            .enumerate()
                            .filter(|(_, p)| p.active)
                            .min_by(|(_, a), (_, b)| {
                                let sa = a.rehearsal_count as f64
                                    + a.social_response_count as f64 * 10.0;
                                let sb = b.rehearsal_count as f64
                                    + b.social_response_count as f64 * 10.0;
                                sa.partial_cmp(&sb)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            })
                            .map(|(i, _)| i)
                            .unwrap_or(0);
                        self.protos[evict] = PhonemeProto {
                            motor,
                            acoustic,
                            rehearsal_count: 1,
                            last_voiced_us: now_us,
                            social_response_rate: 0.0,
                            social_response_count: 0,
                            active: true,
                        };
                        evict
                    }
                }
            }
        };

        // Push to causal ring buffer.
        if self.recent_vocal.len() >= RECENT_VOCAL_CAP {
            self.recent_vocal.pop_front();
        }
        self.recent_vocal.push_back(RecentVocal {
            t_us: now_us,
            proto_idx: Some(idx),
            motor,
        });
        // B2: after rehearsing/birthing, push this proto AWAY from its nearest
        // neighbor so distinct gestures separate into discrete basins.
        self.repel_from_neighbors(idx);
        idx
    }

    /// B2 (2026-06-22): anti-Hebbian inter-prototype repulsion. The EMA merge in
    /// `record_voiced` is mean-seeking — within a basin it (correctly) refines
    /// toward the exemplar centroid, but it gives NO force separating distinct
    /// protos, so the inventory drifts into one overlapping cloud (the confirmed
    /// mean-collapse). This adds the DIVERGENT term: push the just-updated proto
    /// away from its nearest OTHER active proto, strength growing as they get
    /// closer and vanishing beyond REPEL_RADIUS. Distinct gestures thus settle
    /// into SEPARATED basins (a discrete inventory) instead of diluting to a
    /// shared mean — the categorizing dynamic that makes babble→speech possible.
    /// Local, label-free substrate dynamics (no phoneme table, no reward).
    fn repel_from_neighbors(&mut self, idx: usize) {
        const REPEL_RADIUS: f64 = 0.40;
        const REPEL_STRENGTH: f64 = 0.04;
        let me = self.protos[idx].motor;
        // B2 strengthened (2026-06-22): repel from ALL neighbors within the radius,
        // not just the nearest. In a DENSE cloud (the live failure: 32 protos at
        // ~0.1 spacing), pushing away from one neighbor just shoves toward another —
        // net zero. Summing the repulsion from every nearby proto gives a real
        // outward force that expands a packed cloud into separated basins.
        let (mut dp, mut dr, mut dn) = (0.0_f64, 0.0_f64, 0.0_f64);
        for (j, p) in self.protos.iter().enumerate() {
            if j == idx || !p.active {
                continue;
            }
            let nj = p.motor;
            let d = nj.dist(&me);
            if d > 1e-6 && d < REPEL_RADIUS {
                // push strength · unit(neighbor→me); strength grows as they near.
                let s = REPEL_STRENGTH * (1.0 - d / REPEL_RADIUS) / d;
                dp += s * (me.pitch_tension - nj.pitch_tension);
                dr += s * (me.resonance_pos - nj.resonance_pos);
                dn += s * (me.noise_mix - nj.noise_mix);
            }
        }
        let p = &mut self.protos[idx].motor;
        p.pitch_tension = (p.pitch_tension + dp).clamp(0.0, 1.0);
        p.resonance_pos = (p.resonance_pos + dr).clamp(0.0, 1.0);
        p.noise_mix = (p.noise_mix + dn).clamp(0.0, 1.0);
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Distributed-training merge: fold another life's phoneme pool into this one
    // ──────────────────────────────────────────────────────────────────────────

    /// Mainline ← branch. For each active proto in `other`, find the nearest
    /// active local proto within MATCH_RADIUS and combine them EVIDENCE-WEIGHTED
    /// (motor/acoustic averaged by rehearsal_count, rehearsal + social counts
    /// SUM) so a phoneme independently rehearsed across branches grows stronger;
    /// a novel proto fills an inactive slot, or evicts the weakest only if the
    /// incoming proto is stronger (no regression). Mirrors record_voiced's
    /// matching/eviction policy.
    pub fn merge(&mut self, other: &PhonemeMemory) {
        const MATCH_RADIUS: f64 = 0.25;
        let strength = |p: &PhonemeProto| p.rehearsal_count as f64 + p.social_response_count as f64 * 10.0;
        for op in other.protos.iter().filter(|p| p.active) {
            let best = self
                .protos
                .iter()
                .enumerate()
                .filter(|(_, p)| p.active)
                .min_by(|(_, a), (_, b)| {
                    a.motor
                        .dist(&op.motor)
                        .partial_cmp(&b.motor.dist(&op.motor))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|(i, _)| i);
            match best {
                Some(i) if self.protos[i].motor.dist(&op.motor) < MATCH_RADIUS => {
                    let p = &mut self.protos[i];
                    let wa = p.rehearsal_count.max(1) as f64;
                    let wb = op.rehearsal_count.max(1) as f64;
                    let w = wa + wb;
                    p.motor.pitch_tension = (p.motor.pitch_tension * wa + op.motor.pitch_tension * wb) / w;
                    p.motor.resonance_pos = (p.motor.resonance_pos * wa + op.motor.resonance_pos * wb) / w;
                    p.motor.noise_mix = (p.motor.noise_mix * wa + op.motor.noise_mix * wb) / w;
                    p.acoustic.spectral_center =
                        (p.acoustic.spectral_center * wa + op.acoustic.spectral_center * wb) / w;
                    p.acoustic.resonance = (p.acoustic.resonance * wa + op.acoustic.resonance * wb) / w;
                    p.acoustic.envelope = (p.acoustic.envelope * wa + op.acoustic.envelope * wb) / w;
                    p.social_response_rate = (p.social_response_rate * wa + op.social_response_rate * wb) / w;
                    // MAX, not SUM: aborn seeds each branch's pool FROM master and
                    // merges it back, so SUM re-added master's own seeded counts to
                    // itself and DOUBLED every merge cycle → overflow to ~1e19. MAX
                    // is idempotent under re-seed (a genuinely-more-rehearsed lineage
                    // still raises the count) and clamped for safety.
                    p.rehearsal_count = p.rehearsal_count.max(op.rehearsal_count).min(REHEARSAL_MAX);
                    p.social_response_count = p.social_response_count.max(op.social_response_count);
                    p.last_voiced_us = p.last_voiced_us.max(op.last_voiced_us);
                }
                _ => {
                    if let Some(i) = self.protos.iter().position(|p| !p.active) {
                        self.protos[i] = op.clone();
                    } else {
                        // Pool full: evict the weakest, but only if the incoming
                        // proto carries more evidence (never regress the pool).
                        if let Some((wi, _)) = self
                            .protos
                            .iter()
                            .enumerate()
                            .min_by(|(_, a), (_, b)| {
                                strength(a).partial_cmp(&strength(b)).unwrap_or(std::cmp::Ordering::Equal)
                            })
                        {
                            if strength(op) > strength(&self.protos[wi]) {
                                self.protos[wi] = op.clone();
                            }
                        }
                    }
                }
            }
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // L1/L2: refine acoustic fingerprint from self-play echo
    // ──────────────────────────────────────────────────────────────────────────

    /// Called when the body confirms it heard its own recent vocalization back
    /// (self_match > 0.85 in `tick_vocal_target_from_vib`).
    ///
    /// Refines the acoustic fingerprint of the most recently voiced proto using
    /// the **actual observed** VIB data — overriding the initial motor-derived
    /// estimate.  Uses a faster EMA alpha so the fingerprint converges quickly
    /// through repeated babble iterations.
    pub fn update_acoustic_latest(&mut self, acoustic: AcousticState, now_us: u64) {
        const EMA_ALPHA_FAST: f64 = 0.20;
        const WINDOW_US: u64 = 2_000_000; // 2 s

        let proto_idx = self
            .recent_vocal
            .iter()
            .rev()
            .find(|rv| now_us.saturating_sub(rv.t_us) <= WINDOW_US)
            .and_then(|rv| rv.proto_idx);

        let Some(idx) = proto_idx else { return };
        let Some(proto) = self.protos.get_mut(idx) else {
            return;
        };
        if !proto.active {
            return;
        }
        proto.acoustic.spectral_center = proto.acoustic.spectral_center * (1.0 - EMA_ALPHA_FAST)
            + acoustic.spectral_center * EMA_ALPHA_FAST;
        proto.acoustic.resonance = proto.acoustic.resonance * (1.0 - EMA_ALPHA_FAST)
            + acoustic.resonance * EMA_ALPHA_FAST;
        proto.acoustic.envelope = proto.acoustic.envelope * (1.0 - EMA_ALPHA_FAST)
            + acoustic.envelope * EMA_ALPHA_FAST;
    }

    // ──────────────────────────────────────────────────────────────────────────
    // L3/L4: observe an incoming (non-self) VIB event
    // ──────────────────────────────────────────────────────────────────────────

    /// Called when an external VIB event arrives with `self_match < 0.7`.
    ///
    /// Checks whether a recent vocalization falls within the social-response
    /// window.  If so, returns `(proto_idx, confidence)` so the caller can
    /// invoke `credit_response`.
    ///
    /// `confidence` ∈ [0, 1]: immediate responses score ≈ 1.0; responses near
    /// the window edge score ≈ 0.0.
    pub fn observe_vib(
        &mut self,
        _acoustic: AcousticState, // reserved for future acoustic-similarity gating
        self_match: f64,
        now_us: u64,
    ) -> Option<(usize, f64)> {
        // Only attribute external responses, not own echo.
        if self_match >= 0.7 {
            return None;
        }

        // Find the most recent vocal event inside the social-response window.
        let candidate = self
            .recent_vocal
            .iter()
            .rev()
            .find(|rv| {
                let elapsed = now_us.saturating_sub(rv.t_us);
                elapsed > 0 && elapsed <= SOCIAL_RESPONSE_WINDOW_US
            })?;

        let proto_idx = candidate.proto_idx?;
        let elapsed_us = now_us.saturating_sub(candidate.t_us);

        // Confidence: linear decay from 1.0 (immediate) → 0.0 (window edge).
        let confidence =
            (1.0 - elapsed_us as f64 / SOCIAL_RESPONSE_WINDOW_US as f64).clamp(0.0, 1.0);

        Some((proto_idx, confidence))
    }

    // ──────────────────────────────────────────────────────────────────────────
    // L4: social credit assignment
    // ──────────────────────────────────────────────────────────────────────────

    /// Strengthen the `social_response_rate` of the proto that received a
    /// social response.  Uses a slow EMA (α=0.05) so the rate only increases
    /// meaningfully with repeated consistent responses.
    pub fn credit_response(&mut self, proto_idx: usize, confidence: f64) {
        let Some(proto) = self.protos.get_mut(proto_idx) else {
            return;
        };
        if !proto.active {
            return;
        }
        proto.social_response_count += 1;
        // Plain EMA (α=0.05) toward THIS call's confidence — NOT a decay-to-zero.
        // It runs only on an observed response, so it tracks recent response
        // confidence: it rises with confident responses and only eases downward
        // when a call's confidence is below the current rate; it never falls on
        // its own between calls. (The old comment claiming "decay toward 0.0 on
        // every call" was wrong.) Telemetry only — not read by selection/naming.
        proto.social_response_rate =
            proto.social_response_rate * 0.95 + confidence * 0.05;
    }

    // ──────────────────────────────────────────────────────────────────────────
    // L1/L2: exploration motor selection
    // ──────────────────────────────────────────────────────────────────────────

    /// Returns the motor parameters for the next exploratory vocal burst.
    ///
    /// **Strategy**:
    /// - Pool empty / no stable protos → **pure exploration**: uniform random
    ///   sample across motor space (large sigma → wide babble).
    /// - Stable protos exist → **exploit-with-fine-perturbation**: pick the
    ///   least-recently-rehearsed stable proto and add small Gaussian noise
    ///   (σ=0.08) so the body practices the neighbourhood of known sounds.
    ///
    /// `base` is the drive-field's suggested pitch/resonance/noise; used only
    /// when no stable proto exists (it becomes the perturbation centre for
    /// the noisy pure-exploration case).
    pub fn exploration_motor(&mut self, base: MotorState) -> MotorState {
        let stable: Vec<usize> = self
            .protos
            .iter()
            .enumerate()
            .filter(|(_, p)| p.active && p.rehearsal_count >= PROTO_MIN_REHEARSALS)
            .map(|(i, _)| i)
            .collect();

        if stable.is_empty() {
            // Pure exploration: sample broadly but keep noise_mix low so
            // early babble is mostly voiced (more informative self-play).
            let sigma = 0.35_f64;
            return MotorState {
                pitch_tension: (base.pitch_tension + sigma * self.rng.next_f64_signed())
                    .clamp(0.0, 1.0),
                resonance_pos: (base.resonance_pos + sigma * self.rng.next_f64_signed())
                    .clamp(0.0, 1.0),
                noise_mix: (base.noise_mix + sigma * 0.3 * self.rng.next_f64_signed())
                    .clamp(0.0, 0.5),
            };
        }

        // Fine-tune: pick least-recently-rehearsed stable proto.
        let oldest = stable
            .iter()
            .min_by_key(|&&i| self.protos[i].last_voiced_us)
            .copied()
            .unwrap();
        let target = self.protos[oldest].motor;

        const SIGMA_FINE: f64 = 0.08;
        MotorState {
            pitch_tension: (target.pitch_tension + SIGMA_FINE * self.rng.next_f64_signed())
                .clamp(0.0, 1.0),
            resonance_pos: (target.resonance_pos + SIGMA_FINE * self.rng.next_f64_signed())
                .clamp(0.0, 1.0),
            noise_mix: (target.noise_mix + SIGMA_FINE * 0.5 * self.rng.next_f64_signed())
                .clamp(0.0, 1.0),
        }
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Aggregate metrics
    // ──────────────────────────────────────────────────────────────────────────

    /// Returns [0, 1]: overall vocal competence.
    /// Saturates at PROTO_CAP * 50 total rehearsals across the pool.
    pub fn vocal_efficacy(&self) -> f64 {
        let total: u64 = self.protos.iter().filter(|p| p.active).map(|p| p.rehearsal_count).sum();
        let max = (PROTO_CAP as u64) * 50;
        (total as f64 / max as f64).min(1.0)
    }

    /// Number of stable (well-rehearsed) prototypes.
    pub fn stable_count(&self) -> usize {
        self.protos
            .iter()
            .filter(|p| p.active && p.rehearsal_count >= PROTO_MIN_REHEARSALS)
            .count()
    }

    /// Number of active (any rehearsals) prototypes.
    pub fn active_count(&self) -> usize {
        self.protos.iter().filter(|p| p.active).count()
    }

    // ──────────────────────────────────────────────────────────────────────────
    // Serialization / deserialization
    // ──────────────────────────────────────────────────────────────────────────
    //
    // Binary layout (version 1):
    //
    //   [0..4]   magic  = [0xFE, 0xED, 0xBA, 0xBE]
    //   [4]      version = 1 (u8)
    //   [5..7]   n_protos (u16 LE) = PROTO_CAP
    //   per slot (77 bytes each):
    //     [0]    active (u8, 0 or 1)
    //     [1..25] motor: pitch_tension(f64) resonance_pos(f64) noise_mix(f64)
    //     [25..49] acoustic: spectral_center(f64) resonance(f64) envelope(f64)
    //     [49..57] rehearsal_count (u64 LE)
    //     [57..65] last_voiced_us (u64 LE)
    //     [65..73] social_response_rate (f64 LE)
    //     [73..77] social_response_count (u32 LE)

    const MAGIC: [u8; 4] = [0xFE, 0xED, 0xBA, 0xBE];
    const VERSION: u8 = 1;
    const BYTES_PER_PROTO: usize = 1 + 3 * 8 + 3 * 8 + 8 + 8 + 8 + 4; // 77

    pub fn serialize(&self) -> Vec<u8> {
        let mut out =
            Vec::with_capacity(7 + Self::BYTES_PER_PROTO * PROTO_CAP);
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::VERSION);
        out.extend_from_slice(&(PROTO_CAP as u16).to_le_bytes());
        for p in &self.protos {
            out.push(u8::from(p.active));
            for v in &[
                p.motor.pitch_tension,
                p.motor.resonance_pos,
                p.motor.noise_mix,
                p.acoustic.spectral_center,
                p.acoustic.resonance,
                p.acoustic.envelope,
            ] {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&p.rehearsal_count.to_le_bytes());
            out.extend_from_slice(&p.last_voiced_us.to_le_bytes());
            out.extend_from_slice(&p.social_response_rate.to_le_bytes());
            out.extend_from_slice(&p.social_response_count.to_le_bytes());
        }
        out
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        if data.len() < 7 {
            return None;
        }
        if data[0..4] != Self::MAGIC {
            return None;
        }
        if data[4] != Self::VERSION {
            return None;
        }
        let n = u16::from_le_bytes(data[5..7].try_into().ok()?) as usize;
        // n may exceed PROTO_CAP when loading a pool saved under a LARGER cap (the
        // pre-shrink 32-slot master). Read all n slots to advance the cursor, but
        // the loop keeps only the first PROTO_CAP (its `i < PROTO_CAP` guard). Bound
        // n to reject malformed data.
        if n > 1024 {
            return None;
        }
        if data.len() < 7 + n * Self::BYTES_PER_PROTO {
            return None;
        }

        let mut mem = Self::new();
        let mut off = 7usize;

        macro_rules! rf64 {
            () => {{
                let v = f64::from_le_bytes(match data[off..off + 8].try_into() {
                    Ok(b) => b,
                    Err(_) => return None,
                });
                off += 8;
                v
            }};
        }
        macro_rules! ru64 {
            () => {{
                let v = u64::from_le_bytes(match data[off..off + 8].try_into() {
                    Ok(b) => b,
                    Err(_) => return None,
                });
                off += 8;
                v
            }};
        }
        macro_rules! ru32 {
            () => {{
                let v = u32::from_le_bytes(match data[off..off + 4].try_into() {
                    Ok(b) => b,
                    Err(_) => return None,
                });
                off += 4;
                v
            }};
        }

        for i in 0..n {
            let active = data[off] == 1;
            off += 1;
            let pitch_tension = rf64!();
            let resonance_pos = rf64!();
            let noise_mix = rf64!();
            let spectral_center = rf64!();
            let resonance = rf64!();
            let envelope = rf64!();
            let mut rehearsal_count = ru64!();
            let last_voiced_us = ru64!();
            let social_response_rate = rf64!();
            let mut social_response_count = ru32!();
            // Heal the historical merge-doubling corruption on load: any count past
            // the sanity ceiling is unrecoverable garbage (~1e19). Reset rehearsals
            // to the stable threshold (keep the real proto centroid, re-accumulate
            // usage sanely) and zero out garbage social counts. Idempotent for sane
            // files. This self-heals the master pool on the next load by any binary.
            if rehearsal_count > REHEARSAL_MAX {
                rehearsal_count = PROTO_MIN_REHEARSALS;
            }
            if social_response_count as u64 > REHEARSAL_MAX {
                social_response_count = 0;
            }
            if i < PROTO_CAP {
                mem.protos[i] = PhonemeProto {
                    motor: MotorState {
                        pitch_tension,
                        resonance_pos,
                        noise_mix,
                    },
                    acoustic: AcousticState {
                        spectral_center,
                        resonance,
                        envelope,
                    },
                    rehearsal_count,
                    last_voiced_us,
                    social_response_rate,
                    social_response_count,
                    active,
                };
            }
        }
        Some(mem)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(p: f64, r: f64, n: f64) -> MotorState {
        MotorState { pitch_tension: p, resonance_pos: r, noise_mix: n }
    }
    fn ac(s: f64, r: f64, e: f64) -> AcousticState {
        AcousticState { spectral_center: s, resonance: r, envelope: e }
    }

    #[test]
    fn merge_reinforces_shared_phoneme_and_absorbs_novel() {
        let mut mainline = PhonemeMemory::new();
        // mainline has one phoneme, rehearsed once
        mainline.record_voiced(ms(0.5, 0.4, 0.1), ac(0.5, 0.4, 0.6), 1_000);

        let mut branch = PhonemeMemory::new();
        // branch independently rehearsed the SAME phoneme 3× ...
        for _ in 0..3 { branch.record_voiced(ms(0.51, 0.41, 0.1), ac(0.5, 0.4, 0.6), 2_000); }
        // ... and discovered a DISTINCT phoneme (far in motor space)
        branch.record_voiced(ms(0.05, 0.9, 0.8), ac(0.2, 0.8, 0.5), 3_000);

        let before = mainline.active_count();
        mainline.merge(&branch);
        assert_eq!(mainline.active_count(), before + 1, "novel phoneme absorbed, shared one merged");
        // The shared proto's evidence summed (1 + 3 rehearsals).
        let shared = mainline.protos.iter()
            .filter(|p| p.active)
            .find(|p| p.motor.dist(&ms(0.5, 0.4, 0.1)) < 0.25).unwrap();
        // MAX, not SUM: a more-rehearsed lineage (branch=3) raises the count, but
        // re-merging never doubles (idempotent under aborn's seed-from-master).
        assert_eq!(shared.rehearsal_count, 3, "rehearsal evidence takes the max across branches");
    }

    #[test]
    fn merge_is_idempotent_no_rehearsal_doubling() {
        // The corruption root cause: aborn seeds each branch FROM master and merges
        // it back. Re-merging a clone (the seeded baseline) must NOT inflate counts.
        let mut master = PhonemeMemory::new();
        for _ in 0..5 { master.record_voiced(ms(0.5, 0.4, 0.1), ac(0.5, 0.4, 0.6), 1_000); }
        let count0 = master.protos.iter().find(|p| p.active).unwrap().rehearsal_count;
        assert_eq!(count0, 5);
        // Re-merge the seed (a deserialize round-trip from master = exactly how
        // aborn re-seeds a branch) many times.
        let seed = PhonemeMemory::deserialize(&master.serialize()).unwrap();
        for _ in 0..40 { master.merge(&seed); }
        let count1 = master.protos.iter().find(|p| p.active).unwrap().rehearsal_count;
        assert_eq!(count1, 5, "re-merging the seed must not double/grow the count (got {count1})");
        assert!(count1 <= REHEARSAL_MAX, "count bounded by the sanity ceiling");
    }

    #[test]
    fn deserialize_heals_garbage_rehearsal_count() {
        // A pool whose counts overflowed to garbage (~1e19) must heal on load:
        // garbage reset to the stable threshold, real centroids preserved.
        let mut pm = PhonemeMemory::new();
        pm.record_voiced(ms(0.5, 0.4, 0.1), ac(0.5, 0.4, 0.6), 1_000);
        // Inject the historical corruption.
        pm.protos[0].rehearsal_count = 16_950_730_549_853_757_692; // observed live garbage
        let bytes = pm.serialize();
        let healed = PhonemeMemory::deserialize(&bytes).expect("deserialize");
        let p = healed.protos.iter().find(|p| p.active).unwrap();
        assert_eq!(p.rehearsal_count, PROTO_MIN_REHEARSALS, "garbage count healed to stable threshold");
        assert!((p.motor.pitch_tension - 0.5).abs() < 1e-9, "real centroid preserved through heal");
    }

    #[test]
    fn merge_identical_pool_does_not_duplicate() {
        let build = || {
            let mut m = PhonemeMemory::new();
            m.record_voiced(ms(0.5, 0.4, 0.1), ac(0.5, 0.4, 0.6), 1_000);
            m.record_voiced(ms(0.1, 0.8, 0.7), ac(0.2, 0.7, 0.5), 1_000);
            m
        };
        let mut a = build();
        let b = build(); // an identical pool
        let n = a.active_count();
        a.merge(&b); // each proto matches its twin → combine, not birth
        assert_eq!(a.active_count(), n, "merging an identical pool must not duplicate protos");
    }

    #[test]
    fn record_voiced_births_proto() {
        let mut pm = PhonemeMemory::new();
        let motor = MotorState {
            pitch_tension: 0.5,
            resonance_pos: 0.4,
            noise_mix: 0.1,
        };
        let acoustic = AcousticState {
            spectral_center: 0.5,
            resonance: 0.4,
            envelope: 0.6,
        };
        pm.record_voiced(motor, acoustic, 1_000_000);
        assert_eq!(pm.active_count(), 1);
        assert_eq!(pm.protos[0].rehearsal_count, 1);
    }

    #[test]
    fn nearby_motor_merges() {
        let mut pm = PhonemeMemory::new();
        let m1 = MotorState { pitch_tension: 0.5, resonance_pos: 0.4, noise_mix: 0.1 };
        let m2 = MotorState { pitch_tension: 0.52, resonance_pos: 0.38, noise_mix: 0.09 };
        let ac = AcousticState::default();
        pm.record_voiced(m1, ac, 1_000_000);
        pm.record_voiced(m2, ac, 2_000_000);
        assert_eq!(pm.active_count(), 1, "nearby motors should merge");
        assert_eq!(pm.protos[0].rehearsal_count, 2);
    }

    #[test]
    fn protos_mean_collapse_no_separation() {
        // PROBE (2026-06-22): two DISTINCT gestures within MATCH_RADIUS(0.25) do not
        // form two separated categories — they collapse into ONE proto whose
        // centroid drifts to their MEAN via EMA, with zero inter-proto repulsion.
        // Convergent (mean-seeking) dynamics, the mathematical opposite of
        // categorization: the memory cannot crystallize a discrete inventory, so
        // there is nothing discrete to sequence/reinforce. Confirms the binding
        // constraint alongside the combine() midpoint proof.
        let mut pm = PhonemeMemory::new();
        let a = MotorState { pitch_tension: 0.40, resonance_pos: 0.40, noise_mix: 0.0 };
        let b = MotorState { pitch_tension: 0.55, resonance_pos: 0.55, noise_mix: 0.0 }; // dist≈0.21 < 0.25
        let ac = AcousticState::default();
        for k in 0..200u64 {
            let m = if k % 2 == 0 { a } else { b };
            pm.record_voiced(m, ac, 1_000 + k * 1_000);
        }
        assert_eq!(
            pm.active_count(),
            1,
            "two within-radius gestures collapse into ONE proto, not two categories"
        );
        let cen = pm.protos.iter().find(|p| p.active).unwrap().motor.pitch_tension;
        let mean = 0.475; // (0.40 + 0.55) / 2 — expresses NEITHER gesture
        assert!(
            (cen - mean).abs() < 0.05,
            "the single proto sits at the MEAN {mean} of the two gestures (neither a=0.40 nor \
             b=0.55), got {cen} — mean-collapse, no separation"
        );
    }

    #[test]
    fn protos_repel_into_separated_basins() {
        // B2: distinct protos (>MATCH_RADIUS apart) actively REPEL — anti-Hebbian
        // push-apart — so the inventory forms SEPARATED basins instead of drifting
        // toward a shared mean. The divergent (categorizing) dynamic the mean-
        // seeking merge alone lacked. Inverts protos_mean_collapse_no_separation.
        let mut pm = PhonemeMemory::new();
        let a = MotorState { pitch_tension: 0.40, resonance_pos: 0.50, noise_mix: 0.0 };
        let b = MotorState { pitch_tension: 0.70, resonance_pos: 0.50, noise_mix: 0.0 }; // dist 0.30 > 0.25
        let ac = AcousticState::default();
        pm.record_voiced(a, ac, 1_000);
        pm.record_voiced(b, ac, 2_000);
        assert_eq!(pm.active_count(), 2, "two distinct gestures = two protos");
        let dist = |pm: &PhonemeMemory| {
            let ps: Vec<_> = pm.protos.iter().filter(|p| p.active).collect();
            ps[0].motor.dist(&ps[1].motor)
        };
        let d0 = dist(&pm);
        // Rehearse each cluster repeatedly; repulsion should push the basins apart.
        for k in 0..200u64 {
            let m = if k % 2 == 0 { a } else { b };
            pm.record_voiced(m, ac, 3_000 + k * 1_000);
        }
        assert_eq!(pm.active_count(), 2, "still two protos (repulsion does not spawn/merge)");
        let d1 = dist(&pm);
        assert!(
            d1 > d0 + 0.02,
            "protos must SEPARATE into distinct basins under repulsion: d0={d0:.3} → d1={d1:.3}"
        );
    }

    #[test]
    fn dense_cloud_expands_under_all_neighbor_repulsion() {
        // The live failure: many protos packed at ~0.1 spacing (a smear). All-
        // neighbor repulsion must EXPAND a packed cloud — pushing away from one
        // neighbor alone nets zero in a crowd, summing over all gives real spread.
        let mut pm = PhonemeMemory::new();
        // 4 protos in a tight square: side 0.26 (>MATCH_RADIUS 0.25 → no merge),
        // diagonal 0.37 (<REPEL_RADIUS 0.40 → all pairs repel).
        let pts = [(0.40, 0.40), (0.40, 0.66), (0.66, 0.40), (0.66, 0.66)];
        for &(p, r) in &pts {
            pm.record_voiced(ms(p, r, 0.5), ac(0.5, 0.5, 0.5), 1_000);
        }
        assert_eq!(pm.active_count(), 4, "4 distinct protos");
        let mean_pd = |pm: &PhonemeMemory| {
            let ps: Vec<_> = pm.protos.iter().filter(|p| p.active).collect();
            let (mut s, mut c) = (0.0, 0);
            for a in 0..ps.len() {
                for b in a + 1..ps.len() {
                    s += ps[a].motor.dist(&ps[b].motor);
                    c += 1;
                }
            }
            s / c as f64
        };
        let d0 = mean_pd(&pm);
        for k in 0..200u64 {
            let (p, r) = pts[(k as usize) % 4];
            pm.record_voiced(ms(p, r, 0.5), ac(0.5, 0.5, 0.5), 2_000 + k * 1_000);
        }
        let d1 = mean_pd(&pm);
        assert!(
            d1 > d0 + 0.03,
            "a packed cloud must EXPAND under all-neighbor repulsion: d0={d0:.3} → d1={d1:.3}"
        );
    }

    #[test]
    fn distant_motor_births_new_proto() {
        let mut pm = PhonemeMemory::new();
        let m1 = MotorState { pitch_tension: 0.1, resonance_pos: 0.1, noise_mix: 0.0 };
        let m2 = MotorState { pitch_tension: 0.9, resonance_pos: 0.9, noise_mix: 0.5 };
        let ac = AcousticState::default();
        pm.record_voiced(m1, ac, 1_000_000);
        pm.record_voiced(m2, ac, 2_000_000);
        assert_eq!(pm.active_count(), 2, "distant motors should birth separate protos");
    }

    #[test]
    fn social_response_attribution() {
        let mut pm = PhonemeMemory::new();
        let m = MotorState { pitch_tension: 0.5, resonance_pos: 0.4, noise_mix: 0.1 };
        pm.record_voiced(m, AcousticState::default(), 1_000_000);
        // External VIB arrives 1 second later with low self_match
        let result = pm.observe_vib(AcousticState::default(), 0.1, 2_000_000);
        assert!(result.is_some(), "should detect social response");
        let (idx, confidence) = result.unwrap();
        assert_eq!(idx, 0);
        assert!(confidence > 0.5, "confidence at 1s / 3s window = ~0.67");
        pm.credit_response(idx, confidence);
        assert!(pm.protos[0].social_response_count == 1);
    }

    #[test]
    fn no_attribution_for_self_echo() {
        let mut pm = PhonemeMemory::new();
        let m = MotorState { pitch_tension: 0.5, resonance_pos: 0.4, noise_mix: 0.1 };
        pm.record_voiced(m, AcousticState::default(), 1_000_000);
        // High self_match = own echo → no attribution
        let result = pm.observe_vib(AcousticState::default(), 0.9, 2_000_000);
        assert!(result.is_none(), "self-echo must not credit");
    }

    #[test]
    fn no_attribution_outside_window() {
        let mut pm = PhonemeMemory::new();
        let m = MotorState { pitch_tension: 0.5, resonance_pos: 0.4, noise_mix: 0.1 };
        pm.record_voiced(m, AcousticState::default(), 1_000_000);
        // VIB arrives 5 seconds later (beyond 3s window)
        let result = pm.observe_vib(AcousticState::default(), 0.1, 6_000_000);
        assert!(result.is_none(), "outside window = no attribution");
    }

    #[test]
    fn exploration_motor_pure_when_empty() {
        let mut pm = PhonemeMemory::new();
        let base = MotorState { pitch_tension: 0.5, resonance_pos: 0.5, noise_mix: 0.1 };
        let m = pm.exploration_motor(base);
        assert!(m.pitch_tension >= 0.0 && m.pitch_tension <= 1.0);
        assert!(m.resonance_pos >= 0.0 && m.resonance_pos <= 1.0);
    }

    #[test]
    fn exploration_motor_fine_when_stable() {
        let mut pm = PhonemeMemory::new();
        let motor = MotorState { pitch_tension: 0.3, resonance_pos: 0.7, noise_mix: 0.05 };
        let ac = AcousticState::default();
        // Rehearse enough to become stable
        for t in 0..PROTO_MIN_REHEARSALS {
            pm.record_voiced(motor, ac, (t + 1) * 1_000_000);
        }
        let base = MotorState::default();
        let m = pm.exploration_motor(base);
        // Should be close to the stable proto (±0.08 per dimension)
        assert!(
            (m.pitch_tension - motor.pitch_tension).abs() < 0.20,
            "fine exploration should stay near the stable proto"
        );
    }

    #[test]
    fn serialize_roundtrip() {
        let mut pm = PhonemeMemory::new();
        let m = MotorState { pitch_tension: 0.42, resonance_pos: 0.58, noise_mix: 0.11 };
        let ac = AcousticState { spectral_center: 0.3, resonance: 0.6, envelope: 0.7 };
        pm.record_voiced(m, ac, 5_000_000);
        pm.protos[0].social_response_count = 3;
        pm.protos[0].social_response_rate = 0.12;
        let bytes = pm.serialize();
        let pm2 = PhonemeMemory::deserialize(&bytes).expect("deserialize failed");
        let p0 = &pm2.protos[0];
        assert!(p0.active);
        assert!((p0.motor.pitch_tension - 0.42).abs() < 1e-10);
        assert!((p0.acoustic.envelope - 0.7).abs() < 1e-10);
        assert_eq!(p0.social_response_count, 3);
        assert!((p0.social_response_rate - 0.12).abs() < 1e-10);
    }
}
