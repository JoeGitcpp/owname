// Intention system — numeric indices only.
// No semantic labels, no concept_id, no naming.
// Action parameters are neutral starting points, subject to drift
// through err-driven updates. Not pre-decoded world understanding.

/// Intention types (numeric indices only, not semantic labels)
pub const INTENT_0: u8 = 0;
pub const INTENT_1: u8 = 1;
pub const INTENT_2: u8 = 2;
pub const INTENT_3: u8 = 3;

/// Intention — a goal-directed motivational state
#[derive(Clone, Debug)]
pub struct Intention {
    /// Current intention type index (0-3)
    pub intent_type: u8,
    /// How strong this intention currently is [0, 1]
    pub strength: f64,
    /// How long this intention has persisted (ticks)
    pub persistence: u32,
    /// Decay rate if not reinforced [0, 1]
    pub decay_rate: f64,
    /// Last time this intention was reinforced (us)
    pub last_reinforced_us: u64,
    /// Action parameters tuned for this intention
    pub action_params: ActionParameters,
}

/// Action parameters associated with each intention.
/// All defaults are neutral — no pre-decoded "personality" per type.
/// Values drift through err-driven updates.
#[derive(Clone, Debug)]
pub struct ActionParameters {
    /// Probe intensity multiplier [0, 2]
    pub probe_intensity_multiplier: f64,
    /// Learning rate multiplier [0.5, 2]
    pub learning_rate_multiplier: f64,
    /// Attention spread [0, 1]: higher = more diffuse
    pub attention_spread: f64,
    /// Band activity preference [0, 1]: higher = more band activity
    pub band_activity_preference: f64,
}

impl Default for ActionParameters {
    fn default() -> Self {
        Self {
            probe_intensity_multiplier: 1.0,
            learning_rate_multiplier: 1.0,
            attention_spread: 0.5,
            band_activity_preference: 0.5,
        }
    }
}

impl ActionParameters {
    /// Neutral parameters — no pre-decoded preference for any intent type.
    pub fn neutral() -> Self {
        Self::default()
    }
}

impl Default for Intention {
    fn default() -> Self {
        Self {
            intent_type: INTENT_0,
            strength: 0.5,
            persistence: 0,
            decay_rate: 0.95,
            last_reinforced_us: 0,
            action_params: ActionParameters::neutral(),
        }
    }
}

impl Intention {
    /// Create a new intention with neutral defaults.
    pub fn new(now_us: u64) -> Self {
        Self {
            intent_type: INTENT_0,
            strength: 0.5,
            persistence: 0,
            decay_rate: 0.95,
            last_reinforced_us: now_us,
            action_params: ActionParameters::neutral(),
        }
    }

    /// Switch to a new intention type index.
    pub fn switch_to(&mut self, new_type: u8, now_us: u64) {
        self.intent_type = new_type.clamp(0, 3);
        self.action_params = ActionParameters::neutral();
        self.last_reinforced_us = now_us;
    }

    /// Update intention based on numeric outcome and system state.
    ///
    /// - outcome: [0, 1], higher = more successful
    /// - current_vitality: [0, 1], system vitality level
    /// - now_us: current timestamp
    pub fn update(&mut self, outcome: f64, current_vitality: f64, now_us: u64) {
        // Reinforcement: successful actions strengthen current intention
        if outcome > 0.7 {
            self.strength = (self.strength + 0.15).min(1.0);
            self.persistence = self.persistence.saturating_add(1);
            self.last_reinforced_us = now_us;
        }

        // Failure: low outcome weakens and may switch intention
        if outcome < 0.3 {
            self.strength *= 0.6;
            self.persistence = 0;
            // Switch to INTENT_0 as numeric fallback (lowest index)
            if self.strength < 0.2 {
                self.switch_to(INTENT_0, now_us);
                self.strength = 0.4;
            }
        }

        // Vitality check: if too low, force INTENT_3 (lowest activity index)
        if current_vitality < 0.2 && self.intent_type != INTENT_3 {
            self.switch_to(INTENT_3, now_us);
            self.strength = 0.8;
        }

        // Vitality check: if high and at lowest-activity, may return to INTENT_0
        if current_vitality > 0.7 && self.intent_type == INTENT_3 {
            self.switch_to(INTENT_0, now_us);
            self.strength = 0.5;
        }

        // Time decay: intentions weaken over time if not reinforced
        let time_since_reinforce = now_us.saturating_sub(self.last_reinforced_us);
        if time_since_reinforce > 3_000_000 {
            // 3 seconds
            self.strength *= self.decay_rate;
            if self.strength < 0.1 {
                self.switch_to(INTENT_0, now_us);
                self.strength = 0.3;
            }
        }

        // Clamp strength
        self.strength = self.strength.clamp(0.0, 1.0);
    }

