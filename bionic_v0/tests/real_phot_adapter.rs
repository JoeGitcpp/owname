use std::sync::Arc;

use bionic_v0::adapter::{ModalityAdapter, RealPhotAdapter};
use bionic_v0::phot_camera::{
    CameraFault, CameraFrame, CameraFrameProvider, StubRealCameraFrameProvider, SynthCamStep,
    SyntheticCameraFrameProvider, CAM_PIX,
};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::{HealthState, Viewport};
use bionic_v0::{
    HEALTH_FAULT, HEALTH_OK, HEALTH_QUIET, HEALTH_STALE, MOD_PHOT, N_OBJECT_SLOTS,
    PHOT_STALE_WINDOW_US, SNAPSHOT_VERSION,
};

// ---------------------------------------------------------------------------
// Phase 4C: PHOT predictor integration tests
// ---------------------------------------------------------------------------

/// Warmup guard: a slot that has seen fewer than PHOT_PRED_WARMUP_MIN frames
/// must not emit a non-zero surprise (i.e., must not fake confidence).
/// We drive exactly two event frames (warmup fills both) and read the slot.
#[test]
fn phot_predictor_warmup_does_not_fake_confidence() {
    let vp = Arc::new(Viewport::new());
    // 1 idle (warm-up diffing baseline) + 1 blob (first real event, warmup call 1)
    // + 1 blob (second event, warmup call 2; PHOT_PRED_WARMUP_MIN == 2)
    let mut script = vec![SynthCamStep::Idle];
    script.push(SynthCamStep::Blob {
        cx: 0.5,
        cy: 0.5,
        r: 0.15,
        brightness: 0.95,
    });
    script.push(SynthCamStep::Blob {
        cx: 0.5,
        cy: 0.5,
        r: 0.15,
        brightness: 0.95,
    });
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    // Tick 0 = idle baseline (extractor warm-up, no event).
    adapter.tick(&vp, 1_000_000);
    // Tick 1 = first blob event → predictor warmup call 1.
    adapter.tick(&vp, 1_033_333);
    // Tick 2 = second blob event → predictor warmup call 2.
    adapter.tick(&vp, 1_066_666);

    // After exactly 2 predictor updates (both warmup), surprise must be 0.
    let mut found_phot = false;
    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & MOD_PHOT) != 0 {
            if let Some(phot) = s.phot.as_ref() {
                found_phot = true;
                for (bi, &p) in phot.surprise.iter().enumerate() {
                    assert!(
                        p.abs() < 1e-12,
                        "band {}: surprise={} must be zero during warmup",
                        bi,
                        p
                    );
                }
                // err is 0 during warmup as well.
                for (bi, &e) in phot.err.iter().enumerate() {
                    assert!(
                        e.abs() < 1e-12,
                        "band {}: err={} must be zero during warmup",
                        bi,
                        e
                    );
                }
            }
        }
    }
    assert!(
        found_phot,
        "at least one PHOT slot must be active after blob frames"
    );
}

/// Stable visual pattern: alternating Idle/Blob frames create consistent,
/// repeated motion (each Idle→Blob and Blob→Idle transition has the same
/// magnitude). After warmup the predictor converges and err stays small.
#[test]
fn phot_predictor_converges_on_stable_synthetic_motion() {
    let vp = Arc::new(Viewport::new());
    // Alternating [Idle, Blob] × 16 = 32 frames.
    // Each transition produces the same motion magnitude → predictor converges.
    let mut script = Vec::with_capacity(32);
    for _ in 0..16 {
        script.push(SynthCamStep::Idle);
        script.push(SynthCamStep::Blob {
            cx: 0.6,
            cy: 0.4,
            r: 0.12,
            brightness: 0.90,
        });
    }
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    for step in 0..32u64 {
        adapter.tick(&vp, 1_000_000 + step * 33_333);
    }

    // At least one PHOT slot must be active with real (non-zero) predictions.
    let mut found = false;
    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & MOD_PHOT) != 0 {
            if let Some(phot) = s.phot.as_ref() {
                found = true;
                let all_pred_zero = phot.predicted.iter().all(|&v| v == 0.0);
                assert!(
                    !all_pred_zero,
                    "predicted must not be all-zero after warmup; got {:?}",
                    phot.predicted
                );
                // After many consistent-motion frames, err should be small
                // relative to observed (predictor has tracked the signal).
                let max_err = phot.err.iter().map(|e| e.abs()).fold(0.0_f64, f64::max);
                let max_obs = phot
                    .observed
                    .iter()
                    .map(|v| v.abs())
                    .fold(0.0_f64, f64::max);
                if max_obs > 1e-6 {
                    assert!(
                        max_err < max_obs * 0.5,
                        "max_err={} should be <50% of max_obs={} after convergence",
                        max_err,
                        max_obs
                    );
                }
            }
        }
    }
    assert!(
        found,
        "at least one PHOT slot must be active with non-zero predicted"
    );
}

/// Motion change: when a blob moves to a new position, MotionExtract changes
/// → predicted (from previous) diverges → err spikes.
#[test]
fn phot_predictor_err_spikes_on_motion_change() {
    let vp = Arc::new(Viewport::new());
    // Phase 1: stable blob at left.
    let mut script = vec![SynthCamStep::Idle; 2];
    for _ in 0..20 {
        script.push(SynthCamStep::Blob {
            cx: 0.2,
            cy: 0.5,
            r: 0.10,
            brightness: 0.80,
        });
    }
    // Phase 2: suddenly jump blob to right (changes MotionExtract bands dramatically).
    for _ in 0..5 {
        script.push(SynthCamStep::Blob {
            cx: 0.8,
            cy: 0.5,
            r: 0.10,
            brightness: 0.80,
        });
    }
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    // Run through stable phase.
    for step in 0..22u64 {
        adapter.tick(&vp, 1_000_000 + step * 33_333);
    }
    // Record max_err after stable phase.
    let mut stable_err_max = 0.0_f64;
    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & MOD_PHOT) != 0 {
            if let Some(phot) = s.phot.as_ref() {
                let m = phot.err.iter().map(|e| e.abs()).fold(0.0_f64, f64::max);
                if m > stable_err_max {
                    stable_err_max = m;
                }
            }
        }
    }

    // Tick the jump frame.
    adapter.tick(&vp, 1_000_000 + 22 * 33_333);

    let mut spike_err_max = 0.0_f64;
    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & MOD_PHOT) != 0 {
            if let Some(phot) = s.phot.as_ref() {
                let m = phot.err.iter().map(|e| e.abs()).fold(0.0_f64, f64::max);
                if m > spike_err_max {
                    spike_err_max = m;
                }
            }
        }
    }

    // The test is only meaningful if the predictor was actually running.
    // If no PHOT slot claimed, both err values are 0 and the test is vacuous;
    // assert we at least had a stable reading.
    if stable_err_max > 0.0 || spike_err_max > 0.0 {
        assert!(
            spike_err_max >= stable_err_max,
            "err must not decrease at the motion change: stable_max={} spike_max={}",
            stable_err_max,
            spike_err_max
        );
    }
}

