use bionic_v0::adaptive_stability::{
    AdaptiveRouteSample, AdaptiveStabilityState, ROUTE_PHASE_COMPRESSED, ROUTE_PHASE_DEGRADED,
    ROUTE_PHASE_RECOVERING,
};
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use std::fs;
use std::sync::Arc;

fn sample(route_id: u64, t_us: u64, err: f64, cost: f64, closure: f64) -> AdaptiveRouteSample {
    AdaptiveRouteSample {
        route_id,
        route_code: 0,
        t_us,
        err,
        resource_cost: cost,
        closure_gain: closure,
    }
}

fn temp_snapshot_paths(test_name: &str) -> SnapshotPaths {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_adaptive_stability_{}_{}",
        test_name,
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    SnapshotPaths::new(dir)
}

#[test]
fn local_degradation_raises_bypass_and_relearning_without_touching_neighbor_route() {
    let mut st = AdaptiveStabilityState::default();

    for i in 0..8 {
        st.observe_sample(sample(7, 1_000 + i * 100, 0.92, 0.81, 0.03));
        st.observe_sample(sample(11, 1_000 + i * 100, 0.04, 0.20, 0.72));
    }

    let degraded = st.route(7).expect("degraded route trace");
    assert_eq!(degraded.phase_code, ROUTE_PHASE_DEGRADED);
    assert!(degraded.degradation_score > 0.50);
    assert!(degraded.bypass_weight > 0.30);
    assert!(degraded.relearn_pressure > 0.30);

    let neighbor = st.route(11).expect("neighbor route trace");
    assert!(neighbor.degradation_score < 0.20);
    assert!(neighbor.bypass_weight < 0.10);
    assert!(neighbor.relearn_pressure < 0.10);
}

#[test]
fn repeated_stable_samples_form_reversible_fast_path_strength() {
    let mut st = AdaptiveStabilityState::default();

    for i in 0..8 {
        st.observe_sample(sample(3, 2_000 + i * 100, 0.03, 0.18, 0.82));
    }

    let route = st.route(3).expect("stable route trace");
    assert_eq!(route.phase_code, ROUTE_PHASE_COMPRESSED);
    assert!(route.fast_path_strength > 0.35);
    assert!(route.slow_path_weight > 0.0);
    assert!(route.last_good_t_us > 0);
}

#[test]
fn drift_revokes_fast_path_strength_and_keeps_slow_path_available() {
    let mut st = AdaptiveStabilityState::default();

    for i in 0..8 {
        st.observe_sample(sample(5, 3_000 + i * 100, 0.03, 0.18, 0.82));
    }
    let fast_before = st.route(5).unwrap().fast_path_strength;
    assert!(fast_before > 0.35);

    for i in 0..8 {
        st.observe_sample(sample(5, 5_000 + i * 100, 0.88, 0.79, 0.04));
    }

    let route = st.route(5).expect("drifted route trace");
    assert!(route.fast_path_strength < fast_before);
    assert!(route.slow_path_weight > 0.0);
    assert!(
        route.phase_code == ROUTE_PHASE_DEGRADED || route.phase_code == ROUTE_PHASE_RECOVERING
    );
}

#[test]
fn maintenance_decay_keeps_recent_open_route_trace_observable() {
    let mut st = AdaptiveStabilityState::default();

    st.observe_sample(sample(13, 7_000, 0.40, 0.20, 0.30));
    st.tick_decay(7_100);

    assert!(
        st.route(13).is_some(),
        "recent open route trace must remain observable across maintenance tick"
    );
}

#[test]
fn viewport_exposes_adaptive_stability_as_numeric_route_traces() {
    let vp = Arc::new(Viewport::new());

    let empty = vp.read_adaptive_stability();
    assert_eq!(empty.n_routes, 0);

    vp.observe_adaptive_route_sample(sample(21, 9_000, 0.90, 0.84, 0.02));
    let st = vp.read_adaptive_stability();

    assert_eq!(st.n_routes, 1);
    let route = st.route(21).expect("viewport route trace");
    assert_eq!(route.route_id, 21);
    assert_eq!(route.route_code, 0);
    assert!(route.err_ema > 0.0);
    assert_eq!(route.last_update_t_us, 9_000);
}

#[test]
fn snapshot_roundtrip_preserves_adaptive_stability_route_traces() {
    let vp = Arc::new(Viewport::new());
    for i in 0..8 {
        vp.observe_adaptive_route_sample(sample(31, 10_000 + i * 100, 0.03, 0.18, 0.82));
    }

    let paths = temp_snapshot_paths("roundtrip");
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    assert_eq!(snap.adaptive_stability.n_routes, 1);
    let route = snap
        .adaptive_stability
        .route(31)
        .expect("snapshot adaptive route");
    assert_eq!(route.phase_code, ROUTE_PHASE_COMPRESSED);
    assert!(route.fast_path_strength > 0.35);
    assert!(route.slow_path_weight > 0.0);
}

#[test]
fn body_runtime_wires_adaptive_stability_maintenance_ticker() {
    let body = include_str!("../src/bin/body.rs");
    assert!(
        body.contains("tick_adaptive_stability"),
        "body runtime must tick adaptive stability maintenance"
    );
}

#[test]
fn adaptive_stability_source_avoids_rejected_architecture_terms() {
    let src = include_str!("../src/adaptive_stability.rs");
    for banned in [
        "semantic_readiness",
        "SemanticReadinessState",
        "refresh_semantic_readiness",
        "symbolizer",
        "SymbolizerState",
        "refresh_symbolizer",
        "concept_id",
        "ObjectConceptBinding",
        "RelationConceptBinding",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(banned),
            "adaptive stability substrate must not contain rejected term: {}",
            banned
        );
    }
}

#[test]
fn adaptive_stability_route_traces_use_numeric_codes_not_named_routes_or_phases() {
    let src = include_str!("../src/adaptive_stability.rs");
    for forbidden in [
        "AdaptiveRouteKind",
        "AdaptiveRoutePhase",
        "VocalPrototype",
        "Bypassing",
        "Relearning",
        "FastPathActive",
    ] {
        assert!(
            !src.contains(forbidden),
            "adaptive stability must not expose named route or repair phase `{}`",
            forbidden
        );
    }
}
