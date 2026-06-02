//! 改造1: the voicing gate is LEARNED, not a hardcoded 0.40.
//!
//! Old: `reflex_pressure <= 0.40` was a fixed wall — voice or not decided by a
//! pulled-from-thin-air constant. New: the gate lowers as "voicing
//! effectiveness" (prediction_familiarity, accumulated when voicing brought a
//! closure) grows. A body that finds voicing useful gets easier to voice (爱叫);
//! one whose voicing falls on deaf ears stays at the cautious baseline (沉默).
//!
//! Backward-compatible: a newborn (familiarity = 0) keeps the 0.40 baseline, so
//! existing reflex behavior is unchanged at birth.
//!
//! Contracts:
//!   1. Newborn (familiarity=0): a drive that yields reflex_pressure just under
//!      the old 0.40 wall still does NOT voice — baseline unchanged.
//!   2. Experienced (high familiarity): the SAME sub-0.40 drive now DOES voice —
//!      the gate has lowered because voicing proved effective.

use bionic_v0::vocal_reflex::{VocalReflexDrive, VocalReflexState};

// A moderate drive whose resulting reflex_pressure lands just below 0.40 at
// birth (so it sits right around the old fixed wall).
fn moderate_drive() -> VocalReflexDrive {
    VocalReflexDrive {
        drive_pressure: 0.35,
        energy: 0.5,
    }
}

#[test]
fn newborn_gate_matches_old_baseline() {
    let mut s = VocalReflexState::default();
    // familiarity = 0, suppression = 0 → gate at the old 0.40 baseline.
    // efficacy = 0 → gate at 0.40 baseline.
    let burst = s.tick_drive(1_000_000, moderate_drive(), 0.0, 0.0, 0.0);
    assert!(
        burst.is_none(),
        "newborn must keep the 0.40 baseline gate (silent just under it)"
    );
}

#[test]
fn experienced_voicing_lowers_the_gate() {
    let mut s = VocalReflexState::default();
    // High vocal_efficacy → learned lower gate (last arg). The same moderate
    // drive that was silent at birth should now voice. familiarity/suppression
    // left at 0 to isolate the efficacy effect.
    let burst = s.tick_drive(2_000_000, moderate_drive(), 0.0, 0.0, 0.9);
    assert!(
        burst.is_some(),
        "high vocal_efficacy must lower the gate so the same drive voices"
    );
}
