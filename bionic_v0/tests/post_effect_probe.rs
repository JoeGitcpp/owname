fn write_session(path: &std::path::Path, rows: &[(f64, u64, u64, u64, f64, f64, f64, f64)]) {
    let mut text = String::from(
        "seq,clock_s,source_writes,code_frames,code_windows,egress_phase,egress_frames,egress_returns,egress_total_frames,egress_total_returns,egress_pressure,egress_deviation,pair_similarity,pairs,pair_total,pair_framed_byte_total,pair_other_total,probe_i,probe_o,evidence_p,learned_axes,learned_event_total,voc_tE,voc_tp,voc_tr,voc_ob,voc_op,voc_or\n",
    );
    for (
        i,
        (
            clock_s,
            framed_pairs,
            other_pairs,
            learned_events,
            evidence_p,
            egress_pressure,
            egress_deviation,
            pair_similarity,
        ),
    ) in rows.iter().enumerate()
    {
        text.push_str(&format!(
            "{},{:.3},0,0,0,0,0,0,0,0,{:.4},{:.4},{:.4},0,{},{},{},0,0,{:.4},2,{},0,0,0,0,0,0\n",
            i + 1,
            clock_s,
            egress_pressure,
            egress_deviation,
            pair_similarity,
            framed_pairs + other_pairs,
            framed_pairs,
            other_pairs,
            evidence_p,
            learned_events
        ));
    }
    std::fs::write(path, text).unwrap();
}

#[test]
fn post_effect_probe_marks_clean_persistence_without_overclaiming_improvement() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_persistence_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cold = dir.join("cold.csv");
    let train = dir.join("train.csv");
    let after = dir.join("after.csv");
    write_session(
        &cold,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.30, 0.70),
            (120.0, 2000, 0, 2100, 0.052, 0.22, 0.28, 0.72),
        ],
    );
    write_session(
        &train,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.30, 0.70),
            (300.0, 5700, 0, 4400, 0.055, 0.30, 0.20, 0.80),
        ],
    );
    write_session(
        &after,
        &[
            (0.0, 0, 0, 4400, 0.052, 0.30, 0.20, 0.80),
            (120.0, 1990, 0, 6500, 0.049, 0.28, 0.18, 0.82),
        ],
    );

    let report = bionic_v0::post_effect::analyze_sessions(&cold, &train, &after).unwrap();

    assert_eq!(
        report.verdict,
        bionic_v0::post_effect::PostEffectVerdict::PersistenceOnly
    );
    assert!(report.clean_closure);
    assert!(report.persistence_visible);
    assert!(report.after_vs_cold_pair_rate_delta_pct.abs() < 5.0);
    assert!(report.after_vs_cold_egress_pressure_last_delta > 0.0);
    assert!(report.after_vs_cold_pressure_response_ratio_last_delta > 0.0);
    assert!(report.after_vs_cold_egress_deviation_last_delta < 0.0);
    assert!(report.after_vs_cold_pair_similarity_last_delta > 0.0);
    assert!(report.after_vs_cold_egress_deviation_avg_delta < 0.0);
    assert!(report.after_vs_cold_pair_similarity_avg_delta > 0.0);
    assert!(report
        .lines()
        .contains(&"verdict=persistence_only".to_string()));
    assert!(report
        .lines()
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_pressure_last_delta=")));
    assert!(report
        .lines()
        .iter()
        .any(|line| line.starts_with("after_vs_cold_pressure_response_ratio_last_delta=")));
    assert!(report
        .lines()
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_deviation_last_delta=")));
    assert!(report
        .lines()
        .iter()
        .any(|line| line.starts_with("after_vs_cold_pair_similarity_last_delta=")));
    assert!(report
        .lines()
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_deviation_avg_delta=")));
    assert!(report
        .lines()
        .iter()
        .any(|line| line.starts_with("after_vs_cold_pair_similarity_avg_delta=")));
    assert!(report.lines().contains(&"effect_status=pass".to_string()));
    assert!(report
        .lines()
        .contains(&"quality_status=avg_improved".to_string()));
    assert!(report
        .lines()
        .contains(&"pair_rate_status=stable".to_string()));
}

#[test]
fn post_effect_probe_refuses_contaminated_pair_closure() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_contaminated_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cold = dir.join("cold.csv");
    let train = dir.join("train.csv");
    let after = dir.join("after.csv");
    write_session(
        &cold,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.30, 0.70),
            (60.0, 1000, 0, 900, 0.052, 0.22, 0.28, 0.72),
        ],
    );
    write_session(
        &train,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.30, 0.70),
            (60.0, 1000, 1, 900, 0.052, 0.22, 0.28, 0.72),
        ],
    );
    write_session(
        &after,
        &[
            (0.0, 0, 0, 900, 0.052, 0.22, 0.28, 0.72),
            (60.0, 1000, 0, 1800, 0.049, 0.24, 0.26, 0.74),
        ],
    );

    let report = bionic_v0::post_effect::analyze_sessions(&cold, &train, &after).unwrap();

    assert_eq!(
        report.verdict,
        bionic_v0::post_effect::PostEffectVerdict::Contaminated
    );
    assert!(!report.clean_closure);
    assert!(report.lines().contains(&"clean_closure=false".to_string()));
}

