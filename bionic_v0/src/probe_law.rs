// Probe-law substrate (minimal active-probe entry).
//
// Numeric/time-only. Forbidden: named object-target fields, outcome-label
// fields, semantic-gate fields, concept identifiers, or naming qualifiers.
// No high-layer semantics. No fixed curiosity/timing
// gates — phase transitions driven by intensity-vs-suppression ratios.

use crate::action_output::{
    ActionOutputEvent, ActionOutputFeedback, ActionOutputPhase, OutputChannelKind,
};
use crate::viewport::Viewport;
use crate::{
    PROBE_HISTORY_CAP, PROBE_INTENSITY_ALPHA, PROBE_SUPPRESSION_ALPHA,
    RELATION_INSTABILITY_TRIGGER_THRESHOLD, SURPRISE_TRIGGER_THRESHOLD,
};

/// Probe lifecycle phase. Numeric code; no semantic label.
/// 0=Idle, 1=Rising, 2=Active, 3=Decaying.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum ProbePhase {
    #[default]
    Idle = 0,
    Rising = 1,
    Active = 2,
    Decaying = 3,
}

impl ProbePhase {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => ProbePhase::Rising,
            2 => ProbePhase::Active,
            3 => ProbePhase::Decaying,
            _ => ProbePhase::Idle,
        }
    }
}

/// Internal state of the probe-law substrate.
///
/// All fields are numeric or temporal. No target, result, semantic, concept,
/// or naming-qualifier fields are permitted in this struct.
#[derive(Clone, Debug)]
pub struct ProbeLawState {
    /// EMA-smoothed probe intensity. Driven by global reasoning novelty. [0,1].
    pub intensity: f64,
    /// Slower EMA of intensity; acts as adaptive suppression baseline. [0,1].
    pub suppression: f64,
    /// Current lifecycle phase code.
    pub phase: ProbePhase,
    /// Substrate time of last refresh (µs).
    pub last_update_t_us: u64,
    /// Output intensity produced by tick_probe_output_from_law. [0,1].
    pub output_intensity: f64,
    /// Short ring of recent intensity values. Length = PROBE_HISTORY_CAP.
    pub history: [f64; PROBE_HISTORY_CAP],
    pub history_head: usize,
    pub history_len: usize,
    /// Last observed top routing slot id; used by tick_attention_trigger to
    /// detect top-slot switches. -1 = uninitialized (no prior observation).
    /// Ephemeral: not persisted in snapshots (cold-start resets to -1).
    pub last_top_slot_id: i8,
}

impl Default for ProbeLawState {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            suppression: 0.0,
            phase: ProbePhase::Idle,
            last_update_t_us: 0,
            output_intensity: 0.0,
            history: [0.0; PROBE_HISTORY_CAP],
            history_head: 0,
            history_len: 0,
            last_top_slot_id: -1,
        }
    }
}

impl Viewport {
    /// Drive probe-law state from current global reasoning novelty (EMA-based).
    ///
    /// Phase transitions are ratio-based: no fixed absolute gates on curiosity,
    /// timing, or delay. Engineering alphas live in lib.rs constants.
    pub fn refresh_probe_law(&self, now_us: u64) {
        let novelty = self.read_global_reasoning_memory_summary().novelty_score;

        let mut st = self.probe_law.write().unwrap();

        // EMA update: intensity tracks novelty; suppression tracks intensity (slower).
        st.intensity =
            (1.0 - PROBE_INTENSITY_ALPHA) * st.intensity + PROBE_INTENSITY_ALPHA * novelty;
        st.suppression = (1.0 - PROBE_SUPPRESSION_ALPHA) * st.suppression
            + PROBE_SUPPRESSION_ALPHA * st.intensity;

        // Phase transitions driven by intensity/suppression ratio — no fixed gates.
        let ratio = if st.suppression > 1e-9 {
            st.intensity / st.suppression
        } else if st.intensity > 1e-9 {
            2.0
        } else {
            1.0
        };
        st.phase = match st.phase {
            ProbePhase::Idle => {
                if ratio > 1.0 {
                    ProbePhase::Rising
                } else {
                    ProbePhase::Idle
                }
            }
            ProbePhase::Rising => {
                if ratio > 1.05 {
                    ProbePhase::Active
                } else if ratio < 1.0 {
                    ProbePhase::Decaying
                } else {
                    ProbePhase::Rising
                }
            }
            ProbePhase::Active => {
                if ratio < 1.0 {
                    ProbePhase::Decaying
                } else {
                    ProbePhase::Active
                }
            }
            ProbePhase::Decaying => {
                if ratio > 1.0 {
                    ProbePhase::Rising
                } else if st.intensity < st.suppression * 0.1 {
                    ProbePhase::Idle
                } else {
                    ProbePhase::Decaying
                }
            }
        };

        // Append to short history ring.
        let head = st.history_head;
        st.history[head] = st.intensity;
        st.history_head = (head + 1) % PROBE_HISTORY_CAP;
        if st.history_len < PROBE_HISTORY_CAP {
            st.history_len += 1;
        }

        st.last_update_t_us = now_us;
    }

