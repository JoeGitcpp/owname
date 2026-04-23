#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VocalOrganPhase {
    #[default]
    Idle,
    Active,
}

impl VocalOrganPhase {
    pub fn as_u8(self) -> u8 {
        match self {
            VocalOrganPhase::Idle => 0,
            VocalOrganPhase::Active => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct VocalOrganState {
    pub breath_drive: f64,
    pub pitch_tension: f64,
    pub resonance_position: f64,
    pub noise_mix: f64,
    pub phase: VocalOrganPhase,
    pub last_update_t_us: u64,
}

impl VocalOrganState {
    pub fn begin_voicing(&mut self, now_us: u64, breath: f64, pitch: f64, resonance: f64) {
        self.breath_drive = breath;
        self.pitch_tension = pitch;
        self.resonance_position = resonance;
        self.phase = VocalOrganPhase::Active;
        self.last_update_t_us = now_us;
    }

    pub fn adjust_toward(&mut self, now_us: u64, breath: f64, pitch: f64, resonance: f64) {
        let alpha = 0.1;
        self.breath_drive = self.breath_drive * (1.0 - alpha) + breath * alpha;
        self.pitch_tension = self.pitch_tension * (1.0 - alpha) + pitch * alpha;
        self.resonance_position = self.resonance_position * (1.0 - alpha) + resonance * alpha;
        self.last_update_t_us = now_us;
    }

    pub fn tick_decay(&mut self, now_us: u64) {
        let elapsed_us = now_us.saturating_sub(self.last_update_t_us);
        if elapsed_us == 0 || self.phase == VocalOrganPhase::Idle {
            return;
        }
        let decay_factor = 0.5_f64.powf(elapsed_us as f64 / 2_000_000.0);
        self.breath_drive *= decay_factor;
        if self.breath_drive < 0.01 {
            self.phase = VocalOrganPhase::Idle;
            self.breath_drive = 0.0;
        }
    }
}