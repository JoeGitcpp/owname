#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExternalPatternKind {
    VisualGlyph,
    AudiblePattern,
    GesturePattern,
    SymbolSequence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingTargetRef {
    ObjectSlot { slot_id: u8 },
    RelationEdge { edge_index: u16 },
    TimeWindow { start_t_us: u64, end_t_us: u64 },
}

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingStatus {
    Candidate,
    Reinforced,
    Weakened,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindingFeedback {
    Support,
    Contradict,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BindingEvent {
    pub pattern_kind: ExternalPatternKind,
    pub pattern_hash: u64,
    pub target: BindingTargetRef,
    pub feedback: BindingFeedback,
    pub evidence_ref: u64,
    pub event_t_us: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BindingTrace {
    pub pattern_kind: ExternalPatternKind,
    pub pattern_hash: u64,
    pub target: BindingTargetRef,
    pub strength: BindingStrength,
    pub status: BindingStatus,
    pub evidence_refs: Vec<u64>,
    pub reinforce_count: u32,
    pub weaken_count: u32,
    pub last_update_t_us: u64,
}

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
                BindingFeedback::Support => {
                    trace.reinforce_count += 1;
                    trace.strength = BindingStrength::new(trace.strength.value + 0.25);
                    trace.status = BindingStatus::Reinforced;
                }
                BindingFeedback::Contradict => {
                    trace.weaken_count += 1;
                    trace.strength = BindingStrength::new(trace.strength.value - 0.3);
                    trace.status = BindingStatus::Weakened;
                }
            }
            self.n_bindings = self.traces.len();
            return;
        }

        if event.feedback == BindingFeedback::Support {
            self.push_trace(BindingTrace {
                pattern_kind: event.pattern_kind,
                pattern_hash: event.pattern_hash,
                target: event.target,
                strength: BindingStrength::new(0.25),
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
