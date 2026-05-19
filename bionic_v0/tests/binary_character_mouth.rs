use bionic_v0::binary_character_mouth::{BinaryCharacterMouthState, BinaryCharacterOutputDrive};
use bionic_v0::live_byte_closure::LIVE_BYTE_EGRESS_FRAME_LEN;

#[test]
fn mouth_organizes_bounded_output_burst_from_lane_pressure() {
    let mut mouth = BinaryCharacterMouthState::default();

    let burst = mouth.begin_output(BinaryCharacterOutputDrive {
        lane_id: 3,
        t_us: 20_000,
        pressure: 0.7,
        seed_hash: 0x1234,
        requested_len: 64,
    });

    assert_eq!(mouth.active_lane_id, 3);
    assert_eq!(mouth.output_count, 1);
    assert_eq!(burst.bytes.len(), LIVE_BYTE_EGRESS_FRAME_LEN);
    assert_eq!(burst.lane_id, 3);
    assert!(burst.output_ref != 0);
    assert!(mouth.output_pressure > 0.0);
    assert!(mouth.rhythm_score >= 0.0);
}

#[test]
fn mouth_return_updates_match_and_deviation_without_answer_state() {
    let mut mouth = BinaryCharacterMouthState::default();
    let burst = mouth.begin_output(BinaryCharacterOutputDrive {
        lane_id: 4,
        t_us: 30_000,
        pressure: 0.6,
        seed_hash: 0x77,
        requested_len: 8,
    });

    mouth.observe_return(4, 31_000, &burst.bytes, 0.5, 0.0);

    assert_eq!(mouth.return_count, 1);
    assert!(mouth.return_match > 0.5);
    assert_eq!(mouth.correction_deviation, 0.0);
    assert_eq!(mouth.active_lane_id, 4);
}
