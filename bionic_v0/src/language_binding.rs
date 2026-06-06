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
            if !trace.evidence_refs.contains(&event.evidence_ref) {
                trace.evidence_refs.push(event.evidence_ref);
            }
            trace.last_update_t_us = event.event_t_us;
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
        if feedback == BindingFeedback::Align {
            self.advance_maturity(pair.curve_similarity);
        }
        true
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