#[test]
fn post_effect_probe_can_mark_a_conservative_positive_signal() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_positive_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cold = dir.join("cold.csv");
    let train = dir.join("train.csv");
    let after = dir.join("after.csv");
    write_session(
        &cold,
        &[
            (0.0, 0, 0, 0, 0.080, 0.20, 0.30, 0.70),
            (120.0, 1000, 0, 1000, 0.070, 0.22, 0.28, 0.72),
        ],
    );
    write_session(
        &train,
        &[
            (0.0, 0, 0, 0, 0.080, 0.20, 0.30, 0.70),
            (300.0, 5000, 0, 4200, 0.060, 0.32, 0.20, 0.80),
        ],
    );
    write_session(
        &after,
        &[
            (0.0, 0, 0, 4200, 0.058, 0.32, 0.20, 0.80),
            (120.0, 1300, 0, 6100, 0.052, 0.34, 0.18, 0.82),
        ],
    );

    let report = bionic_v0::post_effect::analyze_sessions(&cold, &train, &after).unwrap();

    assert_eq!(
        report.verdict,
        bionic_v0::post_effect::PostEffectVerdict::WeakPositive
    );
    assert!(report.after_vs_cold_pair_rate_delta_pct > 20.0);
    assert!(report.after_vs_cold_evidence_last_delta < 0.0);
}

#[test]
fn post_effect_probe_reports_late_window_retention_separately_from_full_average() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_late_retention_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cold = dir.join("cold.csv");
    let train = dir.join("train.csv");
    let after = dir.join("after.csv");
    write_session(
        &cold,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.25, 0.75),
            (40.0, 650, 0, 700, 0.058, 0.21, 0.31, 0.69),
            (80.0, 1300, 0, 1400, 0.055, 0.23, 0.30, 0.70),
            (120.0, 2000, 0, 2100, 0.052, 0.24, 0.30, 0.70),
        ],
    );
    write_session(
        &train,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.31, 0.69),
            (90.0, 1600, 0, 1800, 0.058, 0.23, 0.29, 0.71),
            (180.0, 3300, 0, 3600, 0.052, 0.25, 0.28, 0.72),
        ],
    );
    write_session(
        &after,
        &[
            (0.0, 0, 0, 3600, 0.052, 0.24, 0.33, 0.67),
            (100.0, 1750, 0, 5200, 0.050, 0.25, 0.31, 0.69),
            (200.0, 3600, 0, 6900, 0.048, 0.27, 0.27, 0.73),
            (300.0, 5750, 0, 8600, 0.046, 0.29, 0.26, 0.74),
        ],
    );

    let report = bionic_v0::post_effect::analyze_sessions(&cold, &train, &after).unwrap();

    assert!(report.after_vs_cold_egress_deviation_avg_delta > 0.0);
    assert!(report.after_vs_cold_egress_deviation_late_avg_delta < 0.0);
    assert!(report.after_vs_cold_pair_similarity_late_avg_delta > 0.0);
    assert!(report.lines().contains(&"effect_status=hold".to_string()));
    assert!(report
        .lines()
        .contains(&"late_quality_status=late_avg_improved".to_string()));
    assert!(report
        .lines()
        .contains(&"retention_status=pass".to_string()));
}

