use crate::live_byte_closure::LIVE_BYTE_EGRESS_FRAME_LEN;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BinaryCharacterMouthState {
    pub active_lane_id: u64,
    pub output_pressure: f64,
    pub burst_length: u32,
    pub rhythm_score: f64,
    pub predicted_byte_hash: u64,
    pub last_output_ref: u64,
    pub output_count: u64,
    pub return_count: u64,
    pub return_match: f64,
    pub correction_deviation: f64,
    pub last_update_t_us: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BinaryCharacterOutputDrive {
    pub lane_id: u64,
    pub t_us: u64,
    pub pressure: f64,
    pub seed_hash: u64,
    pub requested_len: usize,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct BinaryCharacterOutputBurst {
    pub output_ref: u64,
    pub lane_id: u64,
    pub t_us: u64,
    pub bytes: Vec<u8>,
    pub pressure: f64,
}

impl BinaryCharacterMouthState {
    pub fn begin_output(
        &mut self,
        drive: BinaryCharacterOutputDrive,
    ) -> BinaryCharacterOutputBurst {
        self.output_count = self.output_count.saturating_add(1);
        let len = drive.requested_len.clamp(1, LIVE_BYTE_EGRESS_FRAME_LEN);
        let pressure = clamp01(drive.pressure);
        let mut seed = drive.seed_hash
            ^ drive.lane_id.rotate_left(7)
            ^ self.output_count.rotate_left(19)
            ^ ((pressure * 65_535.0) as u64);
        let mut bytes = Vec::with_capacity(len);
        for _ in 0..len {
            seed = seed.rotate_left(9).wrapping_mul(0x9E37_79B1_85EB_CA87);
            bytes.push((seed & 0xff) as u8);
        }

        let byte_hash = hash_bytes(&bytes);
        let output_ref = nonzero_ref(byte_hash ^ self.output_count, self.output_count);
        self.active_lane_id = drive.lane_id;
        self.output_pressure = pressure;
        self.burst_length = bytes.len().min(u32::MAX as usize) as u32;
        self.rhythm_score = clamp01(self.rhythm_score + pressure * 0.25);
        self.predicted_byte_hash = byte_hash;
        self.last_output_ref = output_ref;
        self.last_update_t_us = drive.t_us;

        BinaryCharacterOutputBurst {
            output_ref,
            lane_id: drive.lane_id,
            t_us: drive.t_us,
            bytes,
            pressure,
        }
    }

    pub fn observe_return(
        &mut self,
        lane_id: u64,
        t_us: u64,
        returned_bytes: &[u8],
        magnitude: f64,
        correction_deviation: f64,
    ) {
        self.active_lane_id = lane_id;
        self.return_count = self.return_count.saturating_add(1);
        let deviation = clamp01(correction_deviation);
        self.return_match = clamp01((1.0 - deviation).max(clamp01(magnitude) * 0.5));
        self.correction_deviation = deviation;
        self.predicted_byte_hash = hash_bytes(returned_bytes);
        self.last_update_t_us = t_us;
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
