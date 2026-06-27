//! B2.3 step 3: the held scene is INPUT-DECOUPLED — when the body holds a scene
//! and there is no live perception candidate, the scene's lead drives the next
//! intention commit. And once the scene ages out (TTL), it stops biasing.

use bionic_v0::perturbation_graph::{EdgeKind, Endpoint, PG_NODE_SIG_DIM};
use bionic_v0::viewport::Viewport;
use std::sync::atomic::Ordering;

/// Seed 8 relation-less nodes; node A has the highest bind, so it is BOTH the
/// most-bound anchor (whose `strongest_out_relation` is None → NO perception
/// candidate) AND the rest-walk seed → the held scene is exactly `{A}`.
/// Returns A's id.
fn seed_isolated_held_scene(vp: &Viewport) -> u32 {
    let mut g = vp.perturbation_graph.write().unwrap();
    let mut a = 0u32;
    for i in 0..8 {
        let id = g.add_node(0, [0.05 * i as f64; PG_NODE_SIG_DIM]);
        let n = g.node_mut(id).unwrap();
        n.bind_strength = if i == 0 { 0.9 } else { 0.1 };
        n.persistence_ema = 0.0;
        if i == 0 {
            a = id;
        }
    }
    // Capture {A} as the held scene via the public consolidate path.
    g.consolidate_quiet_neighborhood(0, bionic_v0::QUIET_REPLAY_SPREAD_DEPTH);
    assert_eq!(g.held_scene().lead, Some(a), "A is the rest seed");
    assert_eq!(g.held_scene().nodes, vec![a], "isolated cue → scene is just the seed");
    a
}

#[test]
fn held_scene_drives_intention_when_perception_is_absent() {
    let vp = Viewport::new();
    let a = seed_isolated_held_scene(&vp);
    // No relations anywhere → the perception candidate is always None; only the
    // held scene can commit. Drive past the FastPath-silent gate, within the TTL.
    for t in 1..=3u64 {
        vp.tick_intention_arbitrator(t * 1_000);
    }
    assert_eq!(
        vp.intention.read().unwrap().current_target,
        Some(Endpoint::Node(a)),
        "with no perception candidate, the held scene's lead drives intention (input-decoupled)"
    );
}

#[test]
fn aged_out_held_scene_does_not_drive_intention() {
    let vp = Viewport::new();
    let _a = seed_isolated_held_scene(&vp); // captured at now = 0
    vp.fast_path_silent_ticks.store(5, Ordering::Relaxed); // already past the silent gate
    let now = bionic_v0::HELD_SCENE_TTL_US + 10_000_000; // scene long since released
    vp.tick_intention_arbitrator(now);
    assert_eq!(
        vp.intention.read().unwrap().current_target,
        None,
        "a released (TTL-aged) scene must not bias intention"
    );
}

/// Seed a graph where the most-bound node Z has NO relations (so there is never
/// a perception candidate), but the rest-walk seed Y (2nd-highest bind) DOES —
/// so the captured scene is multi-node {Y, X1, X2}. Returns (Y, X1).
fn seed_multinode_scene(vp: &Viewport) -> (u32, u32) {
    let mut g = vp.perturbation_graph.write().unwrap();
    let sig = |v: f64| [v; PG_NODE_SIG_DIM];
    let z = g.add_node(0, sig(0.01));
    let y = g.add_node(0, sig(0.20));
    let x1 = g.add_node(0, sig(0.40));
    let x2 = g.add_node(0, sig(0.60));
    let mut fill = Vec::new();
    for i in 0..4 {
        fill.push(g.add_node(0, sig(0.80 + 0.01 * i as f64)));
    }
    g.node_mut(z).unwrap().bind_strength = 0.95; // most-bound, no relations
    g.node_mut(y).unwrap().bind_strength = 0.90; // rest seed (top[1])
    g.node_mut(x1).unwrap().bind_strength = 0.50;
    g.node_mut(x2).unwrap().bind_strength = 0.30;
    for &f in &fill {
        g.node_mut(f).unwrap().bind_strength = 0.05;
    }
    for n in [z, y, x1, x2] {
        g.node_mut(n).unwrap().persistence_ema = 0.0;
    }
    // Y's learned relations → the evoked neighbourhood the scene captures.
    g.observe_relation(0, Endpoint::Node(y), Endpoint::Node(x1), EdgeKind::Cooccur);
    g.observe_relation(0, Endpoint::Node(y), Endpoint::Node(x2), EdgeKind::Cooccur);
    // Capture at t=100_000 so the rotation cursor (idx=1) selects Y, not Z.
    g.consolidate_quiet_neighborhood(100_000, bionic_v0::QUIET_REPLAY_SPREAD_DEPTH);
    assert_eq!(
        g.held_scene().lead,
        Some(y),
        "rest seed is Y (2nd-highest), not the most-bound Z"
    );
    (y, x1)
}

#[test]
fn held_scene_plays_through_multiple_nodes_over_time() {
    let vp = Viewport::new();
    let (y, x1) = seed_multinode_scene(&vp);
    vp.fast_path_silent_ticks.store(3, Ordering::Relaxed); // past the silent gate
    // Cursor 0 (most salient = Y) fills the open slot first.
    vp.tick_intention_arbitrator(150_000);
    assert_eq!(
        vp.intention.read().unwrap().current_target,
        Some(Endpoint::Node(y)),
        "playthrough starts on the most-salient held node"
    );
    // After the commitment window, cursor 1 (X1) takes the now-open slot — the
    // body has advanced to a DIFFERENT held node, not just the lead. This is the
    // multi-node recombination consuming the scene set, not only its lead.
    vp.tick_intention_arbitrator(2_400_000);
    assert_eq!(
        vp.intention.read().unwrap().current_target,
        Some(Endpoint::Node(x1)),
        "playthrough advances to the next held node once the window opens"
    );
}
