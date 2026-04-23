// TDD tests for the three additional action-output trigger functions.
//
// Tasks 5-7 from docs/superpowers/plans/2026-04-21-body-side-write-trigger.md
//
// All thresholds are engineering approximations; no target/meaning/semantic
// fields are introduced anywhere in this file.
//
// Run order:
//   1. Tests written first → all fail (RED)
//   2. Implementation lands → all pass (GREEN)

use bionic_v0::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use bionic_v0::viewport::Viewport;
use bionic_v0::{RELATION_INSTABILITY_TRIGGER_THRESHOLD, SURPRISE_TRIGGER_THRESHOLD};

// ─── Task 5: Surprise-driven trigger ──────────────────────────────────────────

#[test]
fn surprise_trigger_above_threshold_fires_begin() {
    let vp = Viewport::new();
    // Inject high global_surprise_load directly into GlobalReasoningState.
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.global_surprise_load = SURPRISE_TRIGGER_THRESHOLD + 0.1;
    }
    vp.tick_surprise_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 1,
        "surprise_load above threshold must fire Begin (new write epoch)"
    );
}

#[test]
fn surprise_trigger_at_threshold_fires_nothing() {
    let vp = Viewport::new();
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.global_surprise_load = SURPRISE_TRIGGER_THRESHOLD; // equal — must NOT fire
    }
    vp.tick_surprise_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 0,
        "surprise_load exactly at threshold must not fire (strictly greater required)"
    );
}

#[test]
fn surprise_trigger_below_threshold_fires_nothing() {
    let vp = Viewport::new();
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.global_surprise_load = SURPRISE_TRIGGER_THRESHOLD * 0.5;
    }
    vp.tick_surprise_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 0,
        "low surprise_load must not fire any event"
    );
}

#[test]
fn surprise_trigger_zero_load_fires_nothing() {
    let vp = Viewport::new();
    // GlobalReasoningState defaults to 0.0 global_surprise_load — no write needed.
    vp.tick_surprise_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(ao.n_write_epochs, 0, "zero surprise_load must not fire");
}

#[test]
fn surprise_trigger_magnitude_equals_surprise_load() {
    let vp = Viewport::new();
    let expected_mag = (SURPRISE_TRIGGER_THRESHOLD + 0.2).min(1.0);
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.global_surprise_load = expected_mag;
    }
    vp.tick_surprise_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(ao.n_write_epochs, 1);
    let ch = ao
        .channels
        .iter()
        .find(|c| c.channel_kind == OutputChannelKind::GenericWrite)
        .expect("channel must exist after Begin");
    assert!(
        (ch.current_magnitude - expected_mag).abs() < 1e-9,
        "Begin magnitude must equal clamped surprise_load"
    );
}

// ─── Task 6: Attention-rebalance trigger ──────────────────────────────────────

#[test]
fn attention_trigger_slot_switch_fires_adjust() {
    let vp = Viewport::new();

    // Create an active channel first (Adjust requires existing channel).
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
        rs.routing_focus_slot_ids[0] = 1;
    }

    vp.tick_attention_trigger(2_000_000);

    let ao = vp.read_action_output();
    let ch = ao
        .channels
        .iter()
        .find(|c| c.channel_kind == OutputChannelKind::GenericWrite)
        .expect("channel must still be present");
    assert!(
        ch.reinforce_count > 0,
        "slot switch must fire Adjust (reinforce_count must increment)"
    );
}

#[test]
fn attention_trigger_updates_last_top_slot_id() {
    let vp = Viewport::new();

    // Seed an active channel.
    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.4,
        event_t_us: 1_000_000,
    });

    {
        vp.probe_law.write().unwrap().last_top_slot_id = 2;
    }
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.routing_focus_slot_ids[0] = 3;
    }

    vp.tick_attention_trigger(2_000_000);

    let saved = vp.probe_law.read().unwrap().last_top_slot_id;
    assert_eq!(
        saved, 3,
        "last_top_slot_id must be updated to current top slot after switch"
    );
}

#[test]
fn attention_trigger_no_slot_switch_fires_nothing() {
    let vp = Viewport::new();
    let same_slot: i8 = 2;
    {
        vp.probe_law.write().unwrap().last_top_slot_id = same_slot;
    }
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.routing_focus_slot_ids[0] = same_slot;
    }

    vp.tick_attention_trigger(1_000_000);

    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 0,
        "same top slot must not fire any event"
    );
}

#[test]
fn attention_trigger_both_minus_one_fires_nothing() {
    let vp = Viewport::new();
    // Default: last_top_slot_id = -1, routing_focus_slot_ids[0] = -1.
    vp.tick_attention_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 0,
        "both -1 (no active slot) must not fire"
    );
}

// ─── Task 7: Relation-instability trigger ─────────────────────────────────────

#[test]
fn relation_instability_above_threshold_fires_begin() {
    let vp = Viewport::new();
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = true;
        gr.edges[0].pred.relation_surprise = RELATION_INSTABILITY_TRIGGER_THRESHOLD + 0.1;
    }
    vp.tick_relation_instability_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 1,
        "active edge with high relation_surprise must fire Begin"
    );
}

#[test]
fn relation_instability_at_threshold_fires_nothing() {
    let vp = Viewport::new();
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = true;
        gr.edges[0].pred.relation_surprise = RELATION_INSTABILITY_TRIGGER_THRESHOLD;
    }
    vp.tick_relation_instability_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 0,
        "relation_surprise exactly at threshold must not fire (strictly greater required)"
    );
}

#[test]
fn relation_instability_below_threshold_fires_nothing() {
    let vp = Viewport::new();
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = true;
        gr.edges[0].pred.relation_surprise = RELATION_INSTABILITY_TRIGGER_THRESHOLD * 0.5;
    }
    vp.tick_relation_instability_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(ao.n_write_epochs, 0, "low relation_surprise must not fire");
}

#[test]
fn relation_instability_inactive_edge_ignored() {
    let vp = Viewport::new();
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = false;
        gr.edges[0].pred.relation_surprise = 1.0;
    }
    vp.tick_relation_instability_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 0,
        "inactive edge must not trigger even with high surprise"
    );
}

#[test]
fn relation_instability_max_surprise_across_edges() {
    let vp = Viewport::new();
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = true;
        gr.edges[0].pred.relation_surprise = RELATION_INSTABILITY_TRIGGER_THRESHOLD * 0.5;
        gr.edges[1].active = true;
        gr.edges[1].pred.relation_surprise = RELATION_INSTABILITY_TRIGGER_THRESHOLD + 0.2;
    }
    vp.tick_relation_instability_trigger(1_000_000);
    let ao = vp.read_action_output();
    assert_eq!(
        ao.n_write_epochs, 1,
        "max surprise across active edges determines trigger; one high edge must fire"
    );
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
