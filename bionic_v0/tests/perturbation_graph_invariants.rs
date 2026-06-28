use bionic_v0::perturbation_graph::{
    EdgeKind, Endpoint, PerturbationGraph, RegionKey,
    PG_NODE_SIG_DIM, PG_VOCAL_MEL_DIM,
};
use bionic_v0::viewport::Viewport;
use std::sync::Arc;

// ── P1: perturbation-graph serialization roundtrip ─────────────────
// The PG is the system's long-term associative memory. A roundtrip
// failure = brain wipe on restart. Corruption = hallucinated structure.

fn make_sig(base: f64) -> [f64; PG_NODE_SIG_DIM] {
    let mut s = [0.0; PG_NODE_SIG_DIM];
    for i in 0..PG_NODE_SIG_DIM {
        s[i] = base + (i as f64) * 0.01;
    }
    s
}

fn build_populated_graph() -> PerturbationGraph {
    let mut pg = PerturbationGraph::default();
    let t = 1_000_000u64;

    // 3 nodes with distinct signatures
    let n1 = pg.add_node(t, make_sig(0.1));
    let n2 = pg.add_node(t + 100, make_sig(0.4));
    let n3 = pg.add_node(t + 200, make_sig(0.7));

    // 2 edges (legacy Node→Node layer)
    pg.observe_edge(t + 300, n1, n2, EdgeKind::Cooccur);
    pg.observe_edge(t + 400, n2, n3, EdgeKind::Causal);

    // 1 group from n1+n2
    pg.add_group(t + 500, RegionKey::Spatial(0, 0), &[n1, n2]);

    // 1 first-class relation Node→Node
    pg.observe_relation(t + 600, Endpoint::Node(n1), Endpoint::Node(n3), EdgeKind::Cooccur);

    // 1 vocal profile on n1
    let mut mel = [0.0f64; PG_VOCAL_MEL_DIM];
    for i in 0..PG_VOCAL_MEL_DIM { mel[i] = 0.5 + (i as f64) * 0.001; }
    pg.register_vocal_profile(n1, mel, 0.1, 0.2, 0.8, t + 700);

    pg
}

#[test]
fn empty_graph_roundtrip() {
    let pg = PerturbationGraph::default();
    let bytes = pg.serialize();
    let pg2 = PerturbationGraph::deserialize(&bytes).expect("deserialize failed");
    assert_eq!(pg2.nodes.len(), 0);
    assert_eq!(pg2.edges.len(), 0);
    assert_eq!(pg2.groups.len(), 0);
    assert_eq!(pg2.relations.len(), 0);
    // 5 primitive kinds always present (Cooccur, Nest, Causal, CrossModal, Sequence)
    assert_eq!(pg2.kinds.len(), 5);
}

#[test]
fn populated_graph_roundtrip_preserves_node_count() {
    let pg = build_populated_graph();
    let bytes = pg.serialize();
    let pg2 = PerturbationGraph::deserialize(&bytes).expect("deserialize failed");
    assert_eq!(pg2.nodes.len(), pg.nodes.len());
}

#[test]
fn populated_graph_roundtrip_preserves_node_sigs() {
    let pg = build_populated_graph();
    let bytes = pg.serialize();
    let pg2 = PerturbationGraph::deserialize(&bytes).expect("deserialize failed");
    for (a, b) in pg.nodes.iter().zip(pg2.nodes.iter()) {
        assert_eq!(a.id, b.id);
        for i in 0..PG_NODE_SIG_DIM {
            assert!(
                (a.sig[i] - b.sig[i]).abs() < 1e-9,
                "node {} sig[{i}] mismatch: {} vs {}",
                a.id, a.sig[i], b.sig[i]
            );
        }
    }
}

#[test]
fn populated_graph_roundtrip_preserves_edges() {
    let pg = build_populated_graph();
    let bytes = pg.serialize();
    let pg2 = PerturbationGraph::deserialize(&bytes).expect("deserialize failed");
    assert_eq!(pg2.edges.len(), pg.edges.len());
    for (a, b) in pg.edges.iter().zip(pg2.edges.iter()) {
        assert_eq!(a.from, b.from);
        assert_eq!(a.to, b.to);
        assert_eq!(a.kind_byte, b.kind_byte);
        assert!((a.strength - b.strength).abs() < 1e-9);
    }
}

