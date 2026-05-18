// Probe-law substrate (minimal active-probe entry).
//
// Numeric/time-only. No semantic labels, no concept identifiers,
// no naming qualifiers. Phase transitions driven by
// intensity-vs-suppression ratios.

use crate::action_output::{
    ActionOutputEvent, ActionOutputFeedback, ActionOutputPhase, OutputChannelKind,
};
use crate::viewport::Viewport;
use crate::{PROBE_HISTORY_CAP, PROBE_INTENSITY_ALPHA, PROBE_SUPPRESSION_ALPHA};

/// Probe lifecycle phase. Numeric code.
/// P0=Idle, P1=Rising, P2=Active, P3=Decaying.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
#[repr(u8)]
pub enum ProbePhase {
    #[default]
    P0 = 0,
    P1 = 1,
    P2 = 2,
    P3 = 3,
}

impl ProbePhase {
    pub fn as_u8(self) -> u8 {
        self as u8
    }
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => ProbePhase::P1,
            2 => ProbePhase::P2,
            3 => ProbePhase::P3,
            _ => ProbePhase::P0,
        }
    }
}

/// Internal state of the probe-law substrate.
///
/// All fields are numeric or temporal.
#[derive(Clone, Debug)]
pub struct ProbeLawState {
    /// EMA-smoothed probe intensity. Driven by global reasoning novelty. [0,1].
    pub intensity: f64,
    /// Slower EMA of intensity; acts as adaptive suppression baseline. [0,1].
    pub suppression: f64,
    /// Current lifecycle phase code.
    pub phase: ProbePhase,
    /// Substrate time of last refresh (us).
    pub last_update_t_us: u64,
    /// Output intensity produced by tick_probe_output_from_law. [0,1].
    pub output_intensity: f64,
    /// Short ring of recent intensity values. Length = PROBE_HISTORY_CAP.
    pub history: [f64; PROBE_HISTORY_CAP],
    pub history_head: usize,
    pub history_len: usize,
    /// Last observed top routing slot id; used during refresh to fold
    /// slot-switch pressure into the shared probe drive. -1 = uninitialized.
    /// Ephemeral: not persisted in snapshots.
    pub last_top_slot_id: i8,
}