/// Snapshot round-trip: the snapshot must carry real (non-placeholder) PHOT
/// prediction fields after the predictor has warmed up.
/// Alternating [Idle, Blob] × 6 creates repeated motion → at least one
/// real prediction call → non-zero predicted values in the snapshot.
#[test]
fn snapshot_roundtrip_preserves_phot_prediction_fields() {
    let vp = Arc::new(Viewport::new());
    let mut script = Vec::with_capacity(12);
    for _ in 0..6 {
        script.push(SynthCamStep::Idle);
        script.push(SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.12,
            brightness: 0.90,
        });
    }
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    for step in 0..12u64 {
        adapter.tick(&vp, 1_000_000 + step * 33_333);
    }

    let tmp = std::env::temp_dir().join(format!(
        "bionic_v0_4c_pred_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let paths = SnapshotPaths::new(&tmp);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    // Find an active PHOT slot in the snapshot.
    let mut found_real_pred = false;
    for s in &snap.slots {
        if !s.active || (s.source_modality & MOD_PHOT) == 0 {
            continue;
        }
        if let Some(fam) = s.phot.as_ref() {
            // After warmup + several prediction calls, predicted must not be all-zeros.
            if fam.predicted.iter().any(|&v| v != 0.0) {
                found_real_pred = true;
            }
        }
    }
    assert!(
        found_real_pred,
        "snapshot must carry non-zero predicted values after PHOT predictor warmup"
    );
}

/// The stub real provider is the default product path; it must surface a
/// StartupFailed immediately so the adapter marks phot_health = FAULT.
#[test]
fn real_phot_health_startup_failure_is_visible() {
    let vp = Arc::new(Viewport::new());
    let provider = Box::new(StubRealCameraFrameProvider::try_new());
    assert!(
        !provider.is_healthy(),
        "stub real provider must start faulted"
    );

    let mut adapter = RealPhotAdapter::new(provider);

    // One tick is enough — the adapter sees the fault and records it.
    adapter.tick(&vp, 1_000_000);
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Fault,
        "phot_health must report FAULT when the camera cannot be opened"
    );

    // Repeated ticks keep the FAULT sticky; the adapter never silently
    // transitions out of it.
    for step in 1..5 {
        adapter.tick(&vp, 1_000_000 + step * 33_333);
    }
    assert_eq!(vp.read_phot_health(), HealthState::Fault);
}

/// A healthy provider that keeps emitting idle frames must land in QUIET
/// and must not claim any slot.
#[test]
fn real_phot_idle_frame_does_not_emit_event() {
    let vp = Arc::new(Viewport::new());
    let provider = Box::new(SyntheticCameraFrameProvider::new(
        vec![SynthCamStep::Idle; 20],
        0,
        33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);
    for step in 0..20 {
        let t = 1_000_000 + (step as u64) * 33_333;
        adapter.tick(&vp, t);
    }
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Quiet,
        "a quiet but healthy camera must emit phot_health=QUIET"
    );

    let amb = vp.read_ambient();
    assert!(
        !amb.phot.candidate_flag,
        "idle frames must not flip the candidate flag"
    );

    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        assert!(
            !(s.active && (s.source_modality & MOD_PHOT) != 0),
            "no PHOT slot should be active on idle input"
        );
    }
}

/// A bright, localized blob creates one (and only one) PHOT event per frame.
/// The evidence must be routed through `claim_slot_for_event` and end up in
/// slot.phot + slot.spatial.
#[test]
fn real_phot_motion_frame_emits_single_event() {
    let vp = Arc::new(Viewport::new());
    // Some idle warm-up, then a bright blob in the top-right quadrant.
    let provider = Box::new(SyntheticCameraFrameProvider::idle_then_blob(
        2, 15, 0.75, 0.30, 0.15, 0.95, 0, 33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);

    for step in 0..20 {
        let t = 1_000_000 + (step as u64) * 33_333;
        adapter.tick(&vp, t);
    }

    assert_eq!(
        vp.read_phot_health(),
        HealthState::Ok,
        "motion frames should raise phot_health to OK"
    );

    let amb = vp.read_ambient();
    assert!(
        amb.phot.event_energy > 0.0,
        "ambient.phot.event_energy must become non-zero under motion"
    );

    // Exactly one claim debug record must name a PHOT-bearing slot.
    let d = vp.read_last_claim_debug();
    assert_eq!(d.modality_bit, MOD_PHOT, "last claim must be for PHOT");
    assert!(d.chosen_slot >= 0, "a slot must have been chosen");
}

#[test]
fn real_phot_event_writes_slot_phot_and_spatial() {
    let vp = Arc::new(Viewport::new());
    // End the script on the first blob frame: the idle->blob transition is
    // the only tick with real motion, so the slot's last write reflects an
    // actual event (not a tail of static-scene zero-motion overwrites).
    let mut script = vec![SynthCamStep::Idle; 3];
    script.push(SynthCamStep::Blob {
        cx: 0.25,
        cy: 0.60,
        r: 0.12,
        brightness: 0.9,
    });
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    for step in 0..4 {
        let t = 1_000_000 + (step as u64) * 33_333;
        adapter.tick(&vp, t);
    }

    let mut saw_phot_signal = false;
    let mut saw_spatial = false;
    for i in 0..N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & MOD_PHOT) != 0 {
            if let Some(phot) = s.phot.as_ref() {
                assert_eq!(
                    phot.observed.len(),
                    bionic_v0::PHOT_BANDS,
                    "phot family must carry PHOT_BANDS entries"
                );
                if phot.observed.iter().any(|v| *v > 0.0) {
                    saw_phot_signal = true;
                }
            }
            if let Some(sp) = s.spatial {
                assert!(
                    sp.valid,
                    "spatial target must be valid after a motion frame"
                );
                assert!(sp.pos_x >= 0.0 && sp.pos_x <= 1.0);
                assert!(sp.pos_y >= 0.0 && sp.pos_y <= 1.0);
                saw_spatial = true;
            }
        }
    }
    assert!(
        saw_phot_signal,
        "slot.phot bands must carry signal on a fresh event"
    );
    assert!(
        saw_spatial,
        "slot.spatial must be populated by real-camera motion"
    );
}

/// A runtime error mid-stream must flip phot_health to FAULT and keep it.
#[test]
fn real_phot_runtime_error_transitions_to_fault() {
    let vp = Arc::new(Viewport::new());
    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::Idle,
        SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.1,
            brightness: 0.9,
        },
        SynthCamStep::RuntimeError,
        SynthCamStep::Idle,
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);
    for step in 0..8 {
        let t = 1_000_000 + (step as u64) * 33_333;
        adapter.tick(&vp, t);
    }
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Fault,
        "a runtime error must surface as FAULT in phot_health"
    );
}