    /// Produce tick output from current probe-law state and drive action_output events.
    ///
    /// Mapping (idempotent — channel phase + probe phase → event):
    ///   Channel Idle (or no channel) + probe Rising/Active → Begin
    ///   Channel Active + probe Rising/Active → Adjust
    ///   Channel Decaying + probe Rising/Active → Begin (new epoch overrides decay)
    ///   Channel Active + probe Decaying → Decay
    ///   Channel Decaying + probe Decaying → Decay
    ///   Channel Active/Decaying + probe Idle → Stop
    ///   Channel Idle + probe Idle → None
    ///   Channel Idle + probe Decaying → None (no active channel to decay)
    ///
    /// Lock ordering: read probe_law (drop), read action_output channel (drop),
    /// fire event (write lock inside apply_action_output_event), then update
    /// output_intensity in probe_law write lock.
    pub fn tick_probe_output_from_law(&self, now_us: u64) {
        // Step 1: read probe_law state (drop read lock).
        let (probe_phase, intensity, current_output_intensity) = {
            let st = self.probe_law.read().unwrap();
            (st.phase, st.intensity, st.output_intensity)
        };

        // Step 2: read action_output channel phase (drop read lock).
        let channel_phase = {
            let ao = self.action_output.read().unwrap();
            ao.channels
                .iter()
                .find(|c| c.channel_kind == OutputChannelKind::GenericWrite)
                .map(|c| c.phase)
        };

        // Step 3: compute event to fire based on channel phase + probe phase.
        let decay_magnitude = (current_output_intensity * PROBE_SUPPRESSION_ALPHA).max(1e-9);

        let maybe_event: Option<ActionOutputEvent> = match (channel_phase, probe_phase) {
            // No channel or Idle channel + Rising/Active → Begin
            (None, ProbePhase::Rising | ProbePhase::Active)
            | (Some(ActionOutputPhase::Idle), ProbePhase::Rising | ProbePhase::Active) => {
                Some(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Begin,
                    magnitude: intensity,
                    event_t_us: now_us,
                })
            }
            // Active channel + Rising/Active → Adjust
            (Some(ActionOutputPhase::Active), ProbePhase::Rising | ProbePhase::Active) => {
                Some(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Adjust,
                    magnitude: intensity,
                    event_t_us: now_us,
                })
            }
            // Decaying channel + Rising/Active → Begin (rising overrides decaying)
            (Some(ActionOutputPhase::Decaying), ProbePhase::Rising | ProbePhase::Active) => {
                Some(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Begin,
                    magnitude: intensity,
                    event_t_us: now_us,
                })
            }
            // Active or Decaying channel + probe Decaying → Decay
            (
                Some(ActionOutputPhase::Active | ActionOutputPhase::Decaying),
                ProbePhase::Decaying,
            ) => Some(ActionOutputEvent {
                channel_kind: OutputChannelKind::GenericWrite,
                feedback: ActionOutputFeedback::Decay,
                magnitude: decay_magnitude,
                event_t_us: now_us,
            }),
            // Active or Decaying channel + probe Idle → Stop
            (Some(ActionOutputPhase::Active | ActionOutputPhase::Decaying), ProbePhase::Idle) => {
                Some(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Stop,
                    magnitude: 0.0,
                    event_t_us: now_us,
                })
            }
            // Idle channel + Idle probe → None
            // Idle channel + Decaying probe → None (no active channel to decay)
            _ => None,
        };

        // Step 4: fire event if applicable.
        if let Some(event) = maybe_event {
            self.apply_action_output_event(event);
        }

        // Step 5: update output_intensity in probe_law write lock.
        let mut st = self.probe_law.write().unwrap();
        st.output_intensity = match probe_phase {
            ProbePhase::Active | ProbePhase::Rising => intensity,
            ProbePhase::Decaying | ProbePhase::Idle => {
                (st.output_intensity * (1.0 - PROBE_SUPPRESSION_ALPHA)).max(0.0)
            }
        };
    }

    /// Returns a clone of the current probe-law state for snapshot and observer.
    pub fn read_probe_law(&self) -> ProbeLawState {
        self.probe_law.read().unwrap().clone()
    }

    /// Surprise-driven trigger (Task 5).
    ///
    /// Fires a Begin event when global_surprise_load strictly exceeds
    /// SURPRISE_TRIGGER_THRESHOLD, regardless of current channel phase.
    /// Begin always opens a new write epoch; channel state is replaced.
    ///
    /// Engineering approximation — threshold in lib.rs.
    /// Lock ordering: read global_reasoning (drop), fire event (write lock
    /// inside apply_action_output_event).
    pub fn tick_surprise_trigger(&self, now_us: u64) {
        let surprise_load = { self.global_reasoning.read().unwrap().global_surprise_load };
        if surprise_load <= SURPRISE_TRIGGER_THRESHOLD {
            return;
        }
        self.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Begin,
            magnitude: surprise_load.clamp(0.0, 1.0),
            event_t_us: now_us,
        });
    }

    /// Attention-rebalance trigger (Task 6).
    ///
    /// Fires an Adjust event when the top routing-focus slot id changes
    /// since the last call. Adjust is a no-op if no channel is active.
    ///
    /// Engineering approximation — top-slot switch detection via stored
    /// last_top_slot_id in ProbeLawState.
    /// Lock ordering: read global_reasoning (drop), read+write probe_law
    /// (drop), fire event (write lock inside apply_action_output_event).
    pub fn tick_attention_trigger(&self, now_us: u64) {
        let current_top = { self.global_reasoning.read().unwrap().routing_focus_slot_ids[0] };

        let slot_changed = {
            let st = self.probe_law.read().unwrap();
            st.last_top_slot_id != current_top
        };

        if !slot_changed {
            return;
        }

        // Persist the new top slot before firing so re-entrant calls are safe.
        {
            self.probe_law.write().unwrap().last_top_slot_id = current_top;
        }

        self.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Adjust,
            magnitude: 1.0, // nominal full magnitude for an attention switch
            event_t_us: now_us,
        });
    }

    /// Relation-instability trigger (Task 7).
    ///
    /// Fires a Begin event when the maximum relation_precision (EMA of
    /// relation_err²) across all active edges strictly exceeds
    /// RELATION_INSTABILITY_TRIGGER_THRESHOLD.
    /// Begin always opens a new write epoch.
    ///
    /// Engineering approximation — threshold in lib.rs.
    /// Lock ordering: read global_relations (drop), fire event (write lock
    /// inside apply_action_output_event).
    pub fn tick_relation_instability_trigger(&self, now_us: u64) {
        let max_precision = {
            let gr = self.global_relations.read().unwrap();
            gr.edges
                .iter()
                .filter(|e| e.active)
                .map(|e| e.pred.relation_precision)
                .fold(0.0_f64, f64::max)
        };

        if max_precision <= RELATION_INSTABILITY_TRIGGER_THRESHOLD {
            return;
        }

        self.apply_action_output_event(ActionOutputEvent {
            channel_kind: OutputChannelKind::GenericWrite,
            feedback: ActionOutputFeedback::Begin,
            magnitude: max_precision.clamp(0.0, 1.0),
            event_t_us: now_us,
        });
    }
}