impl Default for ProbeLawState {
    fn default() -> Self {
        Self {
            intensity: 0.0,
            suppression: 0.0,
            phase: ProbePhase::P0,
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
    /// Drive probe-law state from current pressure substrate (EMA-based).
    ///
    /// Phase transitions are ratio-based.
    pub fn refresh_probe_law(&self, now_us: u64) {
        let novelty = self.read_global_reasoning_memory_summary().novelty_score;
        let evidence_pressure = self.read_evidence_tension().unresolved_pressure;
        let (surprise_pressure, current_top_slot_id) = {
            let rs = self.global_reasoning.read().unwrap();
            (rs.global_surprise_load, rs.routing_focus_slot_ids[0])
        };
        let relation_pressure = {
            let gr = self.global_relations.read().unwrap();
            gr.edges
                .iter()
                .filter(|e| e.active)
                .map(|e| e.pred.relation_surprise)
                .fold(0.0_f64, f64::max)
        };
        let active_gap_pressure = self.read_active_reasoning_gap().gap_pressure;
        let active_surface = self.read_active_surface();
        let field_pressure = if active_surface.traces_alive > 0 {
            active_surface.effective_probe_pressure()
        } else {
            self.read_prediction_pressure_field().dominant_pressure
        };

        let mut st = self.probe_law.write().unwrap();
        let slot_switch_pressure =
            if st.last_top_slot_id != current_top_slot_id && current_top_slot_id >= 0 {
                1.0
            } else {
                0.0
            };
        let raw_drive = novelty
            .max(evidence_pressure)
            .max(surprise_pressure)
            .max(relation_pressure)
            .max(active_gap_pressure)
            .max(field_pressure)
            .max(slot_switch_pressure)
            .clamp(0.0, 1.0);
        let output_suppression = self.read_self_development().output_suppression;
        let drive = (raw_drive * (1.0 - 0.75 * output_suppression)).clamp(0.0, 1.0);

        st.intensity = (1.0 - PROBE_INTENSITY_ALPHA) * st.intensity + PROBE_INTENSITY_ALPHA * drive;
        st.suppression = (1.0 - PROBE_SUPPRESSION_ALPHA) * st.suppression
            + PROBE_SUPPRESSION_ALPHA * st.intensity;

        st.phase = match st.phase {
            ProbePhase::P0 => {
                if st.suppression > 1e-9 {
                    let ratio = st.intensity / st.suppression;
                    if ratio > 1.0 {
                        ProbePhase::P1
                    } else {
                        ProbePhase::P0
                    }
                } else if st.intensity > 1e-9 {
                    ProbePhase::P1
                } else {
                    ProbePhase::P0
                }
            }
            ProbePhase::P1 => {
                let ratio = if st.suppression > 1e-9 {
                    st.intensity / st.suppression
                } else {
                    2.0
                };
                if ratio > 1.05 {
                    ProbePhase::P2
                } else if ratio < 1.0 {
                    ProbePhase::P3
                } else {
                    ProbePhase::P1
                }
            }
            ProbePhase::P2 => {
                let ratio = if st.suppression > 1e-9 {
                    st.intensity / st.suppression
                } else {
                    2.0
                };
                if ratio < 1.0 {
                    ProbePhase::P3
                } else {
                    ProbePhase::P2
                }
            }
            ProbePhase::P3 => {
                if st.suppression > 1e-9 {
                    let ratio = st.intensity / st.suppression;
                    if ratio > 1.0 {
                        ProbePhase::P1
                    } else if st.intensity < st.suppression * 0.1 {
                        ProbePhase::P0
                    } else {
                        ProbePhase::P3
                    }
                } else if st.intensity < 1e-10 {
                    ProbePhase::P0
                } else {
                    ProbePhase::P3
                }
            }
        };

        let head = st.history_head;
        st.history[head] = st.intensity;
        st.history_head = (head + 1) % PROBE_HISTORY_CAP;
        if st.history_len < PROBE_HISTORY_CAP {
            st.history_len += 1;
        }

        st.last_update_t_us = now_us;
        st.last_top_slot_id = current_top_slot_id;
    }

    /// Produce tick output from current probe-law state and drive action_output events.
    ///
    /// Mapping (channel phase + probe phase -> event):
    ///   Channel Idle + probe P1/P2 -> Begin
    ///   Channel Active + probe P1/P2 -> Adjust
    ///   Channel Decaying + probe P1/P2 -> Begin
    ///   Channel Active/P2 + probe P3 -> Decay
    ///   Channel Active/Decaying + probe P0 -> Stop
    ///   Channel Idle + probe P0 -> None
    ///   Channel Idle + probe P3 -> None
    pub fn tick_probe_output_from_law(&self, now_us: u64) {
        let (probe_phase, intensity, current_output_intensity) = {
            let st = self.probe_law.read().unwrap();
            (st.phase, st.intensity, st.output_intensity)
        };

        let channel_phase = {
            let ao = self.action_output.read().unwrap();
            ao.channels
                .iter()
                .find(|c| c.channel_kind == OutputChannelKind::GenericWrite)
                .map(|c| c.phase)
        };

        let typical_delay = self.compute_typical_consequence_delay();
        let delay_factor = if typical_delay > 0 {
            (typical_delay as f64 / 1_000_000.0).clamp(0.1, 5.0)
        } else {
            1.0
        };

        let decay_magnitude =
            ((current_output_intensity * PROBE_SUPPRESSION_ALPHA) / delay_factor).max(1e-9);
        let mag = (intensity / delay_factor).clamp(0.0, 1.0);

        let maybe_event: Option<ActionOutputEvent> = match (channel_phase, probe_phase) {
            (None, ProbePhase::P1 | ProbePhase::P2)
            | (Some(ActionOutputPhase::Idle), ProbePhase::P1 | ProbePhase::P2) => {
                Some(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Begin,
                    magnitude: mag,
                    event_t_us: now_us,
                })
            }
            (Some(ActionOutputPhase::Active), ProbePhase::P1 | ProbePhase::P2) => {
                Some(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Adjust,
                    magnitude: mag,
                    event_t_us: now_us,
                })
            }
            (Some(ActionOutputPhase::Decaying), ProbePhase::P1 | ProbePhase::P2) => {
                Some(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Begin,
                    magnitude: mag,
                    event_t_us: now_us,
                })
            }
            (Some(ActionOutputPhase::Active | ActionOutputPhase::Decaying), ProbePhase::P3) => {
                Some(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Decay,
                    magnitude: decay_magnitude,
                    event_t_us: now_us,
                })
            }
            (Some(ActionOutputPhase::Active | ActionOutputPhase::Decaying), ProbePhase::P0) => {
                Some(ActionOutputEvent {
                    channel_kind: OutputChannelKind::GenericWrite,
                    feedback: ActionOutputFeedback::Stop,
                    magnitude: 0.0,
                    event_t_us: now_us,
                })
            }
            _ => None,
        };

        if let Some(event) = maybe_event {
            self.apply_probe_vocal_write_event(event);
        }

        let mut st = self.probe_law.write().unwrap();
        st.output_intensity = match probe_phase {
            ProbePhase::P2 | ProbePhase::P1 => intensity,
            ProbePhase::P3 | ProbePhase::P0 => {
                (st.output_intensity * (1.0 - PROBE_SUPPRESSION_ALPHA)).max(0.0)
            }
        };
    }

    /// Returns a clone of the current probe-law state for snapshot and observer.
    pub fn read_probe_law(&self) -> ProbeLawState {
        self.probe_law.read().unwrap().clone()
    }
}