#[test]
fn post_effect_probe_reports_after_segment_windows_for_reentry_diagnosis() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_after_segments_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cold = dir.join("cold.csv");
    let train = dir.join("train.csv");
    let after = dir.join("after.csv");
    write_session(
        &cold,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.30, 0.70),
            (40.0, 650, 0, 700, 0.058, 0.21, 0.30, 0.70),
            (80.0, 1300, 0, 1400, 0.055, 0.23, 0.30, 0.70),
            (120.0, 2000, 0, 2100, 0.052, 0.24, 0.30, 0.70),
        ],
    );
    write_session(
        &train,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.31, 0.69),
            (90.0, 1600, 0, 1800, 0.058, 0.23, 0.29, 0.71),
            (180.0, 3300, 0, 3600, 0.052, 0.25, 0.28, 0.72),
        ],
    );
    write_session(
        &after,
        &[
            (0.0, 0, 0, 3600, 0.052, 0.24, 0.36, 0.64),
            (100.0, 1750, 0, 5200, 0.050, 0.25, 0.34, 0.66),
            (200.0, 3600, 0, 6900, 0.048, 0.27, 0.29, 0.71),
            (300.0, 5750, 0, 8600, 0.046, 0.29, 0.26, 0.74),
        ],
    );

    let report = bionic_v0::post_effect::analyze_sessions(&cold, &train, &after).unwrap();

    assert!(report.after_vs_cold_egress_deviation_first_half_delta > 0.0);
    assert!(report.after_vs_cold_egress_deviation_second_half_delta < 0.0);
    assert!(report.after_vs_cold_egress_deviation_last_quarter_delta < 0.0);
    assert!(report.after_vs_cold_egress_deviation_quarter_deltas[0] > 0.0);
    assert!(report.after_vs_cold_egress_deviation_quarter_deltas[1] > 0.0);
    assert!(report.after_vs_cold_egress_deviation_quarter_deltas[2] < 0.0);
    assert!(report.after_vs_cold_egress_deviation_quarter_deltas[3] < 0.0);
    assert!(report.after_vs_cold_pair_similarity_first_half_delta < 0.0);
    assert!(report.after_vs_cold_pair_similarity_second_half_delta > 0.0);
    assert!(report.after_vs_cold_pair_similarity_last_quarter_delta > 0.0);
    assert!(report.after_vs_cold_pair_similarity_quarter_deltas[0] < 0.0);
    assert!(report.after_vs_cold_pair_similarity_quarter_deltas[1] < 0.0);
    assert!(report.after_vs_cold_pair_similarity_quarter_deltas[2] > 0.0);
    assert!(report.after_vs_cold_pair_similarity_quarter_deltas[3] > 0.0);
    let lines = report.lines();
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_deviation_first_half_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_deviation_second_half_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_deviation_last_quarter_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_pair_similarity_first_half_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_pair_similarity_second_half_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_pair_similarity_last_quarter_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_deviation_q1_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_deviation_q4_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_pair_similarity_q1_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_pair_similarity_q4_delta=")));
}

