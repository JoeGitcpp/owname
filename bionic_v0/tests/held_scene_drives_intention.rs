//! B2.3 step 3: the held scene is INPUT-DECOUPLED — when the body holds a scene
//! and there is no live perception candidate, the scene's lead drives the next
//! intention commit. And once the scene ages out (TTL), it stops biasing.

use bionic_v0::perturbation_graph::{Endpoint, PG_NODE_SIG_DIM};
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
