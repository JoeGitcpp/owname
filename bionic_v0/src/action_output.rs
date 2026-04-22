// Action-output substrate — body's write-out side.
//
// Implements the minimal abstract contract defined in:
//   docs/superpowers/specs/2026-04-21-action-output-substrate-minimal-contract-design.md
//
// Channel-based design: one channel per OutputChannelKind, each carrying a
// numeric write-trace.  current_write_seq is a monotonic engineering handle
// used by the action-consequence substrate for pairing; it is not a target,
// concept, or meaning field.

use std::sync::atomic::Ordering;

use crate::viewport::Viewport;
use crate::WRITE_EPOCH_HISTORY_CAP;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum OutputChannelKind {
    #[default]
    GenericWrite,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ActionOutputPhase {
    #[default]
    Idle,
    Active,
    Decaying,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActionOutputFeedback {
    Begin,
    Adjust,
    Decay,
    Stop,
}

#[derive(Clone, Debug)]
pub struct ActionOutputEvent {
    pub channel_kind: OutputChannelKind,
    pub feedback: ActionOutputFeedback,
    pub magnitude: f64,
    pub event_t_us: u64,
}

#[derive(Clone, Debug, Default)]
pub struct ActionOutputWriteTrace {
    pub channel_kind: OutputChannelKind,
    pub phase: ActionOutputPhase,
    pub current_magnitude: f64,
    pub peak_magnitude: f64,
    pub start_t_us: u64,
    pub last_update_t_us: u64,
    pub last_transition_t_us: u64,
    pub reinforce_count: u32,
    pub decay_count: u32,
    /// Monotonic pairing handle incremented on every Begin.
    /// Engineering-only: no target, concept, or meaning semantics.
    pub current_write_seq: u64,
}

/// Minimal write-epoch ledger entry.
///
/// Records the substrate-level facts of one write epoch so that
/// `pair_action_consequence` can verify a write_seq is real and
/// look up its time/phase context. Contains no target, result,
/// meaning, or world-content fields.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct WriteEpochRecord {
    /// Monotonic pairing handle (same as ActionOutputWriteTrace::current_write_seq).
    pub write_seq: u64,
    pub channel_kind: OutputChannelKind,
    pub start_t_us: u64,
    /// Updated on every Adjust / Decay / Stop event for this epoch.
    pub last_update_t_us: u64,
    /// Tracks the last phase transition for this epoch.
    pub last_phase: ActionOutputPhase,
}

#[derive(Clone, Debug, Default)]
pub struct ActionOutputState {
    pub n_channels: u32,
    pub channels: Vec<ActionOutputWriteTrace>,
    /// Append-only ledger of write epochs, one entry per Begin event.
    /// Used by action_consequence to verify write_seq exists before pairing.
    /// Numeric/time-only; no target, concept, or meaning fields.
    pub write_epoch_history: Vec<WriteEpochRecord>,
    pub n_write_epochs: u32,
}

impl Viewport {
    pub fn read_action_output(&self) -> ActionOutputState {
        self.action_output.read().unwrap().clone()
    }

    pub fn apply_action_output_event(&self, event: ActionOutputEvent) {
        let mut s = self.action_output.write().unwrap();
        let pos = s
            .channels
            .iter()
            .position(|c| c.channel_kind == event.channel_kind);
        let mag = event.magnitude.clamp(0.0, 1.0);
        match event.feedback {
            ActionOutputFeedback::Begin => {
                let seq = self.action_output_write_seq.fetch_add(1, Ordering::Relaxed) + 1;
                let trace = ActionOutputWriteTrace {
                    channel_kind: event.channel_kind,
                    phase: ActionOutputPhase::Active,
                    current_magnitude: mag,
                    peak_magnitude: mag,
                    start_t_us: event.event_t_us,
                    last_update_t_us: event.event_t_us,
                    last_transition_t_us: event.event_t_us,
                    reinforce_count: 0,
                    decay_count: 0,
                    current_write_seq: seq,
                };
                if let Some(i) = pos {
                    s.channels[i] = trace;
                } else {
                    s.channels.push(trace);
                    s.n_channels = s.channels.len() as u32;
                }
                // Record the new epoch in the append-only ledger.
                s.write_epoch_history.push(WriteEpochRecord {
                    write_seq: seq,
                    channel_kind: event.channel_kind,
                    start_t_us: event.event_t_us,
                    last_update_t_us: event.event_t_us,
                    last_phase: ActionOutputPhase::Active,
                });
                // Eviction: keep history within the engineering cap. Evict from the front
                // (oldest first). Selection is purely by append order (oldest = index 0),
                // not by result meaning or object importance. This is a substrate knob.
                if s.write_epoch_history.len() > WRITE_EPOCH_HISTORY_CAP {
                    let evict_n = s.write_epoch_history.len() - WRITE_EPOCH_HISTORY_CAP;
                    s.write_epoch_history.drain(0..evict_n);
                }
                // Single authoritative assignment after eviction so n_write_epochs
                // always equals the actual Vec length.
                s.n_write_epochs = s.write_epoch_history.len() as u32;
            }
            ActionOutputFeedback::Adjust => {
                if let Some(i) = pos {
                    let seq = s.channels[i].current_write_seq;
                    s.channels[i].current_magnitude = mag;
                    if mag > s.channels[i].peak_magnitude {
                        s.channels[i].peak_magnitude = mag;
                    }
                    s.channels[i].last_update_t_us = event.event_t_us;
                    s.channels[i].reinforce_count += 1;
                    if let Some(ep) = s
                        .write_epoch_history
                        .iter_mut()
                        .rfind(|r| r.write_seq == seq)
                    {
                        ep.last_update_t_us = event.event_t_us;
                        ep.last_phase = ActionOutputPhase::Active;
                    }
                }
            }
            ActionOutputFeedback::Decay => {
                if let Some(i) = pos {
                    let seq = s.channels[i].current_write_seq;
                    let ch = &mut s.channels[i];
                    if ch.phase != ActionOutputPhase::Decaying {
                        ch.phase = ActionOutputPhase::Decaying;
                        ch.last_transition_t_us = event.event_t_us;
                    }
                    ch.current_magnitude = (ch.current_magnitude - mag).max(0.0);
                    ch.last_update_t_us = event.event_t_us;
                    ch.decay_count += 1;
                    if let Some(ep) = s
                        .write_epoch_history
                        .iter_mut()
                        .rfind(|r| r.write_seq == seq)
                    {
                        ep.last_update_t_us = event.event_t_us;
                        ep.last_phase = ActionOutputPhase::Decaying;
                    }
                }
            }
            ActionOutputFeedback::Stop => {
                if let Some(i) = pos {
                    let seq = s.channels[i].current_write_seq;
                    let ch = &mut s.channels[i];
                    ch.phase = ActionOutputPhase::Idle;
                    ch.current_magnitude = 0.0;
                    ch.last_update_t_us = event.event_t_us;
                    ch.last_transition_t_us = event.event_t_us;
                    if let Some(ep) = s
                        .write_epoch_history
                        .iter_mut()
                        .rfind(|r| r.write_seq == seq)
                    {
                        ep.last_update_t_us = event.event_t_us;
                        ep.last_phase = ActionOutputPhase::Idle;
                    }
                }
            }
        }
    }
}