#[test]
fn post_effect_probe_aligns_after_quality_segments_with_pressure_segments() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_after_pressure_segments_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cold = dir.join("cold.csv");
    let train = dir.join("train.csv");
    let after = dir.join("after.csv");
    write_session(
        &cold,
        &[
            (0.0, 0, 0, 0, 0.040, 0.040, 0.30, 0.70),
            (40.0, 650, 0, 700, 0.040, 0.040, 0.30, 0.70),
            (80.0, 1300, 0, 1400, 0.040, 0.040, 0.30, 0.70),
            (120.0, 2000, 0, 2100, 0.040, 0.040, 0.30, 0.70),
        ],
    );
    write_session(
        &train,
        &[
            (0.0, 0, 0, 0, 0.050, 0.050, 0.31, 0.69),
            (90.0, 1600, 0, 1800, 0.048, 0.052, 0.29, 0.71),
            (180.0, 3300, 0, 3600, 0.046, 0.054, 0.28, 0.72),
        ],
    );
    write_session(
        &after,
        &[
            (0.0, 0, 0, 3600, 0.060, 0.070, 0.36, 0.64),
            (100.0, 1750, 0, 5200, 0.058, 0.068, 0.34, 0.66),
            (200.0, 3600, 0, 6900, 0.030, 0.032, 0.29, 0.71),
            (300.0, 5750, 0, 8600, 0.028, 0.030, 0.26, 0.74),
        ],
    );

    let report = bionic_v0::post_effect::analyze_sessions(&cold, &train, &after).unwrap();

    assert!(report.after_vs_cold_egress_deviation_first_half_delta > 0.0);
    assert!(report.after_vs_cold_egress_pressure_first_half_delta > 0.0);
    assert!(report.after_vs_cold_evidence_first_half_delta > 0.0);
    assert!(report.after_vs_cold_egress_deviation_second_half_delta < 0.0);
    assert!(report.after_vs_cold_egress_pressure_second_half_delta < 0.0);
    assert!(report.after_vs_cold_evidence_second_half_delta < 0.0);
    assert!(report.after_vs_cold_egress_deviation_last_quarter_delta < 0.0);
    assert!(report.after_vs_cold_egress_pressure_last_quarter_delta < 0.0);
    assert!(report.after_vs_cold_evidence_last_quarter_delta < 0.0);
    assert!(report.after_vs_cold_egress_pressure_quarter_deltas[0] > 0.0);
    assert!(report.after_vs_cold_egress_pressure_quarter_deltas[3] < 0.0);
    assert!(report.after_vs_cold_evidence_quarter_deltas[0] > 0.0);
    assert!(report.after_vs_cold_evidence_quarter_deltas[3] < 0.0);
    let lines = report.lines();
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_pressure_first_half_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_pressure_second_half_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_pressure_last_quarter_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_evidence_first_half_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_evidence_second_half_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_evidence_last_quarter_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_pressure_q1_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_egress_pressure_q4_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_evidence_q1_delta=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("after_vs_cold_evidence_q4_delta=")));
}

#[test]
fn post_effect_probe_reports_late_quality_against_cold_quarter_variability() {
    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_cold_variability_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cold = dir.join("cold.csv");
    let train = dir.join("train.csv");
    let after = dir.join("after.csv");
    write_session(
        &cold,
        &[
            (0.0, 0, 0, 0, 0.040, 0.040, 0.20, 0.80),
            (40.0, 650, 0, 700, 0.040, 0.040, 0.21, 0.79),
            (80.0, 1300, 0, 1400, 0.040, 0.040, 0.20, 0.80),
            (120.0, 2000, 0, 2100, 0.040, 0.040, 0.18, 0.82),
        ],
    );
    write_session(
        &train,
        &[
            (0.0, 0, 0, 0, 0.050, 0.050, 0.20, 0.80),
            (90.0, 1600, 0, 1800, 0.048, 0.052, 0.18, 0.82),
            (180.0, 3300, 0, 3600, 0.046, 0.054, 0.17, 0.83),
        ],
    );
    write_session(
        &after,
        &[
            (0.0, 0, 0, 3600, 0.050, 0.050, 0.19, 0.81),
            (100.0, 1750, 0, 5200, 0.048, 0.052, 0.19, 0.81),
            (200.0, 3600, 0, 6900, 0.046, 0.054, 0.20, 0.80),
            (300.0, 5750, 0, 8600, 0.044, 0.056, 0.205, 0.795),
        ],
    );

    let report = bionic_v0::post_effect::analyze_sessions(&cold, &train, &after).unwrap();

    assert!((report.cold_egress_deviation_quarter_span - 0.03).abs() < 1e-9);
    assert!((report.cold_pair_similarity_quarter_span - 0.03).abs() < 1e-9);
    assert!(report.after_vs_cold_egress_deviation_quarter_deltas[3] > 0.0);
    assert!(report.after_vs_cold_egress_deviation_quarter_deltas[3] < 0.03);

    let lines = report.lines();
    assert!(lines.contains(&"late_variance_status=within_cold_variability".to_string()));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("cold_egress_deviation_quarter_span=")));
    assert!(lines
        .iter()
        .any(|line| line.starts_with("cold_pair_similarity_quarter_span=")));
}