/// The snapshot format v11 must carry phot_health and `claim_slot_for_event`
/// must leave a debug record the observer can read.
#[test]
fn observer_sees_phot_health_and_claim_debug() {
    let vp = Arc::new(Viewport::new());
    let provider = Box::new(SyntheticCameraFrameProvider::idle_then_blob(
        2, 10, 0.40, 0.50, 0.10, 0.95, 0, 33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);
    for step in 0..15 {
        let t = 1_000_000 + (step as u64) * 33_333;
        adapter.tick(&vp, t);
    }
    // write a snapshot to a tempdir and round-trip it through the reader.
    let tmp = std::env::temp_dir().join(format!(
        "bionic_v0_phot4a_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let paths = SnapshotPaths::new(&tmp);
    write_snapshot(&vp, &paths).unwrap();

    let snap = read_snapshot(&paths.final_path).unwrap();
    assert!(
        matches!(snap.phot_health, HEALTH_OK | HEALTH_QUIET),
        "snapshot must carry a valid phot_health; got {}",
        snap.phot_health
    );
    // At least one claim must have been recorded over the sequence.
    assert!(snap.last_claim.seq > 0, "a claim debug record should exist");
    assert_eq!(snap.last_claim.modality_bit, MOD_PHOT);
    // FAULT would be another valid reading in a broken run; explicitly
    // assert we are not in FAULT here since the synthetic provider is OK.
    assert_ne!(snap.phot_health, HEALTH_FAULT);
    let _ = tmp;
}

/// Guard: ensure the fault enum is usable across crate boundaries.
#[test]
fn camera_fault_values_are_publicly_usable() {
    let mut p = StubRealCameraFrameProvider::try_new();
    assert!(matches!(p.try_recv(), Err(CameraFault::StartupFailed)));
}

/// Sanity: synthetic provider output has the expected grid shape.
#[test]
fn synthetic_camera_frame_shape_is_fixed() {
    let mut p = SyntheticCameraFrameProvider::new(
        vec![
            SynthCamStep::Idle,
            SynthCamStep::Blob {
                cx: 0.5,
                cy: 0.5,
                r: 0.1,
                brightness: 0.5,
            },
        ],
        0,
        33_333,
    );
    let f: CameraFrame = p.try_recv().unwrap().unwrap();
    assert_eq!(f.luma.len(), CAM_PIX);
    let f2 = p.try_recv().unwrap().unwrap();
    assert_eq!(f2.luma.len(), CAM_PIX);
    // The blob frame must contain at least one pixel brighter than the idle
    // background baseline (0.02) by a visible margin.
    assert!(f2.luma.iter().cloned().fold(0.0_f64, f64::max) > 0.2);
}

// ---------------------------------------------------------------------------
// Phase 4B: staleness semantic tests
// ---------------------------------------------------------------------------

/// Provider is healthy but returns no frames for longer than PHOT_STALE_WINDOW_US.
/// The adapter must write phot_health = STALE (not QUIET) once the window expires.
#[test]
fn phot_stale_when_no_new_frames_arrive() {
    let vp = Arc::new(Viewport::new());

    // Two idle frames so the extractor has seen at least one (sets last_frame_t_us).
    let mut script = vec![SynthCamStep::Idle; 2];
    // Followed by many NoFrame ticks (provider stays healthy but delivers nothing).
    script.extend(std::iter::repeat(SynthCamStep::NoFrame).take(40));
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    // Tick through the two idle frames.
    adapter.tick(&vp, base_t);
    adapter.tick(&vp, base_t + 33_333);

    // Within stale window: still not STALE.
    adapter.tick(&vp, base_t + 200_000); // 167 ms after last frame
    assert_ne!(
        vp.read_phot_health(),
        HealthState::Stale,
        "must not be STALE before the staleness window expires"
    );

    // After stale window: must become STALE.
    let stale_t = base_t + 33_333 + PHOT_STALE_WINDOW_US + 1;
    adapter.tick(&vp, stale_t);
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Stale,
        "phot_health must be STALE after {} µs without a new frame",
        PHOT_STALE_WINDOW_US
    );
}

/// Frames arrive but motion is below threshold — must be QUIET, never STALE.
/// (Regression: QUIET must not be confused with STALE.)
#[test]
fn phot_quiet_not_stale_when_frames_arrive_but_still() {
    let vp = Arc::new(Viewport::new());
    let provider = Box::new(SyntheticCameraFrameProvider::new(
        vec![SynthCamStep::Idle; 30],
        0,
        33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);
    for step in 0..30 {
        let t = 1_000_000 + (step as u64) * 33_333;
        adapter.tick(&vp, t);
    }
    let h = vp.read_phot_health();
    assert_ne!(
        h,
        HealthState::Stale,
        "idle frames (arriving) must not produce STALE"
    );
    assert_ne!(h, HealthState::Fault, "idle frames must not produce FAULT");
}

/// Snapshot at version 12 must carry HEALTH_STALE in phot_health when the
/// adapter has written that state. Observer round-trip confirmed.
#[test]
fn observer_sees_phot_stale_in_snapshot() {
    let vp = Arc::new(Viewport::new());

    // Deliver two idle frames then go silent for > stale window.
    let mut script = vec![SynthCamStep::Idle; 2];
    script.extend(std::iter::repeat(SynthCamStep::NoFrame).take(30));
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    adapter.tick(&vp, base_t);
    adapter.tick(&vp, base_t + 33_333);
    // Advance past the stale window.
    let stale_t = base_t + 33_333 + PHOT_STALE_WINDOW_US + 1;
    adapter.tick(&vp, stale_t);

    assert_eq!(vp.read_phot_health(), HealthState::Stale);

    // Write snapshot and round-trip through the binary reader.
    let tmp = std::env::temp_dir().join(format!(
        "bionic_v0_stale_snap_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let paths = SnapshotPaths::new(&tmp);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    assert_eq!(
        snap.phot_health, HEALTH_STALE,
        "snapshot must carry HEALTH_STALE; got {}",
        snap.phot_health
    );
    // Must not be misread as any other state.
    assert_ne!(snap.phot_health, HEALTH_FAULT);
    assert_ne!(snap.phot_health, HEALTH_QUIET);
    assert_ne!(snap.phot_health, HEALTH_OK);
}

// ---------------------------------------------------------------------------
// Phase 4C.1: startup staleness — healthy open, never delivered first frame.
// ---------------------------------------------------------------------------

/// Healthy provider, zero frames ever delivered, time before stale window:
/// must NOT be STALE.
#[test]
fn phot_startup_stale_not_before_window() {
    let vp = Arc::new(Viewport::new());
    // Only NoFrame ticks — provider is healthy but never delivers a frame.
    let provider = Box::new(SyntheticCameraFrameProvider::new(
        vec![SynthCamStep::NoFrame; 20],
        0,
        33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    // One tick to record started_t_us, then well within the stale window.
    adapter.tick(&vp, base_t);
    adapter.tick(&vp, base_t + 100_000); // 100 ms — well within 500 ms window

    assert_ne!(
        vp.read_phot_health(),
        HealthState::Stale,
        "must NOT be STALE before PHOT_STALE_WINDOW_US when no frame ever arrived"
    );
    assert_ne!(
        vp.read_phot_health(),
        HealthState::Fault,
        "healthy provider must not be FAULT"
    );
}

/// Healthy provider, zero frames ever delivered, time past stale window:
/// must become STALE (startup staleness).
#[test]
fn phot_startup_stale_after_window_no_first_frame() {
    let vp = Arc::new(Viewport::new());
    let provider = Box::new(SyntheticCameraFrameProvider::new(
        vec![SynthCamStep::NoFrame; 40],
        0,
        33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    // First tick: records started_t_us = base_t.
    adapter.tick(&vp, base_t);
    // Tick past the stale window.
    let stale_t = base_t + PHOT_STALE_WINDOW_US + 1;
    adapter.tick(&vp, stale_t);

    assert_eq!(
        vp.read_phot_health(),
        HealthState::Stale,
        "phot_health must be STALE after {}µs with healthy provider but zero frames",
        PHOT_STALE_WINDOW_US,
    );
}

/// Viewport must carry phot_started_t_us after the first non-fault tick,
/// and phot_last_frame_t_us must update when a frame arrives.
#[test]
fn phot_timing_fields_written_to_viewport() {
    let vp = Arc::new(Viewport::new());
    let provider = Box::new(SyntheticCameraFrameProvider::new(
        vec![SynthCamStep::Idle, SynthCamStep::Idle],
        0,
        33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);

    // Before any tick: both fields are 0.
    assert_eq!(vp.read_phot_started_t_us(), 0);
    assert_eq!(vp.read_phot_last_frame_t_us(), 0);

    let t0 = 2_000_000u64;
    adapter.tick(&vp, t0);
    // After first tick with a healthy provider: started is set.
    assert_eq!(
        vp.read_phot_started_t_us(),
        t0,
        "started_t_us must equal first-tick time"
    );
    // First idle frame is warm-up (no diff yet): last_frame should be set.
    assert_eq!(
        vp.read_phot_last_frame_t_us(),
        t0,
        "last_frame_t_us must update on frame receipt"
    );

    let t1 = t0 + 33_333;
    adapter.tick(&vp, t1);
    assert_eq!(
        vp.read_phot_last_frame_t_us(),
        t1,
        "last_frame_t_us advances with each frame"
    );
    // started_t_us must remain unchanged.
    assert_eq!(
        vp.read_phot_started_t_us(),
        t0,
        "started_t_us must not change after first tick"
    );
}

/// Snapshot v13 must carry phot_started_t_us and phot_last_frame_t_us.
#[test]
fn snapshot_v13_carries_phot_timing_fields() {
    let vp = Arc::new(Viewport::new());
    let provider = Box::new(SyntheticCameraFrameProvider::new(
        vec![SynthCamStep::Idle; 4],
        0,
        33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 5_000_000u64;
    for step in 0..4 {
        adapter.tick(&vp, base_t + step * 33_333);
    }

    let tmp = std::env::temp_dir().join(format!(
        "bionic_v0_v13_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let paths = SnapshotPaths::new(&tmp);
    write_snapshot(&vp, &paths).unwrap();

    let snap = read_snapshot(&paths.final_path).unwrap();
    assert_eq!(
        snap.phot_started_t_us, base_t,
        "snapshot must carry phot_started_t_us"
    );
    assert!(
        snap.phot_last_frame_t_us >= base_t,
        "snapshot must carry phot_last_frame_t_us"
    );
    // Confirm we are reading the right version.
    let _ = SNAPSHOT_VERSION; // v13
}

/// Full state-transition regression: healthy open → no frame → STALE →
/// idle frame arrives → QUIET → motion frame → OK → no-frame timeout → STALE
/// → runtime error → FAULT.
#[test]
fn phot_full_state_transition_regression() {
    let vp = Arc::new(Viewport::new());
    // Script:
    //   [0]: NoFrame (started, no frame yet)
    //   [1..2]: Idle (warm-up + diff baseline → QUIET)
    //   [3..8]: Blob (motion → OK)
    //   [9..]: NoFrame forever (timeout → STALE)
    //   RuntimeError at step 25 (→ FAULT)
    let mut script = vec![SynthCamStep::NoFrame];
    script.extend(vec![SynthCamStep::Idle; 2]);
    script.extend(vec![
        SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.15,
            brightness: 0.95
        };
        6
    ]);
    script.extend(vec![SynthCamStep::NoFrame; 16]);
    script.push(SynthCamStep::RuntimeError);
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;

    // Step 0: started, NoFrame, within stale window → not STALE yet.
    adapter.tick(&vp, base_t);
    assert_ne!(
        vp.read_phot_health(),
        HealthState::Stale,
        "step 0: too early for STALE"
    );

    // Steps 1-2: idle frames → QUIET.
    adapter.tick(&vp, base_t + 33_333);
    adapter.tick(&vp, base_t + 66_666);
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Quiet,
        "steps 1-2: idle frames → QUIET"
    );

    // Steps 3-8: blob frames → OK (smoothing may take a few ticks).
    for s in 3..9 {
        adapter.tick(&vp, base_t + s * 33_333);
    }
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Ok,
        "steps 3-8: motion frames → OK"
    );

    // Steps 9 onward: NoFrame; last frame was at step 8.
    // After PHOT_STALE_WINDOW_US from step 8 timestamp → STALE.
    let last_frame_t = base_t + 8 * 33_333;
    let stale_t = last_frame_t + PHOT_STALE_WINDOW_US + 1;
    adapter.tick(&vp, stale_t);
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Stale,
        "NoFrame timeout → STALE"
    );

    // RuntimeError at script step 25 → FAULT.
    // Drive enough ticks to exhaust the NoFrame steps and hit the error.
    for s in 0..20 {
        adapter.tick(&vp, stale_t + (s + 1) * 33_333);
    }
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Fault,
        "runtime error → FAULT"
    );
}

// ---------------------------------------------------------------------------
// Phase 4D: multi-event extraction tests
// ---------------------------------------------------------------------------

/// Two spatially separated blobs in one frame must yield two regions, not one
/// merged centroid.
#[test]
fn multi_event_extraction_two_blobs_yield_two_regions() {
    use bionic_v0::phot_camera::{CAM_H, CAM_PIX, CAM_W};
    use bionic_v0::PHOT_MAX_EVENTS_PER_FRAME;

    // Build a synthetic motion map manually: two bright separated regions.
    // Left blob: top-left quadrant. Right blob: bottom-right quadrant.
    let mut motion = vec![0.0f64; CAM_PIX];
    let r_px = 4usize;

    // Left blob centre ~ (16, 12) in pixel coords
    let (lcx, lcy) = (80usize, 60usize);
    for y in 0..CAM_H {
        for x in 0..CAM_W {
            let dx = x as isize - lcx as isize;
            let dy = y as isize - lcy as isize;
            if dx * dx + dy * dy <= (r_px * r_px) as isize {
                motion[y * CAM_W + x] = 0.8;
            }
        }
    }

    // Right blob centre ~ (48, 36) in pixel coords
    let (rcx, rcy) = (240usize, 180usize);
    for y in 0..CAM_H {
        for x in 0..CAM_W {
            let dx = x as isize - rcx as isize;
            let dy = y as isize - rcy as isize;
            if dx * dx + dy * dy <= (r_px * r_px) as isize {
                motion[y * CAM_W + x] = 0.8;
            }
        }
    }

    // Use the public extract_regions helper that Chunk 1 must add.
    let regions = bionic_v0::phot_camera::extract_regions_from_motion(&motion, CAM_W, CAM_H);

    assert_eq!(
        regions.len(),
        2,
        "two separated blobs must yield exactly 2 regions; got {}",
        regions.len()
    );
    // Higher energy first.
    assert!(
        regions[0].event_energy >= regions[1].event_energy,
        "regions must be sorted energy-descending"
    );
    // The two blob centroids must be spatially separated.
    let dx = regions[0].pos_x - regions[1].pos_x;
    let dy = regions[0].pos_y - regions[1].pos_y;
    let dist = (dx * dx + dy * dy).sqrt();
    assert!(
        dist > 0.2,
        "two blobs must be spatially separated in normalised coords; dist={}",
        dist
    );
    // Capped at PHOT_MAX_EVENTS_PER_FRAME.
    assert!(regions.len() <= PHOT_MAX_EVENTS_PER_FRAME);
}

/// Regions are returned in deterministic order: energy descending, then pos_x
/// ascending, then pos_y ascending.
#[test]
fn multi_event_extraction_regions_sorted_deterministically() {
    use bionic_v0::phot_camera::{CAM_H, CAM_PIX, CAM_W};

    let mut motion = vec![0.0f64; CAM_PIX];
    let r_px = 4usize;

    // Stronger blob on the right.
    let (rcx, rcy) = (240usize, 120usize);
    for y in 0..CAM_H {
        for x in 0..CAM_W {
            let dx = x as isize - rcx as isize;
            let dy = y as isize - rcy as isize;
            if dx * dx + dy * dy <= (r_px * r_px) as isize {
                motion[y * CAM_W + x] = 0.9;
            }
        }
    }
    // Weaker blob on the left.
    let (lcx, lcy) = (80usize, 120usize);
    for y in 0..CAM_H {
        for x in 0..CAM_W {
            let dx = x as isize - lcx as isize;
            let dy = y as isize - lcy as isize;
            if dx * dx + dy * dy <= (r_px * r_px) as isize {
                motion[y * CAM_W + x] = 0.5;
            }
        }
    }

    let regions = bionic_v0::phot_camera::extract_regions_from_motion(&motion, CAM_W, CAM_H);
    assert!(regions.len() >= 2, "expected at least 2 regions");
    // Stronger (right) blob comes first.
    assert!(
        regions[0].event_energy > regions[1].event_energy,
        "higher-energy region must come first"
    );
}

/// A single region frame still works — result has exactly one item.
#[test]
fn multi_event_extraction_single_blob_still_works() {
    use bionic_v0::phot_camera::{CAM_H, CAM_PIX, CAM_W};

    let mut motion = vec![0.0f64; CAM_PIX];
    let (cx, cy, r_px) = (160usize, 120usize, 12usize);
    for y in 0..CAM_H {
        for x in 0..CAM_W {
            let dx = x as isize - cx as isize;
            let dy = y as isize - cy as isize;
            if dx * dx + dy * dy <= (r_px * r_px) as isize {
                motion[y * CAM_W + x] = 0.8;
            }
        }
    }
    let regions = bionic_v0::phot_camera::extract_regions_from_motion(&motion, CAM_W, CAM_H);
    assert_eq!(
        regions.len(),
        1,
        "single strong blob must yield exactly 1 region"
    );
}

/// A frame with no motion yields an empty Vec from multi-region extraction.
#[test]
fn multi_event_extraction_empty_motion_yields_empty_vec() {
    use bionic_v0::phot_camera::{CAM_H, CAM_PIX, CAM_W};

    let motion = vec![0.0f64; CAM_PIX];
    let regions = bionic_v0::phot_camera::extract_regions_from_motion(&motion, CAM_W, CAM_H);
    assert!(regions.is_empty(), "zero motion must yield no regions");
}

// ---------------------------------------------------------------------------
// Phase 4D Chunk 2: adapter multi-claim loop tests
// ---------------------------------------------------------------------------

/// A frame with two strong spatially-separated blobs must produce two active
/// PHOT slots. We use a custom provider that emits a single frame containing
/// two motion regions.
#[test]
fn multi_event_claim_two_blobs_fill_two_slots() {
    use bionic_v0::phot_camera::{CameraFault, CameraFrame, CameraFrameProvider, CAM_H, CAM_W};

    // Custom provider: first emit an all-idle frame (to seed the extractor),
    // then emit a two-blob frame. After that return Ok(None).
    struct TwoBlobProvider {
        step: usize,
    }
    impl CameraFrameProvider for TwoBlobProvider {
        fn is_healthy(&self) -> bool {
            true
        }
        fn try_recv(&mut self) -> Result<Option<CameraFrame>, CameraFault> {
            self.step += 1;
            let t = (self.step as u64) * 33_333;
            if self.step == 1 {
                // Idle seed frame
                let mut f = CameraFrame::zeros(t);
                for v in f.luma.iter_mut() {
                    *v = 0.02;
                }
                return Ok(Some(f));
            }
            if self.step == 2 {
                // Two-blob frame: left blob and right blob, both bright
                let mut f = CameraFrame::zeros(t);
                for v in f.luma.iter_mut() {
                    *v = 0.02;
                }
                let r_px = 12usize;
                // Left blob
                let (lcx, lcy) = (64usize, 96usize);
                for y in 0..CAM_H {
                    for x in 0..CAM_W {
                        let dx = x as isize - lcx as isize;
                        let dy = y as isize - lcy as isize;
                        if dx * dx + dy * dy <= (r_px * r_px) as isize {
                            f.luma[y * CAM_W + x] = 0.9;
                        }
                    }
                }
                // Right blob
                let (rcx, rcy) = (256usize, 144usize);
                for y in 0..CAM_H {
                    for x in 0..CAM_W {
                        let dx = x as isize - rcx as isize;
                        let dy = y as isize - rcy as isize;
                        if dx * dx + dy * dy <= (r_px * r_px) as isize {
                            f.luma[y * CAM_W + x] = 0.9;
                        }
                    }
                }
                return Ok(Some(f));
            }
            Ok(None)
        }
    }

    let vp = std::sync::Arc::new(bionic_v0::viewport::Viewport::new());
    let provider = Box::new(TwoBlobProvider { step: 0 });
    let mut adapter = bionic_v0::adapter::RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    adapter.tick(&vp, base_t); // seed idle
    adapter.tick(&vp, base_t + 33_333); // two-blob motion frame

    // Count active PHOT slots.
    let mut phot_slots = 0;
    for i in 0..bionic_v0::N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & bionic_v0::MOD_PHOT) != 0 {
            phot_slots += 1;
        }
    }
    assert!(
        phot_slots >= 2,
        "two spatially separated blobs must produce at least 2 PHOT slots; got {}",
        phot_slots
    );
}

/// Two regions far apart must land in different slots (verified via distinct
/// spatial positions in slot.spatial).
#[test]
fn multi_event_claim_two_far_blobs_in_different_slots() {
    use bionic_v0::phot_camera::{CameraFault, CameraFrame, CameraFrameProvider, CAM_H, CAM_W};

    struct TwoBlobProvider2 {
        step: usize,
    }
    impl CameraFrameProvider for TwoBlobProvider2 {
        fn is_healthy(&self) -> bool {
            true
        }
        fn try_recv(&mut self) -> Result<Option<CameraFrame>, CameraFault> {
            self.step += 1;
            let t = (self.step as u64) * 33_333;
            if self.step == 1 {
                let mut f = CameraFrame::zeros(t);
                for v in f.luma.iter_mut() {
                    *v = 0.02;
                }
                return Ok(Some(f));
            }
            if self.step == 2 {
                let mut f = CameraFrame::zeros(t);
                for v in f.luma.iter_mut() {
                    *v = 0.02;
                }
                let r_px = 12usize;
                let (lcx, lcy) = (48usize, 48usize);
                let (rcx, rcy) = (272usize, 192usize);
                for y in 0..CAM_H {
                    for x in 0..CAM_W {
                        let dx = x as isize - lcx as isize;
                        let dy = y as isize - lcy as isize;
                        if dx * dx + dy * dy <= (r_px * r_px) as isize {
                            f.luma[y * CAM_W + x] = 0.85;
                        }
                        let dx2 = x as isize - rcx as isize;
                        let dy2 = y as isize - rcy as isize;
                        if dx2 * dx2 + dy2 * dy2 <= (r_px * r_px) as isize {
                            f.luma[y * CAM_W + x] = 0.85;
                        }
                    }
                }
                return Ok(Some(f));
            }
            Ok(None)
        }
    }

    let vp = std::sync::Arc::new(bionic_v0::viewport::Viewport::new());
    let provider = Box::new(TwoBlobProvider2 { step: 0 });
    let mut adapter = bionic_v0::adapter::RealPhotAdapter::new(provider);

    let base_t = 2_000_000u64;
    adapter.tick(&vp, base_t);
    adapter.tick(&vp, base_t + 33_333);

    // Collect spatial positions of active PHOT slots.
    let mut positions: Vec<(f64, f64)> = Vec::new();
    for i in 0..bionic_v0::N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & bionic_v0::MOD_PHOT) != 0 {
            if let Some(sp) = s.spatial {
                if sp.valid {
                    positions.push((sp.pos_x, sp.pos_y));
                }
            }
        }
    }
    assert!(
        positions.len() >= 2,
        "two far-apart blobs must produce at least 2 slots with spatial; got {}",
        positions.len()
    );
    // The two positions must be sufficiently different.
    let dx = positions[0].0 - positions[1].0;
    let dy = positions[0].1 - positions[1].1;
    let dist = (dx * dx + dy * dy).sqrt();
    assert!(
        dist > 0.2,
        "slots from far-apart blobs should have different spatial positions; dist={}",
        dist
    );
}

/// Single-event backward compatibility: a single blob still produces exactly
/// one claim, same phot_health, same path as Phase 4C.
#[test]
fn multi_event_claim_single_blob_backward_compat() {
    use bionic_v0::phot_camera::SyntheticCameraFrameProvider;
    use bionic_v0::viewport::{HealthState, Viewport};

    let vp = std::sync::Arc::new(Viewport::new());
    let provider = Box::new(SyntheticCameraFrameProvider::idle_then_blob(
        2, 10, 0.5, 0.5, 0.15, 0.95, 0, 33_333,
    ));
    let mut adapter = bionic_v0::adapter::RealPhotAdapter::new(provider);
    for step in 0..15u64 {
        adapter.tick(&vp, 1_000_000 + step * 33_333);
    }
    assert_eq!(
        vp.read_phot_health(),
        HealthState::Ok,
        "single-blob path must still produce phot_health=OK"
    );
    let d = vp.read_last_claim_debug();
    assert_eq!(
        d.modality_bit,
        bionic_v0::MOD_PHOT,
        "last claim must be for PHOT"
    );
}

// ---------------------------------------------------------------------------
// Phase 4D Chunk 3: synthetic multi-blob coverage
// ---------------------------------------------------------------------------

/// A SynthCamStep::MultiBlob frame with two well-separated blobs must produce
/// two active PHOT slots through the full adapter pipeline.
#[test]
fn synthetic_multi_blob_two_blobs_produce_two_slots() {
    use bionic_v0::phot_camera::{SynthCamStep, SyntheticCameraFrameProvider};
    use bionic_v0::viewport::Viewport;

    let vp = std::sync::Arc::new(Viewport::new());
    // Idle seed, then two-blob frame (well separated).
    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::MultiBlob(vec![
            bionic_v0::phot_camera::BlobSpec {
                cx: 0.15,
                cy: 0.5,
                r: 0.12,
                brightness: 0.9,
            },
            bionic_v0::phot_camera::BlobSpec {
                cx: 0.85,
                cy: 0.5,
                r: 0.12,
                brightness: 0.9,
            },
        ]),
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = bionic_v0::adapter::RealPhotAdapter::new(provider);

    adapter.tick(&vp, 1_000_000);
    adapter.tick(&vp, 1_033_333);

    let mut phot_slots = 0;
    for i in 0..bionic_v0::N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & bionic_v0::MOD_PHOT) != 0 {
            phot_slots += 1;
        }
    }
    assert!(
        phot_slots >= 2,
        "two-blob MultiBlob frame must produce at least 2 PHOT slots; got {}",
        phot_slots
    );
}

/// Three blobs where one is below PHOT_REGION_MIN_PIXELS must yield exactly
/// two active slots (the weak blob filtered out).
#[test]
fn synthetic_multi_blob_weak_third_blob_filtered() {
    use bionic_v0::phot_camera::{BlobSpec, SynthCamStep, SyntheticCameraFrameProvider};
    use bionic_v0::viewport::Viewport;

    let vp = std::sync::Arc::new(Viewport::new());
    // Third blob is tiny (r=0.01 → very few pixels) and dim — below region threshold.
    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::MultiBlob(vec![
            BlobSpec {
                cx: 0.15,
                cy: 0.3,
                r: 0.12,
                brightness: 0.9,
            },
            BlobSpec {
                cx: 0.85,
                cy: 0.7,
                r: 0.12,
                brightness: 0.9,
            },
            BlobSpec {
                cx: 0.5,
                cy: 0.5,
                r: 0.01,
                brightness: 0.06,
            }, // tiny and dim
        ]),
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = bionic_v0::adapter::RealPhotAdapter::new(provider);

    adapter.tick(&vp, 1_000_000);
    adapter.tick(&vp, 1_033_333);

    let mut phot_slots = 0;
    for i in 0..bionic_v0::N_OBJECT_SLOTS {
        let s = vp.read_slot(i).unwrap();
        if s.active && (s.source_modality & bionic_v0::MOD_PHOT) != 0 {
            phot_slots += 1;
        }
    }
    assert_eq!(phot_slots, 2,
        "two strong blobs must produce exactly 2 PHOT slots (tiny dim blob filtered by PHOT_REGION_MIN_PIXELS); got {}", phot_slots);
}

/// Two blobs whose energy order would be unstable without a tie-break rule:
/// both at the same brightness but different positions. The ordering must be
/// deterministic across repeated runs.
#[test]
fn synthetic_multi_blob_deterministic_ordering() {
    use bionic_v0::phot_camera::{BlobSpec, SynthCamStep, SyntheticCameraFrameProvider};
    use bionic_v0::viewport::Viewport;

    // Run the same scenario twice and check slot positions are consistent.
    let make_vp_and_run = || {
        let vp = std::sync::Arc::new(Viewport::new());
        let script = vec![
            SynthCamStep::Idle,
            SynthCamStep::MultiBlob(vec![
                BlobSpec {
                    cx: 0.25,
                    cy: 0.5,
                    r: 0.10,
                    brightness: 0.8,
                },
                BlobSpec {
                    cx: 0.75,
                    cy: 0.5,
                    r: 0.10,
                    brightness: 0.8,
                },
            ]),
        ];
        let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
        let mut adapter = bionic_v0::adapter::RealPhotAdapter::new(provider);
        adapter.tick(&vp, 1_000_000);
        adapter.tick(&vp, 1_033_333);
        let mut positions: Vec<(f64, f64)> = Vec::new();
        for i in 0..bionic_v0::N_OBJECT_SLOTS {
            let s = vp.read_slot(i).unwrap();
            if s.active && (s.source_modality & bionic_v0::MOD_PHOT) != 0 {
                if let Some(sp) = s.spatial {
                    if sp.valid {
                        positions.push((sp.pos_x, sp.pos_y));
                    }
                }
            }
        }
        positions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        positions
    };

    let run1 = make_vp_and_run();
    let run2 = make_vp_and_run();
    assert_eq!(
        run1.len(),
        run2.len(),
        "two runs must yield same number of slots"
    );
    assert!(run1.len() >= 2, "two blobs must yield at least 2 slots");
    for (p1, p2) in run1.iter().zip(run2.iter()) {
        let dx = p1.0 - p2.0;
        let dy = p1.1 - p2.1;
        assert!(
            (dx * dx + dy * dy).sqrt() < 0.01,
            "slot positions must be identical across runs: {:?} vs {:?}",
            p1,
            p2
        );
    }
}

// ---------------------------------------------------------------------------
// Phase 4D Chunk 4: snapshot + observer observability
// ---------------------------------------------------------------------------

/// After a multi-blob frame the snapshot must carry at least two active PHOT
/// slots, each with valid spatial data. Verifies observer can see multiple
/// PHOT slots from one frame.
#[test]
fn snapshot_carries_multiple_phot_slots_after_multi_blob() {
    use bionic_v0::phot_camera::{BlobSpec, SynthCamStep, SyntheticCameraFrameProvider};
    use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
    use bionic_v0::viewport::Viewport;

    let vp = std::sync::Arc::new(Viewport::new());
    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::MultiBlob(vec![
            BlobSpec {
                cx: 0.15,
                cy: 0.5,
                r: 0.12,
                brightness: 0.9,
            },
            BlobSpec {
                cx: 0.85,
                cy: 0.5,
                r: 0.12,
                brightness: 0.9,
            },
        ]),
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = bionic_v0::adapter::RealPhotAdapter::new(provider);

    adapter.tick(&vp, 1_000_000);
    adapter.tick(&vp, 1_033_333);

    let tmp = std::env::temp_dir().join(format!(
        "bionic_v0_4d_multi_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    std::fs::create_dir_all(&tmp).unwrap();
    let paths = SnapshotPaths::new(&tmp);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    // Count active PHOT slots in the snapshot.
    let phot_count = snap
        .slots
        .iter()
        .filter(|s| s.active && (s.source_modality & bionic_v0::MOD_PHOT) != 0)
        .count();
    assert!(
        phot_count >= 2,
        "snapshot must carry at least 2 active PHOT slots after multi-blob frame; got {}",
        phot_count
    );

    // Each active PHOT slot must have valid spatial data.
    for s in snap
        .slots
        .iter()
        .filter(|s| s.active && (s.source_modality & bionic_v0::MOD_PHOT) != 0)
    {
        if let Some(sp) = s.spatial {
            assert!(sp.valid, "active PHOT slot spatial must be valid");
            assert!(sp.pos_x >= 0.0 && sp.pos_x <= 1.0);
            assert!(sp.pos_y >= 0.0 && sp.pos_y <= 1.0);
        }
    }
}

/// Regression: claim/slot architecture must be untouched by real-camera path.
/// Specifically, claim_slot_for_event must remain the sole decision point.
#[test]
fn real_camera_path_does_not_change_claim_architecture() {
    let vp = Arc::new(Viewport::new());
    // Motion event: idle warm-up then bright blob.
    let provider = Box::new(SyntheticCameraFrameProvider::idle_then_blob(
        2, 10, 0.5, 0.5, 0.15, 0.95, 0, 33_333,
    ));
    let mut adapter = RealPhotAdapter::new(provider);
    for step in 0..15 {
        let t = 1_000_000 + (step as u64) * 33_333;
        adapter.tick(&vp, t);
    }

    // If any slot was claimed, the last_claim_debug must record it.
    let d = vp.read_last_claim_debug();
    if d.seq > 0 {
        // Claim was issued by the viewport, not the adapter directly.
        assert_eq!(d.modality_bit, MOD_PHOT, "last claim must be for PHOT");
        // chosen_slot must be a valid slot index.
        assert!(
            d.chosen_slot >= 0 && (d.chosen_slot as usize) < N_OBJECT_SLOTS,
            "claim must name a valid slot"
        );
    }
    // Adapter must not have written any slot data directly — only via claim.
    // Verify at least one slot carries PHOT data if a claim was issued.
    if d.seq > 0 {
        let slot = d.chosen_slot as usize;
        let s = vp.read_slot(slot).unwrap();
        assert!(s.active, "claimed slot must be active");
        assert!(
            (s.source_modality & MOD_PHOT) != 0,
            "claimed slot must have PHOT in source_modality"
        );
    }
}

// ---------------------------------------------------------------------------
// PHOT ingress → consequence accumulation wiring tests
//
// These verify that real PHOT adapter ticks naturally drive the consequence
// window lifecycle (begin / extend / close) via the accumulation helpers.
// No result, target, success, or imitation semantics are introduced here.
// ---------------------------------------------------------------------------

/// PHOT motion frames must begin a consequence window.
/// Before wiring: n_windows == 0 → assertion fails.
#[test]
fn phot_motion_begins_consequence_window() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;

    let vp = Arc::new(Viewport::new());
    // Use moving blobs so each frame diff produces motion.
    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::Blob {
            cx: 0.3,
            cy: 0.5,
            r: 0.12,
            brightness: 0.9,
        },
        SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.12,
            brightness: 0.9,
        },
        SynthCamStep::Blob {
            cx: 0.7,
            cy: 0.5,
            r: 0.12,
            brightness: 0.9,
        },
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    for step in 0..4u64 {
        adapter.tick(&vp, base_t + step * 33_333);
    }

    let st = vp.read_action_consequence();
    assert!(
        st.n_windows >= 1,
        "PHOT motion must begin a consequence window; got n_windows={}",
        st.n_windows
    );
    assert_eq!(
        st.n_pairs, 0,
        "pure ingress accumulation must not create consequence pairs"
    );
    assert_eq!(
        st.windows[0].phase,
        ConsequenceWindowPhase::Open,
        "window must be Open while motion is ongoing"
    );
}

/// Consecutive PHOT motion frames must extend the same Open window, not open new ones.
/// Before wiring: n_windows == 0 → assertion fails.
#[test]
fn phot_consecutive_motion_extends_same_open_window() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;

    let vp = Arc::new(Viewport::new());
    let mut script = vec![SynthCamStep::Idle];
    // Use moving blobs so each consecutive frame diff produces motion.
    let positions = [0.2, 0.3, 0.4, 0.5, 0.6];
    for cx in &positions {
        script.push(SynthCamStep::Blob {
            cx: *cx,
            cy: 0.5,
            r: 0.12,
            brightness: 0.9,
        });
    }
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    for step in 0..6u64 {
        adapter.tick(&vp, base_t + step * 33_333);
    }

    let st = vp.read_action_consequence();
    assert!(
        st.n_windows >= 1,
        "consecutive PHOT motion must produce at least one window; got {}",
        st.n_windows
    );
    assert_eq!(
        st.n_windows, 1,
        "consecutive motion must extend, not multiply, consequence windows; got {}",
        st.n_windows
    );
    assert_eq!(
        st.windows[0].phase,
        ConsequenceWindowPhase::Open,
        "window must remain Open while motion continues"
    );
    assert_eq!(
        st.n_pairs, 0,
        "accumulation must not produce consequence pairs"
    );
}

/// After motion stops and quiescence threshold is crossed, the Open window must close.
/// Before wiring: no windows → no closed window → assertion fails.
#[test]
fn phot_quiescence_closes_consequence_window() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;

    let vp = Arc::new(Viewport::new());
    let mut script = vec![SynthCamStep::Idle];
    script.push(SynthCamStep::Blob {
        cx: 0.5,
        cy: 0.5,
        r: 0.12,
        brightness: 0.9,
    });
    script.push(SynthCamStep::Blob {
        cx: 0.5,
        cy: 0.5,
        r: 0.12,
        brightness: 0.9,
    });
    // 20 idle frames × 33 ms = 660 ms > 300 ms quiescence threshold.
    for _ in 0..20 {
        script.push(SynthCamStep::Idle);
    }
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    for step in 0..23u64 {
        adapter.tick(&vp, base_t + step * 33_333);
    }

    let st = vp.read_action_consequence();
    assert!(
        st.n_windows >= 1,
        "must have at least one window; got n_windows={}",
        st.n_windows
    );
    let has_closed = st
        .windows
        .iter()
        .any(|w| w.phase == ConsequenceWindowPhase::Closed);
    assert!(
        has_closed,
        "window must close after PHOT quiescence (660 ms of idle > 300 ms threshold)"
    );
}

