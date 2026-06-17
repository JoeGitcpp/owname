// Language binding layer — binds external patterns to internal substrate.
// Numeric-only binding traces; no substrate promotion threshold.
// Binding events are the learning unit.

/// External pattern kind codes. Numeric indices, not semantic labels.
/// 0=VibModality, 1=PhotModality, 2=MechModality, 3=SymbolSeq.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PatternKind {
    VibModality = 0,
    PhotModality = 1,
    MechModality = 2,
    SymbolSeq = 3,
}

/// Binding target reference. Numeric index.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingTargetRef {
    ObjectSlot { slot_id: u8 },
    RelationEdge { edge_index: u16 },
    TimeWindow { start_t_us: u64, end_t_us: u64 },
}

/// Binding strength [0, 1].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BindingStrength {
    pub value: f64,
}

impl BindingStrength {
    pub fn new(value: f64) -> Self {
        Self {
            value: value.clamp(0.0, 1.0),
        }
    }
}

impl Default for BindingStrength {
    fn default() -> Self {
        Self::new(0.0)
    }
}

/// Binding status codes. Numeric.
/// 0=Candidate, 1=Reinforced, 2=Weakened.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingStatus {
    Candidate = 0,
    Reinforced = 1,
    Weakened = 2,
}

/// Binding feedback codes. Numeric.
/// 0=Align, 1=Deviate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingFeedback {
    Align = 0,
    Deviate = 1,
}

/// Binding event — the learning unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BindingEvent {
    pub pattern_kind: PatternKind,
    pub pattern_hash: u64,
    pub target: BindingTargetRef,
    pub feedback: BindingFeedback,
    pub evidence_ref: u64,
    pub event_t_us: u64,
}

/// Binding trace — accumulated evidence for one binding.
#[derive(Clone, Debug, PartialEq)]
pub struct BindingTrace {
    pub pattern_kind: PatternKind,
    pub pattern_hash: u64,
    pub target: BindingTargetRef,
    pub strength: BindingStrength,
    pub status: BindingStatus,
    pub evidence_refs: Vec<u64>,
    pub reinforce_count: u32,
    pub weaken_count: u32,
    pub last_update_t_us: u64,
}

/// Language binding state — collection of traces.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LanguageBindingState {
    pub traces: Vec<BindingTrace>,
    pub n_bindings: usize,
}

impl LanguageBindingState {
    pub fn push_trace(&mut self, trace: BindingTrace) {
        self.traces.push(trace);
        self.n_bindings = self.traces.len();
    }

    pub fn apply_binding_event(&mut self, event: BindingEvent) {
        self.apply_binding_event_with_pressure(event, 0.0);
    }

    pub fn apply_binding_event_with_pressure(&mut self, event: BindingEvent, update_pressure: f64) {
        let update_scale = 1.0 + 0.25 * update_pressure.clamp(0.0, 2.0);
        if let Some(trace) = self.traces.iter_mut().find(|trace| {
            trace.pattern_kind == event.pattern_kind
                && trace.pattern_hash == event.pattern_hash
                && trace.target == event.target
        }) {
            // Only DISTINCT evidence moves strength/count. The dedup formerly
            // guarded ONLY the evidence_refs push while strength/reinforce_count
            // were bumped unconditionally — so a caller re-presenting the SAME
            // evidence_ref (e.g. the live co-occurrence tick re-finding the same
            // Paired pair every 5 Hz) inflated ONE earned event into hundreds of
            // fabricated reinforcements (reinforce_count ≫ evidence_refs.len()).
            // Re-presenting the same evidence now only refreshes the idle clock.
            trace.last_update_t_us = event.event_t_us;
            if !trace.evidence_refs.contains(&event.evidence_ref) {
                trace.evidence_refs.push(event.evidence_ref);
                match event.feedback {
                    BindingFeedback::Align => {
                        trace.reinforce_count += 1;
                        trace.strength =
                            BindingStrength::new(trace.strength.value + 0.25 * update_scale);
                        trace.status = BindingStatus::Reinforced;
                    }
                    BindingFeedback::Deviate => {
                        trace.weaken_count += 1;
                        trace.strength =
                            BindingStrength::new(trace.strength.value - 0.3 * update_scale);
                        trace.status = BindingStatus::Weakened;
                    }
                }
            }
            self.n_bindings = self.traces.len();
            return;
        }

        if event.feedback == BindingFeedback::Align {
            self.push_trace(BindingTrace {
                pattern_kind: event.pattern_kind,
                pattern_hash: event.pattern_hash,
                target: event.target,
                strength: BindingStrength::new(0.25 * update_scale),
                status: BindingStatus::Candidate,
                evidence_refs: vec![event.evidence_ref],
                reinforce_count: 0,
                weaken_count: 0,
                last_update_t_us: event.event_t_us,
            });
        }
    }

