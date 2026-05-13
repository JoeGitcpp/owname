use bionic_v0::framed_byte_egress::FramedByteEgressPhase;
use bionic_v0::viewport::Viewport;

#[test]
fn framed_byte_egress_records_body_side_write_out() {
    let vp = Viewport::new();

    let frame = vp.begin_framed_byte_egress(1_000, 11, &[0x61, 0x62, 0x61], 0.4);

    assert_eq!(frame.stream_id, 11);
    assert_eq!(frame.byte_len, 3);
    assert!(frame.write_seq > 0);

    let st = vp.read_framed_byte_egress();
    assert_eq!(st.phase, FramedByteEgressPhase::Active);
    assert_eq!(st.n_frames, 1);
    assert_eq!(st.output_pressure, 0.4);
}

#[test]
fn framed_byte_return_pairs_with_write_epoch_without_discrete_result_marker() {
    let vp = Viewport::new();
    let frame = vp.begin_framed_byte_egress(1_000, 11, &[0x61, 0x62], 0.5);

    let window_id = vp.observe_framed_byte_return(1_100, 11, &[0x62, 0x61], 0.5, 0.2);

    let acq = vp.read_action_consequence();
    assert!(window_id > 0);
    assert!(acq.n_pairs >= 1);
    assert_eq!(
        vp.read_framed_byte_egress().current_write_seq,
        frame.write_seq
    );
}

#[test]
fn framed_byte_egress_source_keeps_body_boundary() {
    let src = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/framed_byte_egress.rs"
    ))
    .expect("read framed_byte_egress.rs");

    let forbidden_patterns: &[&[u8]] = &[
        &[112, 114, 111, 109, 112, 116],
        &[97, 110, 115, 119, 101, 114],
        &[99, 104, 97, 116],
        &[99, 111, 109, 109, 97, 110, 100],
        &[115, 104, 101, 108, 108],
        &[109, 101, 97, 110, 105, 110, 103],
        &[116, 111, 107, 101, 110],
    ];

    for forbidden in forbidden_patterns {
        let forbidden = std::str::from_utf8(forbidden).expect("ascii pattern");
        assert!(
            !src.contains(forbidden),
            "framed byte egress source must not contain {forbidden}"
        );
    }
}