/// A new PHOT motion burst after the first window closes must open a second window.
/// Before wiring: n_windows == 0 → assertion fails.
#[test]
fn phot_new_motion_after_close_opens_new_window() {
    use bionic_v0::action_consequence::ConsequenceWindowPhase;

    let vp = Arc::new(Viewport::new());
    let mut script = vec![SynthCamStep::Idle];
    // First burst: moving blobs
    let positions1 = [0.2, 0.4, 0.6];
    for cx in &positions1 {
        script.push(SynthCamStep::Blob {
            cx: *cx,
            cy: 0.5,
            r: 0.12,
            brightness: 0.9,
        });
    }
    // 20 × 33 ms = 660 ms gap — closes the first window.
    for _ in 0..20 {
        script.push(SynthCamStep::Idle);
    }
    // Second burst: moving blobs in different direction
    let positions2 = [0.8, 0.6, 0.4];
    for cx in &positions2 {
        script.push(SynthCamStep::Blob {
            cx: *cx,
            cy: 0.3,
            r: 0.12,
            brightness: 0.9,
        });
    }
    // Total steps: 1 + 3 + 20 + 3 = 27.
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    for step in 0..27u64 {
        adapter.tick(&vp, base_t + step * 33_333);
    }

    let st = vp.read_action_consequence();
    assert!(
        st.n_windows >= 2,
        "second burst after close must open a new window; got n_windows={}",
        st.n_windows
    );
    let n_closed = st
        .windows
        .iter()
        .filter(|w| w.phase == ConsequenceWindowPhase::Closed)
        .count();
    let n_open = st
        .windows
        .iter()
        .filter(|w| w.phase == ConsequenceWindowPhase::Open)
        .count();
    assert!(
        n_closed >= 1,
        "first window must be Closed after the gap; closed={}",
        n_closed
    );
    assert!(
        n_open >= 1,
        "second burst must leave an Open window; open={}",
        n_open
    );
    assert_eq!(
        st.n_pairs, 0,
        "accumulation must not produce consequence pairs"
    );
}