#[test]
fn post_effect_probe_analyze_binary_prints_numeric_report() {
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_analyze_bin_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let cold = dir.join("cold.csv");
    let train = dir.join("train.csv");
    let after = dir.join("after.csv");
    write_session(
        &cold,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.30, 0.70),
            (120.0, 2000, 0, 2100, 0.052, 0.22, 0.28, 0.72),
        ],
    );
    write_session(
        &train,
        &[
            (0.0, 0, 0, 0, 0.060, 0.20, 0.30, 0.70),
            (300.0, 5700, 0, 4400, 0.055, 0.30, 0.20, 0.80),
        ],
    );
    write_session(
        &after,
        &[
            (0.0, 0, 0, 4400, 0.052, 0.30, 0.20, 0.80),
            (120.0, 1990, 0, 6500, 0.049, 0.28, 0.18, 0.82),
        ],
    );

    let output = Command::new(env!("CARGO_BIN_EXE_post_effect_probe"))
        .arg("analyze")
        .arg(&cold)
        .arg(&train)
        .arg(&after)
        .output()
        .expect("run post_effect_probe analyze");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("verdict=persistence_only"));
    assert!(stdout.contains("clean_closure=true"));
    assert!(stdout.contains("persistence_visible=true"));
}

#[test]
fn post_effect_probe_analyze_repeat_summarizes_existing_run_roots() {
    use std::process::Command;

    fn write_run(root: &std::path::Path, q4_after_deviation: f64) {
        std::fs::create_dir_all(root.join("cold")).unwrap();
        std::fs::create_dir_all(root.join("train")).unwrap();
        std::fs::create_dir_all(root.join("after")).unwrap();
        write_session(
            &root.join("cold").join("session.csv"),
            &[
                (0.0, 0, 0, 0, 0.040, 0.040, 0.20, 0.80),
                (40.0, 650, 0, 700, 0.040, 0.040, 0.21, 0.79),
                (80.0, 1300, 0, 1400, 0.040, 0.040, 0.20, 0.80),
                (120.0, 2000, 0, 2100, 0.040, 0.040, 0.18, 0.82),
            ],
        );
        write_session(
            &root.join("train").join("session.csv"),
            &[
                (0.0, 0, 0, 0, 0.050, 0.050, 0.20, 0.80),
                (90.0, 1600, 0, 1800, 0.048, 0.052, 0.18, 0.82),
                (180.0, 3300, 0, 3600, 0.046, 0.054, 0.17, 0.83),
            ],
        );
        write_session(
            &root.join("after").join("session.csv"),
            &[
                (0.0, 0, 0, 3600, 0.050, 0.050, 0.19, 0.81),
                (100.0, 1750, 0, 5200, 0.048, 0.052, 0.19, 0.81),
                (200.0, 3600, 0, 6900, 0.046, 0.054, 0.20, 0.80),
                (
                    300.0,
                    5750,
                    0,
                    8600,
                    0.044,
                    0.056,
                    q4_after_deviation,
                    1.0 - q4_after_deviation,
                ),
            ],
        );
    }

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_repeat_bin_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    let run1 = dir.join("run1");
    let run2 = dir.join("run2");
    write_run(&run1, 0.205);
    write_run(&run2, 0.195);

    let output = Command::new(env!("CARGO_BIN_EXE_post_effect_probe"))
        .arg("analyze-repeat")
        .arg(&run1)
        .arg(&run2)
        .output()
        .expect("run post_effect_probe analyze-repeat");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("repeat_runs=2"));
    assert!(stdout.contains("repeat_status=needs_more_runs"));
    assert!(stdout.contains("repeat_late_variance_nonblocking_runs=2"));
    assert!(stdout.contains("repeat_q4_egress_deviation_delta_avg="));
}

