use bionic_v0::action_consequence::{
    ActionConsequenceWindowEvent, ConsequenceModalityKind, ConsequencePairStatus,
};
use bionic_v0::action_output::{
    ActionOutputEvent, ActionOutputFeedback, ActionOutputPhase, OutputChannelKind,
};
use bionic_v0::adapter::{ModalityAdapter, RealPhotAdapter};
use bionic_v0::phot_camera::{BlobSpec, SynthCamStep, SyntheticCameraFrameProvider};
use bionic_v0::probe_law::ProbePhase;
use bionic_v0::snapshot::{read_snapshot, write_snapshot, SnapshotPaths};
use bionic_v0::viewport::Viewport;
use bionic_v0::{MOD_PHOT, N_OBJECT_SLOTS};
use std::sync::Arc;

#[test]
fn manual_vocal_begin_records_action_write_epoch() {
    let vp = Viewport::new();

    vp.begin_voicing(1_000_000, 0.6, 0.3, 0.5);

    let organ = vp.read_vocal_organ();
    assert!(organ.breath_drive > 0.0);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.write_epoch_history.len(), 1);

    let epoch = &out.write_epoch_history[0];
    assert_eq!(epoch.channel_kind, OutputChannelKind::GenericWrite);
    assert_eq!(epoch.start_t_us, 1_000_000);
    assert_eq!(epoch.last_update_t_us, 1_000_000);
    assert_eq!(epoch.last_phase, ActionOutputPhase::Active);
    assert_eq!(epoch.write_seq, out.channels[0].current_write_seq);
}

#[test]
fn vocal_imitation_loop_records_epoch_when_target_starts_voicing() {
    let vp = Viewport::new();

    vp.observe_vocal_target(1_000_000, 0.8, 0.35, 0.60, 0.20);
    vp.tick_vocal_imitation_loop(1_010_000);

    let organ = vp.read_vocal_organ();
    assert!(organ.breath_drive > 0.0);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.write_epoch_history.len(), 1);
    assert_eq!(out.write_epoch_history[0].start_t_us, 1_010_000);
}

#[test]
fn vocal_imitation_adjusts_existing_write_epoch_while_voicing() {
    let vp = Viewport::new();

    vp.observe_vocal_target(1_000_000, 0.8, 0.35, 0.60, 0.20);
    vp.tick_vocal_imitation_loop(1_010_000);
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    vp.observe_vocal_target(1_020_000, 0.9, 0.45, 0.70, 0.20);
    vp.tick_vocal_imitation_loop(1_030_000);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.channels[0].current_write_seq, write_seq);
    assert_eq!(out.channels[0].last_update_t_us, 1_030_000);
    assert_eq!(out.channels[0].reinforce_count, 1);
    assert_eq!(out.write_epoch_history[0].last_update_t_us, 1_030_000);
    assert_eq!(
        out.write_epoch_history[0].last_phase,
        ActionOutputPhase::Active
    );
}

#[test]
fn vocal_decay_to_idle_stops_existing_write_epoch() {
    let vp = Viewport::new();

    vp.begin_voicing(1_000_000, 0.6, 0.3, 0.5);
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    vp.tick_vocal_imitation_loop(20_000_000);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.channels[0].current_write_seq, write_seq);
    assert_eq!(out.channels[0].phase, ActionOutputPhase::Idle);
    assert_eq!(out.channels[0].current_magnitude, 0.0);
    assert_eq!(out.write_epoch_history[0].last_update_t_us, 20_000_000);
    assert_eq!(
        out.write_epoch_history[0].last_phase,
        ActionOutputPhase::Idle
    );
}

#[test]
fn vocal_write_epoch_can_pair_with_vib_consequence_window() {
    let vp = Viewport::new();

    vp.begin_voicing(1_000_000, 0.6, 0.3, 0.5);
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    vp.accumulate_ingress_change(1_020_000, 0.5);

    assert!(vp.try_auto_pair_latest_epoch_to_window(1_020_000));

    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_pairs, 1);
    assert_eq!(ac.pairs[0].write_seq, write_seq);
    assert_eq!(ac.pairs[0].delay_us, 20_000);
    assert_eq!(ac.pairs[0].status, ConsequencePairStatus::Paired);
}

#[test]
fn external_vib_without_write_epoch_remains_unpaired() {
    let vp = Viewport::new();

    vp.accumulate_ingress_change(1_020_000, 0.5);

    assert!(!vp.try_auto_pair_latest_epoch_to_window(1_020_000));

    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_windows, 1);
    assert_eq!(ac.n_pairs, 0);
    assert!(ac.pairs.is_empty());
}

