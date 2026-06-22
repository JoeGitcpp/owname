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

/// Syllable-frame articulator rhythm (Hz). Canonical babble alternates open↔close
/// at ~3 Hz (MacNeilage frame/content): a rhythmic mandibular/aperture limit-cycle
/// onto which slower gestures ride. This is the cheapest DISCRETE-GENERATING
/// dynamic — it segments an otherwise continuous drone into CV-like syllables.
pub const SYLLABLE_RATE_HZ: f64 = 3.0;
/// The aperture never fully closes (a sonorant/approximant constriction, not a
/// hard stop): keeps a thread of self-sound alive through the closure so the
/// heard-self loop never goes 0×x silent, while still giving a deep amplitude dip.
pub const APERTURE_FLOOR: f64 = 0.12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VocalOrganState {
    pub breath_drive: f64,
    pub pitch_tension: f64,
    pub resonance_position: f64,
    pub noise_mix: f64,
    /// Vocal-tract APERTURE [APERTURE_FLOOR, 1]: 1 = open (vowel), low = constricted
    /// (consonant). Driven by an open→close limit cycle (`syllable_phase`); the
    /// synthesizer gates source amplitude by it, so a held breath becomes a
    /// SEQUENCE of syllables instead of one flat vowel. The missing articulatory
    /// degree of freedom whose oscillation generates temporal discreteness.
    pub aperture: f64,
    /// Phase of the syllable-frame oscillator [0, 2π). Organ state advanced by the
    /// body's own voicing dt (NOT a global clock); stops when voicing stops.
    pub syllable_phase: f64,
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
            aperture: 1.0,
            syllable_phase: 0.0,
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
    /// Baseline vowel-line mapping resonance_position → (F1, F2) in Hz. This is
    /// the legacy 1-scalar locus the synthesizer used to derive internally; it
    /// now lives here so the 3-scalar voicing path writes it into the EXPLICIT
    /// formant fields and the synthesizer simply reads those. The imitation path
    /// can then drive F1/F2 OFF this line for independent 2D vowel control —
    /// which the single resonance scalar physically cannot reach.
    pub fn formants_from_resonance(reso: f64) -> (f64, f64) {
        let r = reso.clamp(0.0, 1.0);
        (300.0 + r * 600.0, 850.0 + r * 1500.0) // F1 300–900 Hz, F2 850–2350 Hz
    }

    /// Advance the syllable-frame articulator limit cycle by the elapsed voicing
    /// dt and recompute aperture. Open (1) at phase 0 → constricted (floor) at π →
    /// open again: a held breath is rendered as a SEQUENCE of CV syllables. Called
    /// each voicing tick; engaged only while the organ is actually voicing, so it
    /// is body-state-driven, not a global wall-clock.
    fn advance_syllable(&mut self, now_us: u64) {
        let dt_s = now_us.saturating_sub(self.last_update_t_us) as f64 / 1_000_000.0;
        if dt_s <= 0.0 {
            return;
        }
        self.syllable_phase = (self.syllable_phase
            + std::f64::consts::TAU * SYLLABLE_RATE_HZ * dt_s)
            .rem_euclid(std::f64::consts::TAU);
        let open = 0.5 + 0.5 * self.syllable_phase.cos(); // [0,1], 1 at vowel-open, 0 at closure
        self.aperture = APERTURE_FLOOR + (1.0 - APERTURE_FLOOR) * open;
    }

    pub fn begin_voicing(&mut self, now_us: u64, breath: f64, pitch: f64, resonance: f64) {
        self.breath_drive = breath;
        self.pitch_tension = pitch;
        self.resonance_position = resonance;
        // Start each fresh utterance vowel-open (phase 0).
        self.syllable_phase = 0.0;
        self.aperture = 1.0;
        // Keep explicit formants in sync with the resonance scalar so the
        // synthesizer (which reads them) reproduces the exact legacy vowel locus.
        let (f1, f2) = Self::formants_from_resonance(resonance);
        self.formant_f1 = f1;
        self.formant_f2 = f2;
        self.formant_f3 = 2500.0;
        self.phase = VocalOrganPhase::Active;
        self.last_update_t_us = now_us;
    }

    pub fn adjust_toward(&mut self, now_us: u64, breath: f64, pitch: f64, resonance: f64) {
        // Advance the syllable oscillator over the elapsed dt BEFORE last_update is
        // bumped, so a held voicing is segmented into syllables.
        self.advance_syllable(now_us);
        let alpha = 0.1;
        self.breath_drive = self.breath_drive * (1.0 - alpha) + breath * alpha;
        self.pitch_tension = self.pitch_tension * (1.0 - alpha) + pitch * alpha;
        self.resonance_position = self.resonance_position * (1.0 - alpha) + resonance * alpha;
        let (f1, f2) = Self::formants_from_resonance(self.resonance_position);
        self.formant_f1 = f1;
        self.formant_f2 = f2;
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
        self.syllable_phase = 0.0;
        self.aperture = 1.0;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aperture_limit_cycle_segments_a_held_voicing() {
        // A held voicing must be rendered as a SEQUENCE of syllables: the aperture
        // oscillator opens (vowel) and constricts (consonant) at the syllable rate,
        // never fully closing (floored). This is the cheapest discrete-GENERATING
        // dynamic — without it, a held breath is one flat continuous drone.
        let mut o = VocalOrganState::default();
        o.begin_voicing(0, 0.8, 0.5, 0.5);
        assert!((o.aperture - 1.0).abs() < 1e-9, "fresh utterance starts vowel-open");
        let (mut amin, mut amax) = (f64::INFINITY, 0.0_f64);
        let mut t = 0u64;
        for _ in 0..120 {
            t += 10_000; // 10 ms steps → ~1.2 s ≈ 3.6 syllables at 3 Hz
            o.adjust_toward(t, 0.8, 0.5, 0.5);
            amin = amin.min(o.aperture);
            amax = amax.max(o.aperture);
        }
        assert!(amax > 0.8, "aperture must open (vowel), got max {amax}");
        assert!(amin < 0.3, "aperture must constrict (consonant), got min {amin}");
        assert!(
            amin >= APERTURE_FLOOR - 1e-9,
            "aperture is floored (never fully silent → no 0×x), got min {amin}"
        );
    }

    #[test]
    fn syllable_oscillator_is_engagement_gated_not_a_wall_clock() {
        // Zero elapsed voicing dt → no phase advance. The oscillator is organ state
        // driven by the body's own voicing, not a global clock (which would repeat
        // the scripted-sine anti-pattern).
        let mut o = VocalOrganState::default();
        o.begin_voicing(1_000, 0.8, 0.5, 0.5);
        let p0 = o.syllable_phase;
        o.adjust_toward(1_000, 0.8, 0.5, 0.5); // same now_us → dt = 0
        assert_eq!(o.syllable_phase, p0, "no dt → no oscillator advance");
    }
}