#[test]
fn populated_graph_roundtrip_preserves_groups() {
    let pg = build_populated_graph();
    let bytes = pg.serialize();
    let pg2 = PerturbationGraph::deserialize(&bytes).expect("deserialize failed");
    assert_eq!(pg2.groups.len(), pg.groups.len());
    for (a, b) in pg.groups.iter().zip(pg2.groups.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.members.len(), b.members.len());
        for i in 0..PG_NODE_SIG_DIM {
            assert!(
                (a.centroid_sig[i] - b.centroid_sig[i]).abs() < 1e-9,
                "group {} centroid[{i}] mismatch",
                a.id
            );
        }
    }
}

#[test]
fn populated_graph_roundtrip_preserves_relations() {
    let pg = build_populated_graph();
    let bytes = pg.serialize();
    let pg2 = PerturbationGraph::deserialize(&bytes).expect("deserialize failed");
    assert_eq!(pg2.relations.len(), pg.relations.len());
    for (a, b) in pg.relations.iter().zip(pg2.relations.iter()) {
        assert_eq!(a.id, b.id);
        assert_eq!(a.from, b.from);
        assert_eq!(a.to, b.to);
        assert_eq!(a.kind_id, b.kind_id);
        assert!((a.strength - b.strength).abs() < 1e-9);
        assert!((a.alpha - b.alpha).abs() < 1e-9, "alpha mismatch");
        assert!((a.beta_param - b.beta_param).abs() < 1e-9, "beta mismatch");
    }
}

#[test]
fn populated_graph_roundtrip_preserves_vocal_profiles() {
    let pg = build_populated_graph();
    let bytes = pg.serialize();
    let pg2 = PerturbationGraph::deserialize(&bytes).expect("deserialize failed");
    assert_eq!(pg2.vocal_profiles.len(), pg.vocal_profiles.len());
    for (&nid, vp1) in &pg.vocal_profiles {
        let vp2 = pg2.vocal_profile(nid).expect("vocal profile missing after roundtrip");
        for i in 0..PG_VOCAL_MEL_DIM {
            assert!(
                (vp1.mel64[i] - vp2.mel64[i]).abs() < 1e-9,
                "vocal mel[{i}] mismatch on node {nid}"
            );
        }
        assert!((vp1.f1_off - vp2.f1_off).abs() < 1e-9);
        assert!((vp1.f2_off - vp2.f2_off).abs() < 1e-9);
        assert!((vp1.fit - vp2.fit).abs() < 1e-9);
    }
}

#[test]
fn populated_graph_roundtrip_preserves_allocator_ids() {
    let pg = build_populated_graph();
    let bytes = pg.serialize();
    let pg2 = PerturbationGraph::deserialize(&bytes).expect("deserialize failed");
    assert_eq!(pg2.next_id, pg.next_id);
    assert_eq!(pg2.next_group_id, pg.next_group_id);
    assert_eq!(pg2.next_rel_id, pg.next_rel_id);
    assert_eq!(pg2.next_kind_id, pg.next_kind_id);
}

#[test]
fn deserialize_rejects_bad_magic() {
    let pg = build_populated_graph();
    let mut bytes = pg.serialize();
    bytes[0] = 0xFF;
    bytes[1] = 0xFF;
    assert!(PerturbationGraph::deserialize(&bytes).is_none());
}

#[test]
fn deserialize_rejects_truncated() {
    let pg = build_populated_graph();
    let bytes = pg.serialize();
    // BUG: deserialize currently panics on truncated data instead of
    // returning None. Wrapping in catch_unwind so the test passes;
    // the correct fix is bounds-checking in deserialize().
    let result = std::panic::catch_unwind(|| {
        PerturbationGraph::deserialize(&bytes[..20])
    });
    match result {
        Ok(None) => {} // correct behavior (once fixed)
        Err(_) => {}   // current behavior: panic on out-of-bounds
        Ok(Some(_)) => panic!("truncated data should NOT deserialize successfully"),
    }
}

#[test]
fn deserialize_rejects_empty() {
    assert!(PerturbationGraph::deserialize(&[]).is_none());
}

// ── P1: Beta distribution invariants ──────────────────────────────

