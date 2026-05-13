#[test]
fn training_curve_reports_rates_pressure_range_and_pair_contamination() {
    let dir = std::env::temp_dir().join(format!("bionic_v0_training_curve_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let csv = dir.join("session.csv");

    std::fs::write(
        &csv,
        "\
seq,clock_s,source_writes,code_frames,code_windows,egress_phase,egress_frames,egress_returns,egress_total_frames,egress_total_returns,pairs,pair_total,pair_framed_byte_total,pair_other_total,probe_i,probe_o,evidence_p,learned_axes,learned_event_total,voc_tE,voc_tp,voc_tr,voc_ob,voc_op,voc_or
1,0.000,10,4,1,1,1,1,10,9,2,2,2,0,0.0100,0.0100,0.0200,1,3,0,0,0,0,0,0
2,60.000,70,8,1,1,2,2,70,68,4,4,4,0,0.0200,0.0200,0.0300,1,9,0,0,0,0,0,0
3,120.000,150,12,1,1,3,3,130,128,6,7,6,1,0.0300,0.0300,0.0400,1,15,0,0,0,0,0,0
",
    )
    .unwrap();

    let stdout = bionic_v0::training_curve::summarize_path(&csv)
        .unwrap()
        .join("\n");

    assert!(stdout.contains("duration_s=120.000"));
    assert!(stdout.contains("egress_total_frames_per_min=60.000"));
    assert!(stdout.contains("egress_total_returns_per_min=59.500"));
    assert!(stdout.contains("pair_framed_byte_total_per_min=2.000"));
    assert!(stdout.contains("learned_event_total_per_min=6.000"));
    assert!(stdout.contains("evidence_p_min=0.0200"));
    assert!(stdout.contains("evidence_p_max=0.0400"));
    assert!(stdout.contains("pair_other_total_delta=1"));
    assert!(stdout.contains("closure_status=contaminated"));
}

#[test]
fn training_curve_combines_multiple_checkpoint_files() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_training_curve_multi_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let first = dir.join("session-check-1.csv");
    let second = dir.join("session-check-2.csv");
    let header = "seq,clock_s,source_writes,code_frames,code_windows,egress_phase,egress_frames,egress_returns,egress_total_frames,egress_total_returns,pairs,pair_total,pair_framed_byte_total,pair_other_total,probe_i,probe_o,evidence_p,learned_axes,learned_event_total,voc_tE,voc_tp,voc_tr,voc_ob,voc_op,voc_or\n";

    std::fs::write(
        &first,
        format!(
            "{header}1,60.000,10,4,1,1,1,1,20,18,2,2,2,0,0.0100,0.0100,0.0200,1,4,0,0,0,0,0,0\n"
        ),
    )
    .unwrap();
    std::fs::write(
        &second,
        format!("{header}2,180.000,30,4,1,1,1,1,220,218,2,42,42,0,0.0100,0.0100,0.0500,1,64,0,0,0,0,0,0\n"),
    )
    .unwrap();

    let stdout = bionic_v0::training_curve::summarize_paths(&[first, second])
        .unwrap()
        .join("\n");

    assert!(stdout.contains("rows=2"));
    assert!(stdout.contains("duration_s=120.000"));
    assert!(stdout.contains("egress_total_frames_per_min=100.000"));
    assert!(stdout.contains("pair_framed_byte_total_per_min=20.000"));
    assert!(stdout.contains("learned_event_total_per_min=30.000"));
    assert!(stdout.contains("closure_status=clean"));
}
