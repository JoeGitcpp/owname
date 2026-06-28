use bionic_v0::viewport::Viewport;
use bionic_v0::{MATURITY_PROGRESS_GAIN, MATURITY_PROGRESS_STEP_MAX};
use std::sync::Arc;

// ── P0: maturity invariants ─────────────────────────────────────────
// These tests guard the developmental clock against the class of bugs
// that produced 12/34 of recent fix commits (maturity overshoot, treadmill,
// hallucinated maturity).

#[test]
fn maturity_starts_at_zero() {
    let vp = Viewport::new();
    assert!((vp.developmental_maturity() - 0.0).abs() < 1e-15);
}

#[test]
fn maturity_never_exceeds_one_under_extreme_advance() {
    let vp = Viewport::new();
    for _ in 0..100_000 {
        vp.advance_maturity(1.0);
    }
    assert!(
        vp.developmental_maturity() <= 1.0,
        "maturity exceeded 1.0: {}",
        vp.developmental_maturity()
    );
}

#[test]
fn maturity_never_goes_negative() {
    let vp = Viewport::new();
    vp.advance_maturity(-999.0);
    assert!(
        vp.developmental_maturity() >= 0.0,
        "maturity went negative: {}",
        vp.developmental_maturity()
    );
}

#[test]
fn maturity_never_exceeds_one_under_extreme_progress() {
    let vp = Viewport::new();
    for _ in 0..100_000 {
        vp.advance_maturity_by_progress(1.0);
    }
    assert!(
        vp.developmental_maturity() <= 1.0,
        "maturity exceeded 1.0 via progress: {}",
        vp.developmental_maturity()
    );
}

#[test]
fn maturity_monotonically_increases_under_positive_closure() {
    let vp = Viewport::new();
    let mut prev = vp.developmental_maturity();
    for i in 0..1000 {
        vp.advance_maturity(0.5);
        let curr = vp.developmental_maturity();
        assert!(
            curr >= prev,
            "maturity decreased at step {i}: {prev} -> {curr}"
        );
        prev = curr;
    }
}

#[test]
fn maturity_monotonically_increases_under_positive_progress() {
    let vp = Viewport::new();
    let mut prev = vp.developmental_maturity();
    for i in 0..1000 {
        vp.advance_maturity_by_progress(0.01);
        let curr = vp.developmental_maturity();
        assert!(
            curr >= prev,
            "maturity decreased at step {i}: {prev} -> {curr}"
        );
        prev = curr;
    }
}

#[test]
fn maturity_step_shrinks_as_maturity_grows() {
    let vp = Viewport::new();
    vp.restore_maturity(0.1);
    let step_low = {
        let before = vp.developmental_maturity();
        vp.advance_maturity(1.0);
        vp.developmental_maturity() - before
    };

    vp.restore_maturity(0.9);
    let step_high = {
        let before = vp.developmental_maturity();
        vp.advance_maturity(1.0);
        vp.developmental_maturity() - before
    };

    assert!(
        step_low > step_high,
        "step should shrink with maturity: low_mat_step={step_low}, high_mat_step={step_high}"
    );
}

#[test]
fn progress_step_max_prevents_single_tick_overshoot() {
    let vp = Viewport::new();
    vp.restore_maturity(0.0);
    vp.advance_maturity_by_progress(1000.0);
    let m = vp.developmental_maturity();
    let max_possible = MATURITY_PROGRESS_STEP_MAX * MATURITY_PROGRESS_GAIN;
    assert!(
        m <= max_possible + 1e-9,
        "single progress tick exceeded step cap: m={m}, cap={max_possible}"
    );
}

#[test]
fn restore_then_advance_is_consistent() {
    let vp = Viewport::new();
    vp.restore_maturity(0.5);
    assert!(
        (vp.developmental_maturity() - 0.5).abs() < 1e-9,
        "restore_maturity(0.5) failed"
    );
    vp.advance_maturity_by_progress(0.01);
    assert!(
        vp.developmental_maturity() > 0.5,
        "advance after restore did not increase maturity"
    );
}

#[test]
fn restore_clamps_out_of_range() {
    let vp = Viewport::new();
    vp.restore_maturity(5.0);
    assert!(vp.developmental_maturity() <= 1.0);
    vp.restore_maturity(-3.0);
    assert!(vp.developmental_maturity() >= 0.0);
}

#[test]
fn zero_closure_is_noop() {
    let vp = Viewport::new();
    vp.restore_maturity(0.3);
    let before = vp.developmental_maturity();
    vp.advance_maturity(0.0);
    assert!(
        (vp.developmental_maturity() - before).abs() < 1e-15,
        "zero closure should be no-op"
    );
}

#[test]
fn negative_progress_is_noop() {
    let vp = Viewport::new();
    vp.restore_maturity(0.3);
    let before = vp.developmental_maturity();
    vp.advance_maturity_by_progress(-1.0);
    assert!(
        (vp.developmental_maturity() - before).abs() < 1e-15,
        "negative progress should be no-op"
    );
}

// ── P1: concurrent maturity access ──────────────────────────────────

#[test]
fn concurrent_maturity_advance_never_panics() {
    let vp = Arc::new(Viewport::new());
    let mut handles = vec![];

    for _ in 0..8 {
        let vp = vp.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..10_000 {
                vp.advance_maturity_by_progress(0.001);
                let _ = vp.developmental_maturity();
            }
        }));
    }

    for h in handles {
        h.join().expect("thread panicked");
    }

    let m = vp.developmental_maturity();
    assert!(m >= 0.0 && m <= 1.0, "maturity out of range after concurrent access: {m}");
}