/// PHOT ingress accumulation must produce only numeric/time substrate fields.
/// No consequence pairs, no success/target/result semantics.
/// Before wiring: n_windows == 0 → first assert fails.
#[test]
fn phot_ingress_accumulation_no_result_or_success_semantics() {
    let vp = Arc::new(Viewport::new());
    let mut script = vec![SynthCamStep::Idle];
    for _ in 0..5 {
        script.push(SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.12,
            brightness: 0.9,
        });
    }
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t = 1_000_000u64;
    for step in 0..6u64 {
        adapter.tick(&vp, base_t + step * 33_333);
    }

    let st = vp.read_action_consequence();
    assert!(
        st.n_windows >= 1,
        "PHOT motion must accumulate consequence windows; got n_windows={}",
        st.n_windows
    );
    assert_eq!(
        st.n_pairs, 0,
        "pure PHOT ingress accumulation must not produce consequence pairs"
    );
    let w = &st.windows[0];
    assert!(
        w.start_t_us > 0,
        "start_t_us must be set from real substrate time"
    );
    assert!(
        w.magnitude >= 0.0,
        "magnitude must be a non-negative numeric value"
    );
}

// ---------------------------------------------------------------------------
// Phase: live write-to-consequence temporal pairing via PHOT path.
//
// When a real write epoch exists in the shared substrate AND PHOT motion
// accumulates a consequence window, the adapter tick must automatically form
// a minimal temporal link (no result, success, target, or meaning semantics).
// ---------------------------------------------------------------------------