    pub fn tick_maintenance(&mut self, now_us: u64, idle_window_us: u64, decay_step: f64) {
        if self.traces.is_empty() {
            self.n_bindings = 0;
            return;
        }

        let mut decayed = Vec::with_capacity(self.traces.len());
        for mut trace in self.traces.drain(..) {
            if now_us.saturating_sub(trace.last_update_t_us) >= idle_window_us {
                trace.strength = BindingStrength::new(trace.strength.value - decay_step);
                trace.status = BindingStatus::Weakened;
            }
            if trace.strength.value > 0.0 {
                decayed.push(trace);
            }
        }
        self.traces = decayed;
        self.n_bindings = self.traces.len();
    }
}

/// Stable numeric fingerprint of a heard VIB band contour, for live symbol
/// grounding. Bands are quantized at 1/32 before folding so jitter in a repeated
/// sound maps to the same hash (one trace reinforces). FNV-1a fold; never returns
/// 0 (0 is the "no sound heard" sentinel in tick_form_binding_from_cooccurrence).
/// Mirrors mark_disturbance::hash_bytes for the byte-mark path.
fn hash_vib_contour(observed: &[f64]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for band in observed {
        let q = (band * 32.0).round().clamp(-1.0e9, 1.0e9) as i64;
        h ^= q as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    if h == 0 {
        1
    } else {
        h
    }
}

/// Stable numeric fingerprint of a SEEN visual action, for cross-modal binding
/// (AV-action: heard sound × seen action → "this sound came from that action").
/// Folds the 4 multiscale motion RMS bands (slot.phot.observed; see phot_camera::
/// region_to_extract) plus the slot's spatial pose (centroid + span) and the
/// worldline velocity polar (speed magnitude + 8-way direction bucket). The bands
/// are quantized at PHOT_ACTION_BAND_QUANT, position/span at PHOT_ACTION_POSE_QUANT,
/// speed at PHOT_ACTION_SPEED_QUANT — coarse enough so the SAME gesture re-hashes,
/// fine enough to distinguish gestures of different speeds/directions. FNV-1a fold,
/// never returns 0 (0 is the "no action seen" sentinel in
/// tick_form_av_action_binding). Mirrors hash_vib_contour for the sound side.
pub(crate) fn hash_phot_motion_contour(
    observed: &[f64],
    pos_x: f64,
    pos_y: f64,
    span_x: f64,
    span_y: f64,
    velocity_x_per_s: f64,
    velocity_y_per_s: f64,
) -> u64 {
    const PHOT_ACTION_BAND_QUANT: f64 = 16.0;
    const PHOT_ACTION_POSE_QUANT: f64 = 16.0;
    const PHOT_ACTION_SPEED_QUANT: f64 = 8.0;
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for band in observed {
        let q = (band * PHOT_ACTION_BAND_QUANT).round().clamp(-1.0e9, 1.0e9) as i64;
        h ^= q as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let qx = (pos_x * PHOT_ACTION_POSE_QUANT).round().clamp(-1.0e9, 1.0e9) as i64;
    let qy = (pos_y * PHOT_ACTION_POSE_QUANT).round().clamp(-1.0e9, 1.0e9) as i64;
    let qsx = (span_x * PHOT_ACTION_POSE_QUANT).round().clamp(-1.0e9, 1.0e9) as i64;
    let qsy = (span_y * PHOT_ACTION_POSE_QUANT).round().clamp(-1.0e9, 1.0e9) as i64;
    for q in [qx, qy, qsx, qsy] {
        h ^= q as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    let speed = (velocity_x_per_s * velocity_x_per_s + velocity_y_per_s * velocity_y_per_s).sqrt();
    let q_speed = (speed * PHOT_ACTION_SPEED_QUANT).round().clamp(-1.0e9, 1.0e9) as i64;
    h ^= q_speed as u64;
    h = h.wrapping_mul(0x0000_0100_0000_01b3);
    // 8-way direction bucket (only meaningful above a tiny floor; below it, 0).
    let dir_bucket: u64 = if speed > 1.0e-6 {
        let ang = velocity_y_per_s.atan2(velocity_x_per_s); // [-pi, pi]
        let frac = (ang + std::f64::consts::PI) / (2.0 * std::f64::consts::PI); // [0,1)
        ((frac * 8.0).floor() as i64).rem_euclid(8) as u64
    } else {
        0
    };
    h ^= dir_bucket;
    h = h.wrapping_mul(0x0000_0100_0000_01b3);
    if h == 0 {
        1
    } else {
        h
    }
}

impl LanguageBindingState {
    /// Cross-modal lookup: given a pattern just heard/seen, find the strongest
    /// recorded binding for it. The strength field already ranks evidence, so
    /// "the sound that has co-occurred most often with X" wins. Returns None when
    /// no trace exists — the body has never paired this pattern with anything.
    pub fn lookup_binding(
        &self,
        pattern_kind: PatternKind,
        pattern_hash: u64,
    ) -> Option<BindingTargetRef> {
        self.traces
            .iter()
            .filter(|t| t.pattern_kind == pattern_kind && t.pattern_hash == pattern_hash)
            .max_by(|a, b| a.strength.value.partial_cmp(&b.strength.value).unwrap())
            .map(|t| t.target)
    }

    /// Reverse cross-modal lookup: given an event target (e.g. an ObjectSlot the
    /// body is currently attending), enumerate every pattern that has been bound
    /// to it, with its modality kind, hash, and strength. Used to recover "what
    /// sound have we heard with this seen action" given just the slot — i.e. the
    /// production-side cross-modal hypothesis: SLOT → (sound_hash, action_hash).
    pub fn lookup_associated_patterns(
        &self,
        target: BindingTargetRef,
    ) -> Vec<(PatternKind, u64, f64)> {
        self.traces
            .iter()
            .filter(|t| t.target == target)
            .map(|t| (t.pattern_kind, t.pattern_hash, t.strength.value))
            .collect()
    }
}

impl crate::viewport::Viewport {
    pub fn try_form_binding_event_from_pair(
        &self,
        pattern_kind: PatternKind,
        pattern_hash: u64,
        target: BindingTargetRef,
        write_seq: u64,
        window_id: u64,
    ) -> bool {
        let pair = {
            let ac = self.action_consequence.read().unwrap();
            ac.pairs
                .iter()
                .find(|pair| {
                    pair.write_seq == write_seq
                        && pair.window_id == window_id
                        && pair.status == crate::action_consequence::ConsequencePairStatus::Paired
                })
                .cloned()
        };

        let Some(pair) = pair else {
            return false;
        };

        // Developmental precision: the align/deviate bar is loose for a naive
        // body (no dead zone at m=0, so a rough match still gives graded credit)
        // and tightens to the strict thresholds as the developmental clock
        // matures. Breaks the bootstrap deadlock without sacrificing mature
        // discrimination.
        let maturity = self.developmental_maturity();
        let align_min = crate::binding_align_min(maturity);
        let deviate_max = crate::binding_deviate_max(maturity);
        let feedback = if pair.curve_similarity >= align_min {
            BindingFeedback::Align
        } else if pair.curve_similarity <= deviate_max {
            BindingFeedback::Deviate
        } else {
            return false;
        };

        if !self.binding_target_is_live(target) {
            return false;
        }

        if feedback == BindingFeedback::Deviate
            && !self.binding_trace_exists(pattern_kind, pattern_hash, target)
        {
            return false;
        }

        self.apply_language_binding_event(BindingEvent {
            pattern_kind,
            pattern_hash,
            target,
            feedback,
            evidence_ref: write_seq.rotate_left(32) ^ window_id,
            event_t_us: pair.pair_t_us,
        });
        // Developmental clock: a confirmed Align from real pair evidence is
        // earned competence — advance maturity so precision tightens over time.
        // Throttled to MATURITY_ADVANCE_MIN_INTERVAL_US so the binding path
        // (like the vocal path) cannot saturate maturity faster than 1/second.
        if feedback == BindingFeedback::Align {
            let last = self.last_maturity_advance_us.load(std::sync::atomic::Ordering::Relaxed);
            if pair.pair_t_us.saturating_sub(last) >= crate::MATURITY_ADVANCE_MIN_INTERVAL_US {
                self.last_maturity_advance_us
                    .store(pair.pair_t_us, std::sync::atomic::Ordering::Relaxed);
                self.advance_maturity(pair.curve_similarity);
            }
        }
        true
    }

    /// LIVE symbol grounding (#11): when the body genuinely HEARS a sound (a
    /// fresh active VIB slot) WHILE SEEING an object (a fresh active PHOT slot)
    /// AND a real, recent earned action-consequence Paired pair exists, form/
    /// reinforce a (heard-sound → seen-object) binding. Reuses
    /// try_form_binding_event_from_pair so the maturity-gated align/deviate
    /// precision, the live-target check, and the no-spurious-deviate rule all
    /// apply unchanged. This is the production counterpart to the curriculum-only
    /// formation calls — without it the deployed body could ONLY decay bindings,
    /// never form one (symbol grounding was structurally dead).
    ///
    /// Honesty gates (all reuse existing mechanisms; nothing forms from nothing):
    ///   • Darkness → no active PHOT slot → return.
    ///   • Silence → no active VIB slot / empty contour → return.
    ///   • No coherent motor/consequence state → no recent Paired pair → return.
    ///   • One earned pair grounds AT MOST ONE binding event (idempotence below),
    ///     so a sustained stimulus cannot inflate one event into a per-tick
    ///     runaway of fabricated reinforcements.
    pub fn tick_form_binding_from_cooccurrence(&self, now_us: u64) -> bool {
        // Seen object: a fresh, active PHOT slot.
        let Some(phot_slot) = self.find_active_slot_for(crate::MOD_PHOT, now_us) else {
            return false;
        };
        let target = BindingTargetRef::ObjectSlot {
            slot_id: phot_slot as u8,
        };

        // Heard sound: a fresh, active VIB slot carrying real band evidence. The
        // quantized observed contour is a stable fingerprint of the sound, so the
        // same sound reinforces the same trace instead of spawning new ones.
        let mut pattern_hash = 0u64;
        for sid in 0..crate::N_OBJECT_SLOTS {
            if let Some(slot) = self.read_slot(sid) {
                if slot.active
                    && (slot.source_modality & crate::MOD_VIB) != 0
                    && now_us.saturating_sub(slot.last_update_t_us) <= crate::SLOT_IDLE_US
                {
                    if let Some(ref vib) = slot.vib {
                        let h = hash_vib_contour(&vib.observed);
                        if h != 0 {
                            pattern_hash = h;
                            break;
                        }
                    }
                }
            }
        }
        if pattern_hash == 0 {
            return false;
        }

        // Earned, CONTEMPORANEOUS evidence: the latest action-consequence Paired
        // pair, only if recent (within TEMPORAL_PAIR_MAX_DELAY_US of now) so the
        // binding grounds on a CURRENT consequence, not an arbitrarily stale one.
        let (write_seq, window_id) = {
            let ac = self.read_action_consequence();
            match ac.pairs.iter().rev().find(|p| {
                p.status == crate::action_consequence::ConsequencePairStatus::Paired
                    && now_us.saturating_sub(p.pair_t_us) <= crate::TEMPORAL_PAIR_MAX_DELAY_US
            }) {
                Some(p) => (p.write_seq, p.window_id),
                None => return false,
            }
        };

        // Per-pair idempotence: a single earned pair grounds AT MOST ONE binding
        // event. Without this the same still-Paired pair re-fires every 5 Hz tick
        // while the stimulus persists, repeatedly advancing maturity and (absent
        // the source-level evidence_ref dedup) inflating strength from one event.
        let handle = write_seq.rotate_left(32) ^ window_id;
        if self
            .last_binding_pair_consumed
            .load(std::sync::atomic::Ordering::Relaxed)
            == handle
        {
            return false;
        }
        self.last_binding_pair_consumed
            .store(handle, std::sync::atomic::Ordering::Relaxed);

        self.try_form_binding_event_from_pair(
            PatternKind::VibModality,
            pattern_hash,
            target,
            write_seq,
            window_id,
        )
    }

    /// Cross-modal AV-action grounding: when a single ObjectSlot carries BOTH
    /// active VIB *and* active PHOT evidence simultaneously (the live cross-modal
    /// fusion machinery — see CROSS_MODAL_WINDOW_US / claim_slot_for_event —
    /// already places them on one slot when a sound and a sight refer to the
    /// same event), AND a real, recent Paired pair earned this co-occurrence,
    /// form a symmetric (seen-action → object-slot) binding. The companion
    /// (heard-sound → object-slot) binding is the production-side responsibility
    /// of tick_form_binding_from_cooccurrence; this method writes the PHOT
    /// counterpart so the slot becomes a rendezvous: both modalities' patterns
    /// look up to the same object, and lookup_associated_patterns recovers the
    /// other modality's hash — the "compressed direct understanding" path on
    /// re-encounter.
    ///
    /// Honesty gates (parallel to tick_form_binding_from_cooccurrence):
    ///   • No slot carrying BOTH VIB and PHOT freshness → no co-located evidence
    ///     of a single AV event → return.
    ///   • PHOT energy below PHOT_AV_ACTION_MIN_MOTION → no salient action seen,
    ///     so a learned binding wouldn't generalise (every quiet frame would
    ///     ground a vacuous "action" hash) → return.
    ///   • No recent Paired pair within TEMPORAL_PAIR_MAX_DELAY_US → return.
    ///   • Per-pair idempotence (last_av_action_pair_consumed): a single earned
    ///     pair grounds AT MOST ONE AV-action binding event, mirroring the VIB
    ///     side. No double-counting across the 5 Hz refresh.
    pub fn tick_form_av_action_binding(&self, now_us: u64) -> bool {
        const PHOT_AV_ACTION_MIN_MOTION: f64 = 0.05;

        // Walk slots once and find one with BOTH VIB and PHOT active+fresh.
        let mut chosen: Option<(usize, u64, u64)> = None; // (slot_id, vib_hash, phot_action_hash)
        for sid in 0..crate::N_OBJECT_SLOTS {
            let Some(slot) = self.read_slot(sid) else {
                continue;
            };
            if !slot.active {
                continue;
            }
            if now_us.saturating_sub(slot.last_update_t_us) > crate::SLOT_IDLE_US {
                continue;
            }
            let has_vib = (slot.source_modality & crate::MOD_VIB) != 0;
            let has_phot = (slot.source_modality & crate::MOD_PHOT) != 0;
            if !(has_vib && has_phot) {
                continue;
            }
            let Some(vib) = slot.vib.as_ref() else { continue };
            let Some(phot) = slot.phot.as_ref() else { continue };
            if phot.observed.is_empty() {
                continue;
            }
            let phot_energy: f64 =
                phot.observed.iter().sum::<f64>() / phot.observed.len() as f64;
            if phot_energy < PHOT_AV_ACTION_MIN_MOTION {
                continue;
            }
            let vib_hash = hash_vib_contour(&vib.observed);
            if vib_hash == 0 {
                continue;
            }
            let (pos_x, pos_y, span_x, span_y) = slot
                .spatial
                .as_ref()
                .map(|s| (s.pos_x, s.pos_y, s.span_x, s.span_y))
                .unwrap_or((0.5, 0.5, 0.0, 0.0));
            let (vx, vy) = slot
                .spatial_worldline
                .as_ref()
                .map(|w| (w.velocity_x_per_s, w.velocity_y_per_s))
                .unwrap_or((0.0, 0.0));
            let phot_hash = hash_phot_motion_contour(
                &phot.observed,
                pos_x,
                pos_y,
                span_x,
                span_y,
                vx,
                vy,
            );
            if phot_hash == 0 {
                continue;
            }
            chosen = Some((sid, vib_hash, phot_hash));
            break;
        }

        let Some((slot_id, _vib_hash, phot_hash)) = chosen else {
            return false;
        };
        let target = BindingTargetRef::ObjectSlot {
            slot_id: slot_id as u8,
        };

        // Earned, contemporaneous evidence: latest Paired pair within the temporal
        // window. Identical gate to tick_form_binding_from_cooccurrence so the two
        // ticks ground on the SAME pair-class evidence (each guarded by its own
        // idempotence atomic so neither blocks the other).
        let (write_seq, window_id) = {
            let ac = self.read_action_consequence();
            match ac.pairs.iter().rev().find(|p| {
                p.status == crate::action_consequence::ConsequencePairStatus::Paired
                    && now_us.saturating_sub(p.pair_t_us) <= crate::TEMPORAL_PAIR_MAX_DELAY_US
            }) {
                Some(p) => (p.write_seq, p.window_id),
                None => return false,
            }
        };

        // Per-pair idempotence for the AV-action side (independent atomic from
        // last_binding_pair_consumed so the VIB side and the PHOT side can each
        // ground exactly once per earned pair).
        let handle = write_seq.rotate_left(32) ^ window_id;
        if self
            .last_av_action_pair_consumed
            .load(std::sync::atomic::Ordering::Relaxed)
            == handle
        {
            return false;
        }
        self.last_av_action_pair_consumed
            .store(handle, std::sync::atomic::Ordering::Relaxed);

        self.try_form_binding_event_from_pair(
            PatternKind::PhotModality,
            phot_hash,
            target,
            write_seq,
            window_id,
        )
    }

    fn binding_target_is_live(&self, target: BindingTargetRef) -> bool {
        match target {
            BindingTargetRef::ObjectSlot { slot_id } => self
                .read_slot(slot_id as usize)
                .map(|slot| slot.active)
                .unwrap_or(false),
            BindingTargetRef::RelationEdge { edge_index } => self
                .global_relations
                .read()
                .unwrap()
                .edges
                .get(edge_index as usize)
                .map(|edge| edge.active)
                .unwrap_or(false),
            BindingTargetRef::TimeWindow {
                start_t_us,
                end_t_us,
            } => start_t_us < end_t_us,
        }
    }

    fn binding_trace_exists(
        &self,
        pattern_kind: PatternKind,
        pattern_hash: u64,
        target: BindingTargetRef,
    ) -> bool {
        self.language_binding
            .read()
            .unwrap()
            .traces
            .iter()
            .any(|trace| {
                trace.pattern_kind == pattern_kind
                    && trace.pattern_hash == pattern_hash
                    && trace.target == target
            })
    }
}