#[test]
fn overlapping_windows_remain_independent_numeric_windows() {
    let vp = Viewport::new();

    vp.begin_voicing(1_000_000, 0.6, 0.3, 0.5);
    let write_seq = vp.read_action_output().channels[0].current_write_seq;

    let idx_a = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1_010_000,
        end_t_us: 1_030_000,
        magnitude: 0.4,
    });
    let idx_b = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 1_015_000,
        end_t_us: 1_035_000,
        magnitude: 0.7,
    });

    let ac_before = vp.read_action_consequence();
    let window_a = ac_before.windows[idx_a as usize].window_id;
    let window_b = ac_before.windows[idx_b as usize].window_id;
    assert_ne!(window_a, window_b);

    assert!(vp.pair_action_consequence(write_seq, window_b, 15_000));

    let ac = vp.read_action_consequence();
    assert_eq!(ac.n_windows, 2);
    assert_eq!(ac.n_pairs, 1);
    assert_eq!(ac.pairs[0].window_id, window_b);
    assert_eq!(ac.pairs[0].delay_us, 15_000);
}

#[test]
fn probe_law_active_tick_starts_physical_vocal_write() {
    let vp = Viewport::new();
    {
        let mut st = vp.probe_law.write().unwrap();
        st.intensity = 0.6;
        st.suppression = 0.2;
        st.phase = ProbePhase::P2;
    }

    vp.tick_probe_output_from_law(3_000_000);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);

    let organ = vp.read_vocal_organ();
    assert!(organ.breath_drive > 0.0);
    assert!((organ.breath_drive - 0.6).abs() < 1e-9);
    assert_eq!(organ.last_update_t_us, 3_000_000);
}

#[test]
fn probe_law_begin_uses_active_vocal_target_curve() {
    let vp = Viewport::new();
    vp.observe_vocal_target(2_900_000, 0.8, 0.22, 0.78, 0.20);
    {
        let mut st = vp.probe_law.write().unwrap();
        st.intensity = 0.6;
        st.suppression = 0.2;
        st.phase = ProbePhase::P2;
    }

    vp.tick_probe_output_from_law(3_000_000);

    let organ = vp.read_vocal_organ();
    assert!((organ.pitch_tension - 0.22).abs() < 1e-9);
    assert!((organ.resonance_position - 0.78).abs() < 1e-9);
}

#[test]
fn probe_pair_delay_adjusts_physical_vocal_magnitude() {
    let vp = Viewport::new();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude: 0.2,
        event_t_us: 1_000_000,
    });
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    vp.push_action_consequence_window(ActionConsequenceWindowEvent {
        modality_kind: ConsequenceModalityKind::GenericIngress,
        start_t_us: 3_000_000,
        end_t_us: 3_020_000,
        magnitude: 0.2,
    });
    let window_id = vp.read_action_consequence().windows[0].window_id;
    assert!(vp.pair_action_consequence(write_seq, window_id, 2_000_000));

    vp.begin_voicing(4_000_000, 1.0, 0.3, 0.5);
    {
        let mut st = vp.probe_law.write().unwrap();
        st.intensity = 1.0;
        st.suppression = 0.2;
        st.phase = ProbePhase::P2;
    }

    vp.tick_probe_output_from_law(4_100_000);

    let organ = vp.read_vocal_organ();
    assert!((organ.breath_drive - 0.5).abs() < 1e-9);
    assert_eq!(organ.last_update_t_us, 4_100_000);

    let out = vp.read_action_output();
    assert!((out.channels[0].current_magnitude - 0.5).abs() < 1e-9);
    assert_eq!(out.channels[0].last_update_t_us, 4_100_000);
}

#[test]
fn surprise_pressure_refresh_starts_physical_vocal_write() {
    let vp = Viewport::new();
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.global_surprise_load = 0.7;
    }

    vp.refresh_probe_law(5_000_000);
    vp.tick_probe_output_from_law(5_000_000);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);

    let organ = vp.read_vocal_organ();
    assert!(organ.breath_drive > 0.0);
    assert_eq!(organ.last_update_t_us, 5_000_000);
}

#[test]
fn relation_pressure_refresh_starts_physical_vocal_write() {
    let vp = Viewport::new();
    {
        let mut gr = vp.global_relations.write().unwrap();
        gr.edges[0].active = true;
        gr.edges[0].pred.relation_surprise = 0.7;
    }

    vp.refresh_probe_law(5_100_000);
    vp.tick_probe_output_from_law(5_100_000);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);

    let organ = vp.read_vocal_organ();
    assert!(organ.breath_drive > 0.0);
    assert_eq!(organ.last_update_t_us, 5_100_000);
}

