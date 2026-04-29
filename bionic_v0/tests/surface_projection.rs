use bionic_v0::language_binding::LanguageBindingState;
use bionic_v0::snapshot::{
    SnapshotActionConsequence, SnapshotActionOutput, SnapshotAmbientSub, SnapshotData,
    SnapshotLastClaim, SnapshotProbeLaw, SnapshotSlot, SnapshotSpatial, SnapshotVocalMemory,
};
use bionic_v0::surface_projection::project_surface_frame;
use bionic_v0::viewport::{
    GlobalAttentionState, GlobalReasoningMemorySummary, GlobalReasoningState, GlobalRelationState,
};
use bionic_v0::vocal_imitation::VocalImitationState;
use bionic_v0::vocal_organ::VocalOrganState;
use bionic_v0::vocal_target::VocalTargetState;
use bionic_v0::vpc::VpcContractState;

fn make_empty_snapshot() -> SnapshotData {
    SnapshotData {
        seq: 0,
        master_tick: 0,
        clock_us: 0,
        start_us: 0,
        source_writes: 0,
        predictor_ticks: 0,
        snapshot_fail_count: 0,
        history_cap: 0,
        axis_count: 0,
        hearing_fault: false,
        hearing_fault_count: 0,
        hl_a: 0.0,
        hl_avg_mse: 0.0,
        hl_pred_mse: 0.0,
        hl_meta_err: 0.0,
        hl_modulation: 0.0,
        last_obs_t_us: 0,
        health_state: 0,
        phot_health: 0,
        phot_started_t_us: 0,
        phot_last_frame_t_us: 0,
        axes: Vec::new(),
        ambient_nomination_seq: 0,
        ambient_vib: SnapshotAmbientSub::default(),
        ambient_phot: SnapshotAmbientSub::default(),
        ambient_mech: SnapshotAmbientSub::default(),
        slots: Vec::new(),
        last_claim: SnapshotLastClaim::default(),
        global_attention: GlobalAttentionState::default(),
        global_relations: GlobalRelationState::default(),
        global_reasoning: GlobalReasoningState::default(),
        global_reasoning_memory: GlobalReasoningMemorySummary::default(),
        language_binding: LanguageBindingState::default(),
        vpc_contract: VpcContractState::default(),
        action_output: SnapshotActionOutput::default(),
        action_consequence: SnapshotActionConsequence::default(),
        probe_law: SnapshotProbeLaw::default(),
        vocal_target: VocalTargetState::default(),
        vocal_organ: VocalOrganState::default(),
        vocal_imitation: VocalImitationState::default(),
        vocal_memory: SnapshotVocalMemory::default(),
    }
}

fn make_snapshot_with_one_slot(
    pos_x: f64,
    pos_y: f64,
    span_x: f64,
    span_y: f64,
    attention_share: f64,
    active: bool,
) -> SnapshotData {
    let mut snap = make_empty_snapshot();
    let spatial = SnapshotSpatial {
        pos_x,
        pos_y,
        span_x,
        span_y,
        precision: 0.5,
        last_update_t_us: 1000,
        valid: true,
    };
    let slot = SnapshotSlot {
        active,
        last_update_t_us: 1000,
        source_modality: 0b0000_0001,
        nomination_seq: 1,
        vib: None,
        phot: None,
        mech: None,
        spatial: Some(spatial),
        fusion: None,
        high_layer: None,
        attention_share,
        precision_bias: 0.0,
    };
    snap.slots = vec![slot];
    snap
}

fn make_snapshot_with_mixed_slots() -> SnapshotData {
    let mut snap = make_empty_snapshot();

    let active_spatial = SnapshotSpatial {
        pos_x: 0.5,
        pos_y: 0.5,
        span_x: 0.1,
        span_y: 0.1,
        precision: 0.5,
        last_update_t_us: 2000,
        valid: true,
    };
    let inactive_slot = SnapshotSlot {
        active: false,
        last_update_t_us: 500,
        source_modality: 0b0000_0001,
        nomination_seq: 1,
        vib: None,
        phot: None,
        mech: None,
        spatial: Some(active_spatial.clone()),
        fusion: None,
        high_layer: None,
        attention_share: 0.0,
        precision_bias: 0.0,
    };
    let invalid_spatial = SnapshotSpatial {
        pos_x: 0.0,
        pos_y: 0.0,
        span_x: 0.0,
        span_y: 0.0,
        precision: 0.5,
        last_update_t_us: 500,
        valid: false,
    };
    let invalid_slot = SnapshotSlot {
        active: true,
        last_update_t_us: 500,
        source_modality: 0b0000_0010,
        nomination_seq: 2,
        vib: None,
        phot: None,
        mech: None,
        spatial: Some(invalid_spatial),
        fusion: None,
        high_layer: None,
        attention_share: 0.0,
        precision_bias: 0.0,
    };
    let valid_slot = SnapshotSlot {
        active: true,
        last_update_t_us: 2000,
        source_modality: 0b0000_0100,
        nomination_seq: 3,
        vib: None,
        phot: None,
        mech: None,
        spatial: Some(active_spatial),
        fusion: None,
        high_layer: None,
        attention_share: 0.6,
        precision_bias: 1.0,
    };
    snap.slots = vec![inactive_slot, invalid_slot, valid_slot];
    snap
}

#[test]
fn projection_uses_active_slot_spatial_as_object_center_and_extent() {
    let snap = make_snapshot_with_one_slot(0.25, 0.75, 0.20, 0.10, 0.6, true);
    let frame = project_surface_frame(&snap);

    assert_eq!(frame.objects.len(), 1);
    let g = &frame.objects[0];
    assert!(g.center_x > 0.20 && g.center_x < 0.30);
    assert!(g.center_y > 0.70 && g.center_y < 0.80);
    assert!(g.radius_x > 0.0);
    assert!(g.radius_y > 0.0);
}

#[test]
fn projection_ignores_inactive_or_invalid_slots() {
    let snap = make_snapshot_with_mixed_slots();
    let frame = project_surface_frame(&snap);

    assert_eq!(frame.objects.len(), 1);
}
