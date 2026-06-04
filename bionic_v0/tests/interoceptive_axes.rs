//! Stage 0: interoceptive substrate unification — axis definition tests.
//!
//! These tests guard the axis coordinate system: non-overlapping ranges,
//! correct base offsets, correct total count. They are pure constant checks —
//! no runtime, no viewport — so they compile-time catch layout regressions.
//!
//! 推论5 compliance: axis names are numeric engineering locators. No test here
//! asserts that any axis is "positive" or "negative" — they are coordinates,
//! not value labels.

use bionic_v0::{
    AXIS_INTERO_BASE, AXIS_VIS_BASE, AXIS_CODE_BASE, AXIS_COUNT,
    INTERO_AXES, CODE_AXES, VIB_BANDS, VIS_AXES,
    INTERO_RESERVE, INTERO_VITAL_ANXIETY, INTERO_PAIN,
    INTERO_SELF_COHERENCE, INTERO_AGENCY, INTERO_ATTACHMENT,
    INTERO_GAP_PRESSURE, INTERO_CLOSURE, INTERO_LEARNING_PROGRESS,
    INTERO_PRESENCE,
};

/// Axis ranges are contiguous and non-overlapping:
///   [0 .. VIB_BANDS)  hearing
///   [AXIS_VIS_BASE .. AXIS_VIS_BASE+VIS_AXES)  vision
///   [AXIS_INTERO_BASE .. AXIS_INTERO_BASE+INTERO_AXES)  interoception
#[test]
fn axis_ranges_are_non_overlapping_and_cover_axis_count() {
    // Hearing       [0 .. VIB_BANDS)
    assert_eq!(AXIS_VIS_BASE, VIB_BANDS, "vision starts right after hearing");
    // Vision        [AXIS_VIS_BASE .. AXIS_INTERO_BASE)
    assert_eq!(AXIS_INTERO_BASE, VIB_BANDS + VIS_AXES, "intero starts right after vision");
    // Interoception [AXIS_INTERO_BASE .. AXIS_CODE_BASE)
    assert_eq!(AXIS_CODE_BASE, VIB_BANDS + VIS_AXES + INTERO_AXES, "code axes start right after interoception");
    // Code/digital  [AXIS_CODE_BASE .. AXIS_COUNT)
    assert_eq!(AXIS_COUNT, VIB_BANDS + VIS_AXES + INTERO_AXES + CODE_AXES, "AXIS_COUNT = all four bands");
    // No gap between bands
    assert_eq!(AXIS_COUNT, 39, "expected 4+20+10+5=39 axes total");
}

/// All ten interoceptive axis indices fall in [AXIS_INTERO_BASE, AXIS_COUNT).
#[test]
fn interoceptive_indices_inside_intero_band() {
    let axes = [
        INTERO_RESERVE, INTERO_VITAL_ANXIETY, INTERO_PAIN,
        INTERO_SELF_COHERENCE, INTERO_AGENCY, INTERO_ATTACHMENT,
        INTERO_GAP_PRESSURE, INTERO_CLOSURE, INTERO_LEARNING_PROGRESS,
        INTERO_PRESENCE,
    ];
    for &ax in &axes {
        assert!(ax >= AXIS_INTERO_BASE, "axis {ax} must be >= AXIS_INTERO_BASE={AXIS_INTERO_BASE}");
        assert!(ax < AXIS_COUNT, "axis {ax} must be < AXIS_COUNT={AXIS_COUNT}");
    }
    assert_eq!(axes.len(), INTERO_AXES, "exactly INTERO_AXES axes defined");
}

/// All ten indices are distinct (no two axes share a coordinate).
#[test]
fn interoceptive_indices_are_all_distinct() {
    let mut axes = [
        INTERO_RESERVE, INTERO_VITAL_ANXIETY, INTERO_PAIN,
        INTERO_SELF_COHERENCE, INTERO_AGENCY, INTERO_ATTACHMENT,
        INTERO_GAP_PRESSURE, INTERO_CLOSURE, INTERO_LEARNING_PROGRESS,
        INTERO_PRESENCE,
    ];
    axes.sort();
    for w in axes.windows(2) {
        assert_ne!(w[0], w[1], "duplicate interoceptive axis index: {}", w[0]);
    }
}

/// Interoceptive axes do NOT collide with hearing or vision indices.
#[test]
fn interoceptive_indices_do_not_collide_with_sensory() {
    let intero = [
        INTERO_RESERVE, INTERO_VITAL_ANXIETY, INTERO_PAIN,
        INTERO_SELF_COHERENCE, INTERO_AGENCY, INTERO_ATTACHMENT,
        INTERO_GAP_PRESSURE, INTERO_CLOSURE, INTERO_LEARNING_PROGRESS,
        INTERO_PRESENCE,
    ];
    // hearing: 0..VIB_BANDS; vision: AXIS_VIS_BASE..AXIS_INTERO_BASE
    for &ax in &intero {
        assert!(ax >= VIB_BANDS + VIS_AXES,
            "axis {ax} collides with hearing/vision range");
    }
}
