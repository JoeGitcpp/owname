use bionic_v0::action_consequence::{
    ActionConsequenceWindowEvent, ConsequenceModalityKind, ConsequencePairStatus,
};
use bionic_v0::viewport::Viewport;
use bionic_v0::vocal_organ::VocalOrganPhase;
use bionic_v0::{MOD_PHOT, MOD_VIB, PHOT_BANDS, VIB_BANDS};

fn write_vib_sample(vp: &Viewport, slot_id: usize, t_us: u64, energy: f64) {
    let bands = [energy; VIB_BANDS];
    let pred = [energy * 0.80; VIB_BANDS];
    let err = [energy * 0.20; VIB_BANDS];
    let prec = [0.65; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &bands, &pred, &err, &prec);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);
}

fn write_phot_sample(vp: &Viewport, slot_id: usize, t_us: u64, energy: f64) {
    let obs = vec![energy; PHOT_BANDS];
    let pred = vec![energy * 0.82; PHOT_BANDS];
    let err = vec![energy * 0.18; PHOT_BANDS];
    let prec = vec![0.70; PHOT_BANDS];
    vp.write_slot_phot(slot_id, t_us, &obs, &pred, &err, &prec);
    vp.refresh_slot_fusion(slot_id, MOD_PHOT, t_us);
}

#[test]
fn vocal_write_with_vib_return_raises_predictable_return_strength() {
    let vp = Viewport::new();

    for step in 0..3_u64 {
        let t = 1_000_000 + step * 120_000;
        vp.begin_voicing(t, 0.56, 0.34, 0.52);
        let write_seq = vp.read_action_output().channels[0].current_write_seq;
        let idx = vp.begin_action_consequence_window(ActionConsequenceWindowEvent {
            modality_kind: ConsequenceModalityKind::GenericIngress,
            start_t_us: t + 20_000,
            end_t_us: t + 20_500,
            magnitude: 0.50,
        });
        let window_id = vp.read_action_consequence().windows[idx as usize].window_id;
        assert!(vp.pair_action_consequence(write_seq, window_id, 20_000));
        let ac = vp.read_action_consequence();
        let pair = &ac.pairs[step as usize];
        assert_eq!(pair.write_seq, write_seq);
        assert_eq!(pair.status, ConsequencePairStatus::Paired);
        vp.tick_auditory_visual_from_pairs(t + 22_000);
    }

    let state = vp.read_auditory_visual();
    assert!(
        state.predictable_return_strength > 0.40,
        "self-return strength should rise, got {}",
        state.predictable_return_strength
    );
    assert_eq!(state.return_closure_count, 3);
    assert!(state.average_return_delay_us > 0.0);
}

#[test]
fn aligned_vib_phot_activity_raises_cross_channel_predictability() {
    let vp = Viewport::new();
    let slot_id = vp.allocate_slot(MOD_VIB, 2_000_000, 1).unwrap();

    for step in 0..4_u64 {
        let t = 2_000_000 + step * 60_000;
        write_vib_sample(&vp, slot_id, t, 0.24);
        write_phot_sample(&vp, slot_id, t + 8_000, 0.28);
        vp.tick_auditory_visual_from_fusion(t + 9_000);
    }

    let state = vp.read_auditory_visual();
    assert!(
        state.cross_channel_predictability > 0.40,
        "aligned channel predictability should rise, got {}",
        state.cross_channel_predictability
    );
    assert_eq!(state.aligned_event_count, 4);
    assert!(state.alignment_pressure > 0.0);
}

#[test]
fn cross_channel_pressure_can_drive_non_symbolic_vocal_expression() {
    let vp = Viewport::new();
    let slot_id = vp.allocate_slot(MOD_VIB, 3_000_000, 1).unwrap();

    for step in 0..5_u64 {
        let t = 3_000_000 + step * 50_000;
        write_vib_sample(&vp, slot_id, t, 0.35);
        write_phot_sample(&vp, slot_id, t + 4_000, 0.40);
        vp.tick_auditory_visual_from_fusion(t + 5_000);
    }
    vp.tick_auditory_visual_expression(3_400_000);

    let state = vp.read_auditory_visual();
    let out = vp.read_action_output();
    let organ = vp.read_vocal_organ();
    assert!(
        state.expression_write_epochs >= 1,
        "cross-channel pressure should reach vocal write path"
    );
    assert!(out.n_write_epochs >= 1);
    assert_eq!(organ.phase, VocalOrganPhase::Active);
    assert!(state.expressive_pressure_peak > 0.35);

    let before = state.expressive_pressure;
    vp.observe_auditory_visual_expression_closure(3_450_000, 0.85);
    let after = vp.read_auditory_visual().expressive_pressure;
    assert!(after < before, "closure should lower expressive pressure");
}

#[test]
fn misaligned_vib_phot_activity_raises_drift_pressure() {
    let vp = Viewport::new();
    let slot_id = vp.allocate_slot(MOD_VIB, 4_000_000, 1).unwrap();

    write_vib_sample(&vp, slot_id, 4_000_000, 0.30);
    write_phot_sample(&vp, slot_id, 4_200_000, 0.32);
    vp.tick_auditory_visual_from_fusion(4_210_000);

    let state = vp.read_auditory_visual();
    assert!(
        state.drift_pressure > 0.05,
        "misalignment should leave drift pressure, got {}",
        state.drift_pressure
    );
    assert_eq!(state.drift_event_count, 1);
}

#[test]
fn auditory_visual_sources_do_not_introduce_recognition_or_language_ontology() {
    let src = include_str!("../src/auditory_visual.rs");
    for forbidden in [
        "speech",
        "word",
        "text",
        "semantic",
        "concept",
        "token",
        "label",
        "recognition",
        "meaning",
        "emotion",
        "intent",
        "println!",
        "eprintln!",
    ] {
        assert!(
            !src.contains(forbidden),
            "auditory_visual source must not introduce forbidden ontology/output: {forbidden}"
        );
    }
}
