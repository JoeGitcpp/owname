// Probe output pressure tests.
//
// Pressures are folded into refresh_probe_law and emitted through the single
// tick_probe_output_from_law path. No target/meaning fields are introduced.

use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::viewport::Viewport;

#[test]
fn refresh_probe_law_collapses_multiple_output_pressures_to_one_write_epoch() {
    let vp = Viewport::new();
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.global_surprise_load = 1.0;
        rs.routing_focus_slot_ids[0] = 2;
    }
    {
        vp.probe_law.write().unwrap().last_top_slot_id = 0;
    }
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = true;
        gr.edges[0].pred.relation_surprise = 1.0;
    }

    vp.refresh_probe_law(1_000_000);
    vp.tick_probe_output_from_law(1_000_000);

    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 1,
        "one refresh/tick cycle must collapse simultaneous pressures into one write epoch"
    );
}

#[test]
fn refresh_probe_law_surprise_pressure_starts_one_write_epoch() {
    let vp = Viewport::new();
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.global_surprise_load = 0.8;
    }

    vp.refresh_probe_law(1_000_000);
    vp.tick_probe_output_from_law(1_000_000);

    let ao = vp.read_action_output();
    assert_eq!(ao.n_write_epochs, 1);
}

#[test]
fn refresh_probe_law_without_pressure_fires_nothing() {
    let vp = Viewport::new();

    vp.refresh_probe_law(1_000_000);
    vp.tick_probe_output_from_law(1_000_000);

    let ao = vp.read_action_output();
    assert_eq!(ao.n_write_epochs, 0);
}

#[test]
fn refresh_probe_law_slot_switch_updates_last_slot_and_adjusts_existing_epoch() {
    let vp = Viewport::new();
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.5,
        event_t_us: 1_000_000,
    });

    // Previous top slot = 0, new top slot = 1 → switch.
    {
        vp.probe_law.write().unwrap().last_top_slot_id = 0;
    }
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.routing_focus_slot_ids[0] = 3;
    }

    vp.refresh_probe_law(2_000_000);
    vp.tick_probe_output_from_law(2_000_000);

    let ao = vp.read_action_output();
    let ch = ao
        .channels
        .iter()
        .find(|c| c.channel_kind == OutputChannelKind::GenericWrite)
        .expect("channel must still be present");
    assert_eq!(ao.n_write_epochs, 1);
    assert!(
        ch.reinforce_count > 0,
        "slot switch pressure must adjust the existing write epoch"
    );
    let saved = vp.probe_law.read().unwrap().last_top_slot_id;
    assert_eq!(saved, 3, "refresh must store the current top slot");
}

#[test]
fn refresh_probe_law_same_slot_does_not_create_epoch() {
    let vp = Viewport::new();
    let same_slot: i8 = 2;
    {
        vp.probe_law.write().unwrap().last_top_slot_id = same_slot;
    }
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.routing_focus_slot_ids[0] = same_slot;
    }

    vp.refresh_probe_law(1_000_000);
    vp.tick_probe_output_from_law(1_000_000);

    let ao = vp.read_action_output();
    assert_eq!(ao.n_write_epochs, 0);
}

#[test]
fn refresh_probe_law_relation_pressure_starts_one_write_epoch_from_active_edge() {
    let vp = Viewport::new();
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = true;
        gr.edges[0].pred.relation_surprise = 0.7;
    }

    vp.refresh_probe_law(1_000_000);
    vp.tick_probe_output_from_law(1_000_000);

    let ao = vp.read_action_output();
    assert_eq!(ao.n_write_epochs, 1);
}

#[test]
fn refresh_probe_law_inactive_relation_edge_is_ignored() {
    let vp = Viewport::new();
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = false;
        gr.edges[0].pred.relation_surprise = 1.0;
    }

    vp.refresh_probe_law(1_000_000);
    vp.tick_probe_output_from_law(1_000_000);

    let ao = vp.read_action_output();
    assert_eq!(ao.n_write_epochs, 0);
}

// ─── Hygiene: no semantic/target/meaning fields introduced ────────────────────

#[test]
fn probe_tick_module_no_semantic_fields() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/probe_law.rs",))
        .expect("read probe_law.rs");
    let forbidden = &[
        "target_object",
        "result_label",
        "concept_id",
        "token",
        "semantic_gate",
        "curiosity_threshold",
        "meaning",
    ];
    for kw in forbidden {
        assert!(!src.contains(kw), "probe_law.rs must not contain: {}", kw);
    }
}
