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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VocalOrganState {
    pub breath_drive: f64,
    pub pitch_tension: f64,
    pub resonance_position: f64,
    pub noise_mix: f64,
    // v2 formant parameters (Hz)
    pub formant_f1: f64,
    pub formant_f2: f64,
    pub formant_f3: f64,
    pub voiced_ratio: f64,
    pub breath_noise_ratio: f64,
    // v2 envelope
    pub envelope_attack: f64,
    pub envelope_decay: f64,
    pub envelope_sustain: f64,
    pub phase: VocalOrganPhase,
    pub last_update_t_us: u64,
}

impl Default for VocalOrganState {
    fn default() -> Self {
        Self {
            breath_drive: 0.0,
            pitch_tension: 0.0,
            resonance_position: 0.0,
            noise_mix: 0.0,
            // Infant vocal tract defaults (shorter tract → higher formants)
            formant_f1: 800.0,
            formant_f2: 2500.0,
            formant_f3: 4000.0,
            voiced_ratio: 0.0,
            breath_noise_ratio: 0.0,
            envelope_attack: 0.0,
            envelope_decay: 0.0,
            envelope_sustain: 0.0,
            phase: VocalOrganPhase::Idle,
            last_update_t_us: 0,
        }
    }
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

    pub fn begin_voicing_v2(
        &mut self, now_us: u64,
        pitch: f64, f1: f64, f2: f64, f3: f64,
        voiced: f64, noise: f64,
        attack: f64, decay: f64, sustain: f64,
    ) {
        self.pitch_tension = pitch.clamp(0.0, 1.0);
        self.breath_drive = 1.0;
        self.formant_f1 = f1.clamp(150.0, 1200.0);
        self.formant_f2 = f2.clamp(400.0, 3500.0);
        self.formant_f3 = f3.clamp(1200.0, 4500.0);
        self.voiced_ratio = voiced.clamp(0.0, 1.0);
        self.breath_noise_ratio = noise.clamp(0.0, 1.0);
        self.envelope_attack = attack.clamp(0.0, 1.0);
        self.envelope_decay = decay.clamp(0.0, 1.0);
        self.envelope_sustain = sustain.clamp(0.0, 1.0);
        self.phase = VocalOrganPhase::Active;
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
