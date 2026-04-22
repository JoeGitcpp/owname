// TDD tests for VIB/PHOT ingress accumulation into the action-consequence loop.
//
// Plan: docs/superpowers/plans/2026-04-22-close-action-consequence-loop.md
//
// Run order:
//   1. Tests written first → all fail (RED)
//   2. Implementation lands → all pass (GREEN)

use std::sync::Arc;

use bionic_v0::action_consequence::{tick_vib_ingress_consequence, ConsequenceWindowPhase};
use bionic_v0::viewport::Viewport;

// ─── 1. VIB high energy opens a consequence window ───────────────────────────

#[test]
fn vib_high_energy_opens_consequence_window() {
    let vp = Arc::new(Viewport::new());

    // Write high-energy VIB data directly into the ambient channel.
    {
        let mut vib_energy = vp.ambient.vib.event_energy.write().unwrap();
        *vib_energy = 0.5;
    }
    vp.ambient
        .vib
        .last_event_t_us
        .store(1_000, std::sync::atomic::Ordering::Relaxed);

    // Trigger VIB ingress accumulation.
    tick_vib_ingress_consequence(&vp, 1_000);

    let st = vp.read_action_consequence();
    assert!(
        st.n_windows >= 1,
        "VIB high energy must open a consequence window"
    );
}

// ─── 2. VIB low energy does NOT open a window ────────────────────────────────

#[test]
fn vib_low_energy_no_consequence_window() {
    let vp = Arc::new(Viewport::new());

    // Write well-below-threshold energy.
    {
        let mut vib_energy = vp.ambient.vib.event_energy.write().unwrap();
        *vib_energy = 0.0001;
    }
    vp.ambient
        .vib
        .last_event_t_us
        .store(1_000, std::sync::atomic::Ordering::Relaxed);

    tick_vib_ingress_consequence(&vp, 1_000);

    let st = vp.read_action_consequence();
    assert_eq!(st.n_windows, 0, "VIB low energy must not open a window");
}

// ─── 3. VIB energy at threshold boundary ─────────────────────────────────────

#[test]
fn vib_energy_at_threshold_boundary() {
    use bionic_v0::INGRESS_ACCUMULATION_THRESHOLD;
    let vp = Arc::new(Viewport::new());

    // Energy exactly at threshold — should NOT trigger (strictly greater).
    {
        let mut vib_energy = vp.ambient.vib.event_energy.write().unwrap();
        *vib_energy = INGRESS_ACCUMULATION_THRESHOLD;
    }

    tick_vib_ingress_consequence(&vp, 1_000);

    let st = vp.read_action_consequence();
    assert_eq!(
        st.n_windows, 0,
        "VIB energy at threshold must not open a window (strict >)"
    );
}

// ─── 4. VIB high energy just above threshold opens a window ──────────────────

#[test]
fn vib_energy_just_above_threshold_opens_window() {
    use bionic_v0::INGRESS_ACCUMULATION_THRESHOLD;
    let vp = Arc::new(Viewport::new());

    {
        let mut vib_energy = vp.ambient.vib.event_energy.write().unwrap();
        *vib_energy = INGRESS_ACCUMULATION_THRESHOLD + 0.0001;
    }

    tick_vib_ingress_consequence(&vp, 1_000);

    let st = vp.read_action_consequence();
    assert!(
        st.n_windows >= 1,
        "VIB energy just above threshold must open a window"
    );
}

// ─── 5. Multiple VIB ticks extend the same window ────────────────────────────

#[test]
fn vib_multiple_ticks_extend_same_window() {
    let vp = Arc::new(Viewport::new());

    {
        let mut vib_energy = vp.ambient.vib.event_energy.write().unwrap();
        *vib_energy = 0.5;
    }

    let wid0 = tick_vib_ingress_consequence(&vp, 1_000);
    let wid1 = tick_vib_ingress_consequence(&vp, 1_200);
    let wid2 = tick_vib_ingress_consequence(&vp, 1_400);

    let st = vp.read_action_consequence();
    assert_eq!(
        st.n_windows, 1,
        "multiple VIB ticks must extend the same window"
    );
    assert_eq!(wid0, wid1, "same window_id on second tick");
    assert_eq!(wid1, wid2, "same window_id on third tick");
    assert_eq!(
        st.windows[0].end_t_us, 1_400,
        "end_t_us must advance to latest tick"
    );
}

// ─── 6. VIB window is in Open phase after accumulation ───────────────────────

#[test]
fn vib_window_is_open_after_accumulation() {
    let vp = Arc::new(Viewport::new());

    {
        let mut vib_energy = vp.ambient.vib.event_energy.write().unwrap();
        *vib_energy = 0.3;
    }

    tick_vib_ingress_consequence(&vp, 1_000);

    let st = vp.read_action_consequence();
    assert_eq!(
        st.windows[0].phase,
        ConsequenceWindowPhase::Open,
        "VIB accumulation must create an Open window"
    );
}

// ─── 7. VIB accumulation produces no pairs on its own ────────────────────────

#[test]
fn vib_accumulation_no_pairs_without_write_epoch() {
    let vp = Arc::new(Viewport::new());

    {
        let mut vib_energy = vp.ambient.vib.event_energy.write().unwrap();
        *vib_energy = 0.4;
    }

    tick_vib_ingress_consequence(&vp, 1_000);

    let st = vp.read_action_consequence();
    assert_eq!(
        st.n_pairs, 0,
        "VIB accumulation alone must not produce pairs"
    );
}

// ─── 8. VIB window closes via quiescence tick ────────────────────────────────

#[test]
fn vib_window_closes_via_quiescence() {
    let vp = Arc::new(Viewport::new());

    {
        let mut vib_energy = vp.ambient.vib.event_energy.write().unwrap();
        *vib_energy = 0.5;
    }

    tick_vib_ingress_consequence(&vp, 1_000);

    // Quiescence: last_update = 1000, tick at 2000 with quiescence_us = 500
    vp.tick_ingress_accumulation(2_000, 500);

    let st = vp.read_action_consequence();
    assert_eq!(
        st.windows[0].phase,
        ConsequenceWindowPhase::Closed,
        "quiescence must close the VIB-opened window"
    );
}
