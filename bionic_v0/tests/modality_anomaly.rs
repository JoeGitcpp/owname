use std::sync::Arc;
use bionic_v0::viewport::Viewport;
use bionic_v0::AXIS_COUNT;

fn push_err_and_update(vp: &Arc<Viewport>, axis: usize, err: f64) {
    vp.update_predictor(axis, 0.0, 0.0, err);
}

fn warmup_axis(vp: &Arc<Viewport>, axis: usize, n: u64) {
    for i in 0..n {
        vp.push_observation(axis, i * 1000, 0.5);
        push_err_and_update(vp, axis, 0.01);
    }
}

fn calibrate_sigma(vp: &Arc<Viewport>, n_ticks: usize) {
    for t in 0..n_ticks {
        vp.tick_modality_anomaly(t as u64 * 33_000);
    }
}

#[test]
fn t1_zero_error_gives_zero_anomaly() {
    let vp = Arc::new(Viewport::new());
    for i in 0..AXIS_COUNT {
        warmup_axis(&vp, i, 40);
        push_err_and_update(&vp, i, 0.0);
    }
    for _ in 0..20 {
        vp.tick_modality_anomaly(100_000);
    }
    let frac = vp.read_modality_anomaly();
    for f in &frac {
        assert!(*f < 0.01, "zero-error should give near-zero anomaly, got {}", f);
    }
}

#[test]
fn t2_vib_anomaly_isolated() {
    let vp = Arc::new(Viewport::new());
    for i in 0..AXIS_COUNT {
        warmup_axis(&vp, i, 40);
    }
    // Let sigma calibrate to baseline (err=0.01) before injecting spike
    calibrate_sigma(&vp, 100);

    // Push large errors on all VIB bands
    for i in 0..bionic_v0::VIB_BANDS {
        for _ in 0..10 {
            push_err_and_update(&vp, i, 10.0);
        }
    }
    for t in 0..30 {
        vp.tick_modality_anomaly(500_000 + t as u64 * 33_000);
    }
    let frac = vp.read_modality_anomaly();
    assert!(frac[0] > 0.5, "VIB should be anomalous, got {}", frac[0]);
    for m in 1..frac.len() {
        assert!(frac[m] < 0.1, "modality {} should be unaffected, got {}", m, frac[m]);
    }
}

#[test]
fn t3_warmup_guard() {
    let vp = Arc::new(Viewport::new());
    for i in 0..bionic_v0::VIB_BANDS {
        for j in 0..5 {
            vp.push_observation(i, j * 1000, 0.5);
            push_err_and_update(&vp, i, 100.0);
        }
    }
    vp.tick_modality_anomaly(50_000);
    let frac = vp.read_modality_anomaly();
    assert!(frac[0] < 0.01, "axes below warmup should not count as anomalous, got {}", frac[0]);
}

#[test]
fn t4_ema_decay() {
    let vp = Arc::new(Viewport::new());
    for i in 0..bionic_v0::VIB_BANDS {
        warmup_axis(&vp, i, 40);
    }
    // Spike errors
    for i in 0..bionic_v0::VIB_BANDS {
        push_err_and_update(&vp, i, 50.0);
    }
    vp.tick_modality_anomaly(100_000);
    let peak = vp.read_modality_anomaly()[0];
    assert!(peak > 0.05, "should detect spike");

    // Now push zero errors and tick many times
    for i in 0..bionic_v0::VIB_BANDS {
        for _ in 0..20 {
            push_err_and_update(&vp, i, 0.001);
        }
    }
    for t in 0..30 {
        vp.tick_modality_anomaly(200_000 + t as u64 * 33_000);
    }
    let decayed = vp.read_modality_anomaly()[0];
    assert!(decayed < 0.1, "EMA should decay after spike, got {}", decayed);
}

#[test]
fn t5_maturity_gate_blocks() {
    let vp = Arc::new(Viewport::new());
    for i in 0..AXIS_COUNT {
        warmup_axis(&vp, i, 40);
    }
    // Create anomaly: large errors on VIB
    for i in 0..bionic_v0::VIB_BANDS {
        for _ in 0..20 {
            push_err_and_update(&vp, i, 50.0);
        }
    }
    for _ in 0..10 {
        vp.tick_modality_anomaly(100_000);
    }
    assert!(vp.modality_anomaly_blocks_maturity(), "should block maturity during anomaly");

    let m_before = vp.developmental_maturity();
    vp.advance_maturity(1.0);
    let m_after = vp.developmental_maturity();
    assert!(
        (m_after - m_before).abs() < 1e-12,
        "maturity should not advance during anomaly: before={}, after={}",
        m_before, m_after,
    );
}

#[test]
fn t6_maturity_resumes_after_anomaly() {
    let vp = Arc::new(Viewport::new());
    for i in 0..AXIS_COUNT {
        warmup_axis(&vp, i, 40);
    }
    // Let sigma calibrate to baseline first
    calibrate_sigma(&vp, 100);

    // Set all last_err to near-zero so nothing looks anomalous
    for i in 0..AXIS_COUNT {
        push_err_and_update(&vp, i, 0.0);
    }
    for _ in 0..50 {
        vp.tick_modality_anomaly(500_000);
    }
    assert!(!vp.modality_anomaly_blocks_maturity(), "should not block after decay");

    let m_before = vp.developmental_maturity();
    vp.advance_maturity(1.0);
    let m_after = vp.developmental_maturity();
    assert!(
        m_after > m_before,
        "maturity should advance after anomaly clears: before={}, after={}",
        m_before, m_after,
    );
}

#[test]
fn t7_rollout_trigger() {
    let vp = Arc::new(Viewport::new());
    for i in 0..bionic_v0::VIB_BANDS {
        warmup_axis(&vp, i, 40);
    }
    // Moderate anomaly
    for i in 0..bionic_v0::VIB_BANDS {
        for _ in 0..15 {
            push_err_and_update(&vp, i, 20.0);
        }
    }
    for _ in 0..10 {
        vp.tick_modality_anomaly(100_000);
    }
    assert!(
        vp.modality_anomaly_triggers_rollout(),
        "should trigger rollout during anomaly"
    );
}

#[test]
fn t9_modality_isolation() {
    let vp = Arc::new(Viewport::new());
    for i in 0..AXIS_COUNT {
        warmup_axis(&vp, i, 40);
    }
    // Let sigma calibrate to baseline before injecting spike
    calibrate_sigma(&vp, 100);

    // Push anomaly only on VIS
    for i in bionic_v0::AXIS_VIS_BASE..(bionic_v0::AXIS_VIS_BASE + bionic_v0::VIS_AXES) {
        for _ in 0..10 {
            push_err_and_update(&vp, i, 50.0);
        }
    }
    for _ in 0..20 {
        vp.tick_modality_anomaly(500_000);
    }
    let frac = vp.read_modality_anomaly();
    assert!(frac[1] > 0.3, "VIS modality should be anomalous, got {}", frac[1]);
    assert!(frac[0] < 0.1, "VIB should be unaffected, got {}", frac[0]);
    assert!(frac[2] < 0.1, "INTERO should be unaffected, got {}", frac[2]);
}