#[test]
fn post_effect_probe_run_binary_wires_three_stages_without_body_output() {
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_run_bin_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_post_effect_probe"))
        .env("BIONIC_BODY_BIN", env!("CARGO_BIN_EXE_body"))
        .env("BIONIC_LOOPBACK_BIN", env!("CARGO_BIN_EXE_byte_loopback"))
        .env("BIONIC_OBSERVER_BIN", env!("CARGO_BIN_EXE_observer"))
        .arg("run")
        .arg(&dir)
        .arg("2")
        .arg("4")
        .arg("2")
        .arg("5")
        .output()
        .expect("run post_effect_probe run");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("root="));
    assert!(stdout.contains("body_output_status=silent"));
    assert!(stdout.contains("verdict="));
    assert!(stdout.contains("persistence_visible=true"));
    assert!(dir.join("cold").join("session.csv").exists());
    assert!(dir.join("train").join("session.csv").exists());
    assert!(dir.join("after").join("session.csv").exists());
}

#[test]
fn post_effect_probe_run_hold_wires_idle_retention_stage_without_body_output() {
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_run_hold_bin_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_post_effect_probe"))
        .env("BIONIC_BODY_BIN", env!("CARGO_BIN_EXE_body"))
        .env("BIONIC_LOOPBACK_BIN", env!("CARGO_BIN_EXE_byte_loopback"))
        .env("BIONIC_OBSERVER_BIN", env!("CARGO_BIN_EXE_observer"))
        .arg("run-hold")
        .arg(&dir)
        .arg("1")
        .arg("2")
        .arg("1")
        .arg("1")
        .arg("5")
        .output()
        .expect("run post_effect_probe run-hold");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("hold_dir="));
    assert!(stdout.contains("hold_output_status=silent"));
    assert!(stdout.contains("hold_persistence_visible=true"));
    assert!(dir.join("cold").join("session.csv").exists());
    assert!(dir.join("train").join("session.csv").exists());
    assert!(dir.join("hold").join("session.csv").exists());
    assert!(dir.join("after").join("session.csv").exists());
}

#[test]
fn post_effect_probe_dose_curve_runs_multiple_training_durations() {
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_dose_bin_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_post_effect_probe"))
        .env("BIONIC_BODY_BIN", env!("CARGO_BIN_EXE_body"))
        .env("BIONIC_LOOPBACK_BIN", env!("CARGO_BIN_EXE_byte_loopback"))
        .env("BIONIC_OBSERVER_BIN", env!("CARGO_BIN_EXE_observer"))
        .arg("dose")
        .arg(&dir)
        .arg("1")
        .arg("1")
        .arg("5")
        .arg("1")
        .arg("2")
        .output()
        .expect("run post_effect_probe dose");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("dose_root="));
    assert!(stdout.contains("dose_train_secs=1"));
    assert!(stdout.contains("dose_train_secs=2"));
    assert!(stdout.contains("effect_status="));
    assert!(dir
        .join("train-1")
        .join("after")
        .join("session.csv")
        .exists());
    assert!(dir
        .join("train-2")
        .join("after")
        .join("session.csv")
        .exists());
}

#[test]
fn post_effect_probe_generalize_uses_distinct_after_disturbance() {
    use std::process::Command;

    let dir = std::env::temp_dir().join(format!(
        "bionic_v0_post_effect_generalize_bin_{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);

    let output = Command::new(env!("CARGO_BIN_EXE_post_effect_probe"))
        .env("BIONIC_BODY_BIN", env!("CARGO_BIN_EXE_body"))
        .env("BIONIC_LOOPBACK_BIN", env!("CARGO_BIN_EXE_byte_loopback"))
        .env("BIONIC_OBSERVER_BIN", env!("CARGO_BIN_EXE_observer"))
        .arg("generalize")
        .arg(&dir)
        .arg("1")
        .arg("2")
        .arg("1")
        .arg("5")
        .output()
        .expect("run post_effect_probe generalize");

    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("generalize_after_disturbance=alternate"));
    assert!(stdout.contains("effect_status="));
    let cold_seed = std::fs::read(dir.join("cold").join("live_bytes").join("in.frames")).unwrap();
    let after_seed = std::fs::read(dir.join("after").join("live_bytes").join("in.frames")).unwrap();
    assert_ne!(cold_seed, after_seed);
}