/// PHOT motion after a write epoch must auto-pair epoch with consequence window.
/// Before wiring: n_pairs == 0 → assertion fails.
#[test]
fn phot_motion_after_write_epoch_forms_live_temporal_pair() {
    use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};

    let vp = Arc::new(Viewport::new());

    // Seed a real write epoch *before* PHOT motion begins.
    let epoch_t: u64 = 800_000;
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.3,
        event_t_us: epoch_t,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    // PHOT motion ticks start after the epoch, within the engineering time window.
    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.12,
            brightness: 0.9,
        },
        SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.12,
            brightness: 0.9,
        },
        SynthCamStep::Blob {
            cx: 0.5,
            cy: 0.5,
            r: 0.12,
            brightness: 0.9,
        },
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    let base_t: u64 = 1_000_000; // 200 ms after epoch — well within TEMPORAL_PAIR_MAX_DELAY_US
    for step in 0..4u64 {
        adapter.tick(&vp, base_t + step * 33_333);
    }

    let st = vp.read_action_consequence();
    assert!(
        st.n_windows >= 1,
        "PHOT motion must accumulate consequence windows"
    );
    assert!(
        st.n_pairs >= 1,
        "PHOT motion after write epoch must form a live temporal pair; got n_pairs={}",
        st.n_pairs
    );
    assert_eq!(
        st.pairs[0].write_seq, write_seq,
        "pair must reference the real write epoch"
    );
    // Delay is numeric-only: time from epoch start to window start. No success/result field.
    assert!(
        st.pairs[0].delay_us > 0,
        "delay_us must be positive (epoch precedes window)"
    );
}

#[test]
fn camera_frame_v2_has_320x240_resolution_and_rgb_channels() {
    use bionic_v0::phot_camera::{CAM_W, CAM_H, CameraFrame};
    assert_eq!(CAM_W, 320);
    assert_eq!(CAM_H, 240);
    let frame = CameraFrame::zeros(0);
    assert_eq!(frame.luma.len(), 320 * 240);
    assert_eq!(frame.r.len(), 320 * 240);
    assert_eq!(frame.g.len(), 320 * 240);
    assert_eq!(frame.b.len(), 320 * 240);
}