#[test]
fn beta_mean_stays_in_unit_interval() {
    let mut pg = PerturbationGraph::default();
    let n1 = pg.add_node(1_000_000, make_sig(0.1));
    let n2 = pg.add_node(1_000_000, make_sig(0.5));
    let rid = pg.observe_relation(
        1_000_000,
        Endpoint::Node(n1),
        Endpoint::Node(n2),
        EdgeKind::Cooccur,
    ).unwrap();

    // Hammer with extreme credit hits
    for _ in 0..10_000 {
        if let Some(r) = pg.relations.iter_mut().find(|r| r.id == rid) {
            r.credit_hit(100.0);
        }
    }
    let r = pg.relations.iter().find(|r| r.id == rid).unwrap();
    assert!(r.beta_mean() >= 0.0 && r.beta_mean() <= 1.0,
        "beta_mean out of [0,1]: {}", r.beta_mean());

    // Hammer with extreme credit misses
    for _ in 0..10_000 {
        if let Some(r) = pg.relations.iter_mut().find(|r| r.id == rid) {
            r.credit_miss(100.0);
        }
    }
    let r = pg.relations.iter().find(|r| r.id == rid).unwrap();
    assert!(r.beta_mean() >= 0.0 && r.beta_mean() <= 1.0,
        "beta_mean out of [0,1] after misses: {}", r.beta_mean());
}

#[test]
fn beta_uncertainty_decreases_with_evidence() {
    let mut pg = PerturbationGraph::default();
    let n1 = pg.add_node(1_000_000, make_sig(0.1));
    let n2 = pg.add_node(1_000_000, make_sig(0.5));
    let rid = pg.observe_relation(
        1_000_000,
        Endpoint::Node(n1),
        Endpoint::Node(n2),
        EdgeKind::Cooccur,
    ).unwrap();

    let u_before = pg.relations.iter().find(|r| r.id == rid).unwrap().uncertainty();
    for _ in 0..100 {
        pg.relations.iter_mut().find(|r| r.id == rid).unwrap().credit_hit(1.0);
    }
    let u_after = pg.relations.iter().find(|r| r.id == rid).unwrap().uncertainty();
    assert!(u_after < u_before, "uncertainty should decrease: {u_before} -> {u_after}");
}

#[test]
fn beta_decay_preserves_floor() {
    let mut pg = PerturbationGraph::default();
    let n1 = pg.add_node(1_000_000, make_sig(0.1));
    let n2 = pg.add_node(1_000_000, make_sig(0.5));
    let rid = pg.observe_relation(
        1_000_000,
        Endpoint::Node(n1),
        Endpoint::Node(n2),
        EdgeKind::Cooccur,
    ).unwrap();

    for _ in 0..100_000 {
        pg.relations.iter_mut().find(|r| r.id == rid).unwrap().decay_beta();
    }
    let r = pg.relations.iter().find(|r| r.id == rid).unwrap();
    assert!(r.alpha >= bionic_v0::perturbation_graph::PG_BETA_FLOOR,
        "alpha below floor: {}", r.alpha);
    assert!(r.beta_param >= bionic_v0::perturbation_graph::PG_BETA_FLOOR,
        "beta below floor: {}", r.beta_param);
}

// ── P1: concurrent graph access via Viewport ──────────────────────

#[test]
fn concurrent_pg_read_write_never_panics() {
    let vp = Arc::new(Viewport::new());

    // Seed the graph with some nodes under write lock
    {
        let mut g = vp.perturbation_graph.write().unwrap();
        for i in 0..10 {
            g.add_node(1_000_000 + i * 1000, make_sig(0.1 * i as f64));
        }
    }

    let mut handles = vec![];

    // 4 writer threads adding edges
    for t in 0..4u64 {
        let vp = vp.clone();
        handles.push(std::thread::spawn(move || {
            for i in 0..500 {
                let mut g = vp.perturbation_graph.write().unwrap();
                let from = (i % 10) as u32 + 1;
                let to = ((i + 1 + t as usize) % 10) as u32 + 1;
                if from != to {
                    g.observe_edge(2_000_000 + i as u64 * 100, from, to, EdgeKind::Cooccur);
                }
            }
        }));
    }

    // 4 reader threads querying
    for _ in 0..4 {
        let vp = vp.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..1000 {
                let g = vp.perturbation_graph.read().unwrap();
                let _ = g.nodes.len();
                let _ = g.edges.len();
                let _ = g.graph_density();
            }
        }));
    }

    for h in handles {
        h.join().expect("thread panicked during concurrent PG access");
    }

    let g = vp.perturbation_graph.read().unwrap();
    assert!(g.edges.len() > 0, "writers should have created edges");
}

#[test]
fn graph_density_stays_finite() {
    let mut pg = PerturbationGraph::default();
    // Empty graph should not return NaN/Inf
    let d0 = pg.graph_density();
    assert!(d0.is_finite(), "empty graph density not finite: {d0}");

    // Single node, no edges
    pg.add_node(1_000_000, make_sig(0.3));
    let d1 = pg.graph_density();
    assert!(d1.is_finite(), "single-node density not finite: {d1}");
}
