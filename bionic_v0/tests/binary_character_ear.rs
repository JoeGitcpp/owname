use bionic_v0::binary_character_ear::{BinaryCharacterBurstInput, BinaryCharacterEarState};
use bionic_v0::viewport::Viewport;

#[test]
fn ear_separates_concurrent_fibers_into_distinct_lanes() {
    let mut ear = BinaryCharacterEarState::default();

    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 1,
        carrier_id: 10,
        t_start_us: 1_000,
        t_end_us: 1_100,
        bytes: b"hi",
        burst_energy: 0.4,
        framing_validity: 1.0,
    });
    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 2,
        carrier_id: 10,
        t_start_us: 1_020,
        t_end_us: 1_130,
        bytes: b"ok",
        burst_energy: 0.5,
        framing_validity: 1.0,
    });
    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 3,
        carrier_id: 99,
        t_start_us: 1_040,
        t_end_us: 1_150,
        bytes: b"no",
        burst_energy: 0.6,
        framing_validity: 1.0,
    });

    assert_eq!(ear.lane_count, 3);
    assert_eq!(ear.total_bursts, 3);
    assert_eq!(ear.lanes.len(), 3);
    assert!(ear.lanes.iter().all(|lane| lane.burst_count == 1));
}

#[test]
fn ear_repeated_fiber_raises_continuity_and_recurrence() {
    let mut ear = BinaryCharacterEarState::default();
    for idx in 0..4 {
        let t = 1_000 + idx * 1_000;
        ear.observe_burst(BinaryCharacterBurstInput {
            fiber_id: 7,
            carrier_id: 3,
            t_start_us: t,
            t_end_us: t + 100,
            bytes: b"aa",
            burst_energy: 0.3,
            framing_validity: 1.0,
        });
    }

    assert_eq!(ear.lane_count, 1);
    let lane = &ear.lanes[0];
    assert_eq!(lane.burst_count, 4);
    assert!(lane.continuity_score > 0.5);
    assert!(lane.recurrence_score > 0.5);
    assert!(lane.prediction_pressure < 0.5);
}

#[test]
fn ear_collision_preserves_ambiguity_without_merging_lanes() {
    let mut ear = BinaryCharacterEarState::default();
    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 1,
        carrier_id: 1,
        t_start_us: 2_000,
        t_end_us: 2_100,
        bytes: b"xx",
        burst_energy: 0.8,
        framing_validity: 1.0,
    });
    ear.observe_burst(BinaryCharacterBurstInput {
        fiber_id: 2,
        carrier_id: 1,
        t_start_us: 2_030,
        t_end_us: 2_120,
        bytes: b"xx",
        burst_energy: 0.8,
        framing_validity: 1.0,
    });

    assert_eq!(ear.lane_count, 2);
    assert!(ear.collision_pressure > 0.0);
    assert!(ear.lanes.iter().any(|lane| lane.collision_pressure > 0.0));
}

#[test]
fn viewport_observes_binary_character_burst_and_forwards_code_ingress() {
    let vp = Viewport::new();

    vp.observe_binary_character_burst(BinaryCharacterBurstInput {
        fiber_id: 11,
        carrier_id: 4,
        t_start_us: 10_000,
        t_end_us: 10_100,
        bytes: b"hi",
        burst_energy: 0.5,
        framing_validity: 1.0,
    });

    let ear = vp.read_binary_character_ear();
    let code = vp.read_code_ingress();

    assert_eq!(ear.lane_count, 1);
    assert_eq!(code.recent_frames.len(), 1);
    assert_eq!(
        code.recent_frames[0].stream_context_id,
        BinaryCharacterEarState::lane_stream_context_id(ear.active_lane_id)
    );
    assert_eq!(code.recent_frames[0].byte_len, 2);
    assert_eq!(vp.read_language_binding().n_bindings, 0);
}
