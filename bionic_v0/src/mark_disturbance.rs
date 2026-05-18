use crate::language_binding::{BindingEvent, BindingFeedback, BindingTargetRef, PatternKind};
use crate::viewport::Viewport;

pub const MARK_CARRIER_BYTE: u8 = 0;
const MARK_TRACE_CAP: usize = 64;
const MARK_EVIDENCE_CAP: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct MarkTrace {
    pub carrier_kind_code: u8,
    pub stream_context_id: u64,
    pub mark_hash: u64,
    pub first_t_us: u64,
    pub last_t_us: u64,
    pub recurrence_count: u32,
    pub stability: f64,
    pub ambiguity: f64,
    pub evidence_refs: Vec<u64>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkDisturbanceState {
    pub n_traces: u32,
    pub traces: Vec<MarkTrace>,
    pub latest_mark_hash: u64,
    pub last_update_t_us: u64,
}

impl MarkDisturbanceState {
    pub fn observe_bytes(
        &mut self,
        stream_context_id: u64,
        t_start_us: u64,
        t_end_us: u64,
        bytes: &[u8],
        framing_validity: f64,
    ) -> u64 {
        let mark_hash = hash_bytes(bytes);
        let validity = unit(framing_validity);
        let evidence_ref = nonzero_ref(mark_hash ^ t_end_us.rotate_left(13), t_end_us);

        if let Some(trace) = self.traces.iter_mut().find(|trace| {
            trace.mark_hash == mark_hash && trace.carrier_kind_code == MARK_CARRIER_BYTE
        }) {
            trace.last_t_us = t_end_us;
            trace.recurrence_count = trace.recurrence_count.saturating_add(1);
            trace.stability = (trace.stability + 0.25 + validity * 0.25).clamp(0.0, 1.0);
            trace.ambiguity = (trace.ambiguity * 0.85 + (1.0 - validity) * 0.10).clamp(0.0, 1.0);
            push_bounded(&mut trace.evidence_refs, evidence_ref, MARK_EVIDENCE_CAP);
            self.latest_mark_hash = mark_hash;
            self.last_update_t_us = t_end_us;
            return mark_hash;
        }

        self.traces.push(MarkTrace {
            carrier_kind_code: MARK_CARRIER_BYTE,
            stream_context_id,
            mark_hash,
            first_t_us: t_start_us,
            last_t_us: t_end_us,
            recurrence_count: 1,
            stability: (0.25 + validity * 0.25).clamp(0.0, 1.0),
            ambiguity: (1.0 - validity).clamp(0.0, 1.0) * 0.25,
            evidence_refs: vec![evidence_ref],
        });
        if self.traces.len() > MARK_TRACE_CAP {
            let evict_n = self.traces.len() - MARK_TRACE_CAP;
            self.traces.drain(0..evict_n);
        }
        self.n_traces = self.traces.len() as u32;
        self.latest_mark_hash = mark_hash;
        self.last_update_t_us = t_end_us;
        mark_hash
    }

    pub fn record_feedback(&mut self, mark_hash: u64, feedback: BindingFeedback, t_us: u64) {
        if let Some(trace) = self
            .traces
            .iter_mut()
            .find(|trace| trace.mark_hash == mark_hash)
        {
            trace.last_t_us = t_us;
            match feedback {
                BindingFeedback::Align => {
                    trace.stability = (trace.stability + 0.10).clamp(0.0, 1.0);
                    trace.ambiguity = (trace.ambiguity * 0.80).clamp(0.0, 1.0);
                }
                BindingFeedback::Deviate => {
                    trace.ambiguity = (trace.ambiguity + 0.25).clamp(0.0, 1.0);
                    trace.stability = (trace.stability * 0.92).clamp(0.0, 1.0);
                }
            }
            self.last_update_t_us = t_us;
        }
    }
}

impl Viewport {
    pub fn read_mark_disturbance(&self) -> MarkDisturbanceState {
        self.mark_disturbance.read().unwrap().clone()
    }

    pub fn observe_mark_bytes(
        &self,
        stream_context_id: u64,
        t_start_us: u64,
        t_end_us: u64,
        bytes: &[u8],
        framing_validity: f64,
    ) -> u64 {
        self.mark_disturbance.write().unwrap().observe_bytes(
            stream_context_id,
            t_start_us,
            t_end_us,
            bytes,
            framing_validity,
        )
    }

    pub fn bind_latest_mark_to_target(
        &self,
        target: BindingTargetRef,
        feedback: BindingFeedback,
        event_t_us: u64,
    ) -> bool {
        let mark_hash = self.mark_disturbance.read().unwrap().latest_mark_hash;
        if mark_hash == 0 || !mark_binding_target_is_live(self, target) {
            return false;
        }

        self.apply_language_binding_event(BindingEvent {
            pattern_kind: PatternKind::SymbolSeq,
            pattern_hash: mark_hash,
            target,
            feedback,
            evidence_ref: nonzero_ref(mark_hash ^ event_t_us.rotate_left(7), event_t_us),
            event_t_us,
        });
        self.mark_disturbance
            .write()
            .unwrap()
            .record_feedback(mark_hash, feedback, event_t_us);
        true
    }
}

fn mark_binding_target_is_live(vp: &Viewport, target: BindingTargetRef) -> bool {
    match target {
        BindingTargetRef::ObjectSlot { slot_id } => vp
            .read_slot(slot_id as usize)
            .map(|slot| slot.active)
            .unwrap_or(false),
        BindingTargetRef::RelationEdge { edge_index } => vp
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

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    nonzero_ref(h, bytes.len() as u64)
}

fn nonzero_ref(value: u64, fallback: u64) -> u64 {
    if value == 0 {
        fallback.max(1)
    } else {
        value
    }
}

fn push_bounded<T>(items: &mut Vec<T>, item: T, cap: usize) {
    if items.len() == cap {
        items.remove(0);
    }
    items.push(item);
}

fn unit(x: f64) -> f64 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}