    /// Get current action parameters.
    pub fn get_action_params(&self) -> &ActionParameters {
        &self.action_params
    }

    /// Check if this intention is strong enough to be active.
    pub fn is_active(&self) -> bool {
        self.strength > 0.2
    }
}

/// IntentionBank — manages multiple competing intentions.
#[derive(Clone, Debug, Default)]
pub struct IntentionBank {
    /// All intentions.
    pub intentions: [Intention; 4],
    /// Currently dominant intention index.
    pub dominant: u8,
    /// Last update time.
    pub last_update_us: u64,
}

impl IntentionBank {
    pub fn new(now_us: u64) -> Self {
        let mut bank = Self {
            intentions: [
                Intention::new(now_us),
                Intention {
                    intent_type: INTENT_1,
                    ..Default::default()
                },
                Intention {
                    intent_type: INTENT_2,
                    ..Default::default()
                },
                Intention {
                    intent_type: INTENT_3,
                    ..Default::default()
                },
            ],
            dominant: INTENT_0,
            last_update_us: now_us,
        };
        // Initialize intent types.
        for (i, intent) in bank.intentions.iter_mut().enumerate() {
            intent.intent_type = i as u8;
            intent.action_params = ActionParameters::neutral();
        }
        bank
    }

    /// Update all intentions and determine dominant.
    pub fn tick(&mut self, outcome: f64, current_vitality: f64, now_us: u64) {
        for intent in &mut self.intentions {
            intent.update(outcome, current_vitality, now_us);
        }

        // Find dominant (strongest).
        let mut max_strength = 0.0;
        let mut new_dominant = self.dominant;
        for (i, intent) in self.intentions.iter().enumerate() {
            if intent.strength > max_strength && intent.is_active() {
                max_strength = intent.strength;
                new_dominant = i as u8;
            }
        }
        self.dominant = new_dominant;
        self.last_update_us = now_us;
    }

    /// Get the currently dominant intention.
    pub fn get_dominant(&self) -> &Intention {
        &self.intentions[self.dominant as usize]
    }

    /// Get aggregate action params from all active intentions (weighted).
    pub fn aggregate_action_params(&self) -> ActionParameters {
        let mut total_weight = 0.0;
        let mut probe_mult = 0.0;
        let mut learn_mult = 0.0;
        let mut attn_spread = 0.0;
        let mut band_pref = 0.0;

        for intent in &self.intentions {
            if intent.is_active() {
                let weight = intent.strength;
                total_weight += weight;
                probe_mult += intent.action_params.probe_intensity_multiplier * weight;
                learn_mult += intent.action_params.learning_rate_multiplier * weight;
                attn_spread += intent.action_params.attention_spread * weight;
                band_pref += intent.action_params.band_activity_preference * weight;
            }
        }

        if total_weight > 0.0 {
            ActionParameters {
                probe_intensity_multiplier: probe_mult / total_weight,
                learning_rate_multiplier: learn_mult / total_weight,
                attention_spread: attn_spread / total_weight,
                band_activity_preference: band_pref / total_weight,
            }
        } else {
            ActionParameters::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intention_creation() {
        let intent = Intention::new(1000);
        assert_eq!(intent.intent_type, INTENT_0);
        assert!((intent.strength - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_intention_switch() {
        let mut intent = Intention::new(1000);
        intent.switch_to(INTENT_1, 2000);
        assert_eq!(intent.intent_type, INTENT_1);
        // Neutral params: learning_rate_multiplier = 1.0
        assert!((intent.action_params.learning_rate_multiplier - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_outcome_reinforcement() {
        let mut intent = Intention::new(1000);
        intent.strength = 0.5;
        intent.update(0.8, 0.8, 2000);
        assert!(intent.strength > 0.5);
    }

    #[test]
    fn test_outcome_failure() {
        let mut intent = Intention::new(1000);
        intent.strength = 0.5;
        intent.update(0.2, 0.8, 2000);
        assert!(intent.strength < 0.5);
    }

    #[test]
    fn test_low_vitality_force_rest() {
        let mut intent = Intention::new(1000);
        intent.intent_type = INTENT_0;
        intent.update(0.5, 0.1, 2000);
        assert_eq!(intent.intent_type, INTENT_3);
    }

    #[test]
    fn test_intention_bank_dominant() {
        let mut bank = IntentionBank::new(1000);
        bank.intentions[1].strength = 0.8;
        bank.tick(0.5, 0.8, 2000);
        assert_eq!(bank.dominant, INTENT_1);
    }
}
