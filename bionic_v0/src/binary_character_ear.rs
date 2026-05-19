use crate::code_ingress::CodeIngressFrameInput;

pub const BINARY_CHARACTER_LANE_CAP: usize = 16;

pub struct BinaryCharacterBurstInput<'a> {
    pub fiber_id: u64,
    pub carrier_id: u64,
    pub t_start_us: u64,
    pub t_end_us: u64,
    pub bytes: &'a [u8],
    pub burst_energy: f64,
    pub framing_validity: f64,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BinaryCharacterEarState {
    pub lane_count: u32,
    pub total_bursts: u64,
    pub last_update_t_us: u64,
    pub collision_pressure: f64,
    pub active_lane_id: u64,
    pub lanes: Vec<BinaryCharacterLaneTrace>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BinaryCharacterLaneTrace {
    pub lane_id: u64,
    pub fiber_id: u64,
    pub carrier_id: u64,
    pub last_burst_ref: u64,
    pub last_t_us: u64,
    pub byte_hash: u64,
    pub byte_len: u32,
    pub burst_count: u32,
    pub continuity_score: f64,
    pub recurrence_score: f64,
    pub collision_pressure: f64,
    pub prediction_pressure: f64,
    pub return_correlation: f64,
}

impl BinaryCharacterEarState {
    pub fn observe_burst(&mut self, input: BinaryCharacterBurstInput<'_>) -> u64 {
        self.total_bursts = self.total_bursts.saturating_add(1);
        let byte_hash = hash_bytes(input.bytes);
        let burst_ref = nonzero_ref(byte_hash ^ self.total_bursts, self.total_bursts);
        let collision = self.collision_for(&input);
        let lane_idx = self
            .lanes
            .iter()
            .position(|lane| lane.fiber_id == input.fiber_id && lane.carrier_id == input.carrier_id)
            .unwrap_or_else(|| self.push_lane(&input));

        let lane = &mut self.lanes[lane_idx];
        let repeated_shape = lane.byte_hash == byte_hash && lane.burst_count > 0;
        let previous_count = lane.burst_count;
        lane.last_burst_ref = burst_ref;
        lane.last_t_us = input.t_end_us;
        lane.byte_hash = byte_hash;
        lane.byte_len = input.bytes.len().min(u32::MAX as usize) as u32;
        lane.burst_count = lane.burst_count.saturating_add(1);

        if previous_count > 0 {
            lane.continuity_score = clamp01(lane.continuity_score + 0.25);
        }
        if repeated_shape {
            lane.recurrence_score = clamp01(lane.recurrence_score + 0.25);
        }

        lane.collision_pressure = clamp01(lane.collision_pressure + collision);
        lane.prediction_pressure =
            clamp01(1.0 - ((lane.continuity_score + lane.recurrence_score) * 0.5));

        self.collision_pressure = clamp01(self.collision_pressure + collision);
        self.active_lane_id = lane.lane_id;
        self.last_update_t_us = input.t_end_us;
        self.lane_count = self.lanes.len().min(u32::MAX as usize) as u32;
        burst_ref
    }

    pub fn lane_stream_context_id(lane_id: u64) -> u64 {
        0xB1C0_0000_0000_0000_u64 ^ lane_id
    }

    fn push_lane(&mut self, input: &BinaryCharacterBurstInput<'_>) -> usize {
        if self.lanes.len() == BINARY_CHARACTER_LANE_CAP {
            self.lanes.remove(0);
        }
        let lane_id = nonzero_ref(
            input.fiber_id.rotate_left(17) ^ input.carrier_id,
            input.fiber_id,
        );
        self.lanes.push(BinaryCharacterLaneTrace {
            lane_id,
            fiber_id: input.fiber_id,
            carrier_id: input.carrier_id,
            prediction_pressure: 1.0,
            ..BinaryCharacterLaneTrace::default()
        });
        self.lanes.len() - 1
    }

    fn collision_for(&self, input: &BinaryCharacterBurstInput<'_>) -> f64 {
        let overlap = self.lanes.iter().any(|lane| {
            lane.fiber_id != input.fiber_id
                && lane.carrier_id == input.carrier_id
                && lane.last_t_us >= input.t_start_us
        });
        if overlap {
            clamp01(input.burst_energy * input.framing_validity)
        } else {
            0.0
        }
    }
}

pub fn to_code_ingress_frame<'a>(
    lane_id: u64,
    input: &BinaryCharacterBurstInput<'a>,
) -> CodeIngressFrameInput<'a> {
    CodeIngressFrameInput {
        stream_context_id: BinaryCharacterEarState::lane_stream_context_id(lane_id),
        t_start_us: input.t_start_us,
        t_end_us: input.t_end_us,
        bytes: input.bytes,
        framing_validity: input.framing_validity,
    }
}

fn clamp01(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325u64;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn nonzero_ref(value: u64, fallback: u64) -> u64 {
    if value == 0 {
        fallback.max(1)
    } else {
        value
    }
}