#[test]
fn slot_switch_pressure_refresh_adjusts_physical_vocal_write() {
    let vp = Viewport::new();
    vp.begin_voicing(5_200_000, 0.2, 0.3, 0.5);
    {
        vp.probe_law.write().unwrap().last_top_slot_id = 0;
    }
    {
        let mut rs = vp.global_reasoning.write().unwrap();
        rs.routing_focus_slot_ids[0] = 1;
    }

    vp.refresh_probe_law(5_210_000);
    vp.tick_probe_output_from_law(5_210_000);

    let organ = vp.read_vocal_organ();
    assert!(organ.breath_drive > 0.0);
    assert_eq!(organ.last_update_t_us, 5_210_000);

    let out = vp.read_action_output();
    assert_eq!(out.n_write_epochs, 1);
    assert_eq!(out.channels[0].reinforce_count, 1);
    assert!(out.channels[0].current_magnitude > 0.0);
}

#[test]
fn synthetic_phot_preflight_records_slots_spatial_and_relations() {
    let vp = Arc::new(Viewport::new());
    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::MultiBlob(vec![
            BlobSpec {
                cx: 0.15,
                cy: 0.35,
                r: 0.12,
                brightness: 0.9,
            },
            BlobSpec {
                cx: 0.85,
                cy: 0.65,
                r: 0.12,
                brightness: 0.9,
            },
        ]),
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);

    adapter.tick(&vp, 1_000_000);
    adapter.tick(&vp, 1_033_333);

    let mut phot_slots = 0;
    for slot_id in 0..N_OBJECT_SLOTS {
        let slot = vp.read_slot(slot_id).unwrap();
        if slot.active && (slot.source_modality & MOD_PHOT) != 0 {
            phot_slots += 1;
            let spatial = slot.spatial.expect("PHOT slot must carry spatial state");
            assert!(spatial.valid);
            assert!(spatial.precision > 0.0);
            assert!(spatial.last_update_t_us > 0);
        }
    }
    assert!(phot_slots >= 2);

    vp.refresh_global_attention(1_040_000);
    vp.refresh_global_relations(1_040_000);
    let relations = vp.read_global_relations();
    assert!(relations.n_active_edges > 0);
    assert!(relations.edges.iter().any(|edge| edge.active));
}

#[test]
fn snapshot_preflight_exposes_live_io_evidence() {
    let vp = Arc::new(Viewport::new());

    vp.begin_voicing(2_000_000, 0.6, 0.3, 0.5);
    let write_seq = vp.read_action_output().channels[0].current_write_seq;
    vp.accumulate_ingress_change(2_020_000, 0.5);
    assert!(vp.try_auto_pair_latest_epoch_to_window(2_020_000));

    let script = vec![
        SynthCamStep::Idle,
        SynthCamStep::MultiBlob(vec![
            BlobSpec {
                cx: 0.2,
                cy: 0.3,
                r: 0.12,
                brightness: 0.9,
            },
            BlobSpec {
                cx: 0.8,
                cy: 0.7,
                r: 0.12,
                brightness: 0.9,
            },
        ]),
    ];
    let provider = Box::new(SyntheticCameraFrameProvider::new(script, 0, 33_333));
    let mut adapter = RealPhotAdapter::new(provider);
    adapter.tick(&vp, 2_030_000);
    adapter.tick(&vp, 2_063_333);
    vp.refresh_global_attention(2_070_000);
    vp.refresh_global_relations(2_070_000);

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_live_io_preflight_snap_{}",
        std::process::id()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    let paths = SnapshotPaths::new(&dir);
    write_snapshot(&vp, &paths).unwrap();
    let snap = read_snapshot(&paths.final_path).unwrap();

    assert_eq!(snap.action_output.n_write_epochs, 1);
    assert_eq!(
        snap.action_output.write_epoch_history[0].write_seq,
        write_seq
    );
    assert_eq!(snap.action_consequence.n_pairs, 1);
    assert_eq!(snap.action_consequence.pairs[0].write_seq, write_seq);
    assert!(snap.vocal_organ.breath_drive > 0.0);
    assert!(snap.phot_last_frame_t_us > 0);

    let phot_slots = snap
        .slots
        .iter()
        .filter(|slot| {
            slot.active
                && (slot.source_modality & MOD_PHOT) != 0
                && slot.spatial.map(|spatial| spatial.valid).unwrap_or(false)
        })
        .count();
    assert!(phot_slots >= 2);
    assert!(snap.global_relations.n_active_edges > 0);
}
