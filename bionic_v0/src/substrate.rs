//! Shared substrate primitives.
//!
//! These two operations were independently re-implemented in ~13 places across
//! body-owned modules (each `unit()`/`ema()` identical in meaning, differing
//! only in parameter names). Per the emergence principle — "one integrated
//! substrate observed from many angles, not many isolated re-implementations" —
//! they are unified here. This is a pure refactor: identical numeric semantics,
//! no behavior change. Callers read this one substrate instead of reinventing it.

/// Clamp a value into the unit interval [0,1]; non-finite maps to 0.0.
/// This is the canonical numeric form for every bounded substrate signal.
#[inline]
pub fn unit(value: f64) -> f64 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Exponential moving average, kept inside the unit interval.
/// `alpha` in [0,1] is the weight of the new sample; the result is `unit()`-ed
/// so an EMA over bounded signals stays bounded.
#[inline]
pub fn ema(previous: f64, next: f64, alpha: f64) -> f64 {
    unit((1.0 - alpha) * previous + alpha * next)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_clamps_and_rejects_nonfinite() {
        assert_eq!(unit(0.5), 0.5);
        assert_eq!(unit(-1.0), 0.0);
        assert_eq!(unit(2.0), 1.0);
        assert_eq!(unit(f64::NAN), 0.0);
        assert_eq!(unit(f64::INFINITY), 0.0);
        assert_eq!(unit(f64::NEG_INFINITY), 0.0);
    }

    #[test]
    fn ema_blends_and_stays_bounded() {
        // alpha=0 keeps previous; alpha=1 takes next.
        assert_eq!(ema(0.2, 0.8, 0.0), 0.2);
        assert_eq!(ema(0.2, 0.8, 1.0), 0.8);
        // midpoint
        assert!((ema(0.0, 1.0, 0.5) - 0.5).abs() < 1e-12);
        // stays in [0,1] even if inputs would overshoot
        assert_eq!(ema(1.0, 5.0, 0.5), 1.0);
    }
}
