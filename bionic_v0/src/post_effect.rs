use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::path::Path;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PostEffectVerdict {
    Contaminated,
    Insufficient,
    PersistenceOnly,
    WeakPositive,
}

#[derive(Clone, Debug)]
pub struct PostEffectReport {
    pub verdict: PostEffectVerdict,
    pub clean_closure: bool,
    pub persistence_visible: bool,
    pub cold_pair_framed_byte_per_min: f64,
    pub train_pair_framed_byte_per_min: f64,
    pub after_pair_framed_byte_per_min: f64,
    pub after_vs_cold_pair_rate_delta_pct: f64,
    pub after_vs_cold_evidence_last_delta: f64,
    pub after_vs_cold_egress_pressure_last_delta: f64,
    pub after_vs_cold_pressure_response_ratio_last_delta: f64,
    pub after_vs_cold_egress_deviation_last_delta: f64,
    pub after_vs_cold_pair_similarity_last_delta: f64,
    pub after_vs_cold_egress_deviation_avg_delta: f64,
    pub after_vs_cold_pair_similarity_avg_delta: f64,
    pub after_vs_cold_egress_deviation_late_avg_delta: f64,
    pub after_vs_cold_pair_similarity_late_avg_delta: f64,
    pub after_vs_cold_egress_deviation_first_half_delta: f64,
    pub after_vs_cold_egress_deviation_second_half_delta: f64,
    pub after_vs_cold_egress_deviation_last_quarter_delta: f64,
    pub after_vs_cold_pair_similarity_first_half_delta: f64,
    pub after_vs_cold_pair_similarity_second_half_delta: f64,
    pub after_vs_cold_pair_similarity_last_quarter_delta: f64,
    pub after_vs_cold_egress_deviation_quarter_deltas: [f64; 4],
    pub after_vs_cold_pair_similarity_quarter_deltas: [f64; 4],
    pub cold_egress_deviation_quarter_span: f64,
    pub cold_pair_similarity_quarter_span: f64,
    pub after_vs_cold_egress_pressure_first_half_delta: f64,
    pub after_vs_cold_egress_pressure_second_half_delta: f64,
    pub after_vs_cold_egress_pressure_last_quarter_delta: f64,
    pub after_vs_cold_evidence_first_half_delta: f64,
    pub after_vs_cold_evidence_second_half_delta: f64,
    pub after_vs_cold_evidence_last_quarter_delta: f64,
    pub after_vs_cold_egress_pressure_quarter_deltas: [f64; 4],
    pub after_vs_cold_evidence_quarter_deltas: [f64; 4],
    pub cold_pair_other_delta: u64,
    pub train_pair_other_delta: u64,
    pub after_pair_other_delta: u64,
    pub train_learned_event_last: u64,
    pub after_learned_event_first: u64,
    pub after_learned_event_last: u64,
}

#[derive(Clone, Debug)]
pub struct PostEffectRepeatReport {
    pub repeat_runs: usize,
    pub repeat_clean_runs: usize,
    pub repeat_persistent_runs: usize,
    pub repeat_retention_pass_runs: usize,
    pub repeat_late_variance_nonblocking_runs: usize,
    pub repeat_q4_egress_deviation_delta_avg: f64,
    pub repeat_q4_egress_deviation_delta_min: f64,
    pub repeat_q4_egress_deviation_delta_max: f64,
    pub repeat_avg_egress_deviation_delta_avg: f64,
    pub repeat_late_egress_deviation_delta_avg: f64,
}

#[derive(Clone, Debug)]
struct StageMetrics {
    pair_framed_byte_per_min: f64,
    pair_other_delta: u64,
    learned_event_first: u64,
    learned_event_last: u64,
    evidence_last: f64,
    egress_pressure_last: f64,
    egress_deviation_last: f64,
    pair_similarity_last: f64,
    pressure_response_ratio_last: f64,
    egress_deviation_avg: f64,
    pair_similarity_avg: f64,
    egress_deviation_late_avg: f64,
    pair_similarity_late_avg: f64,
    egress_deviation_first_half_avg: f64,
    egress_deviation_second_half_avg: f64,
    egress_deviation_last_quarter_avg: f64,
    pair_similarity_first_half_avg: f64,
    pair_similarity_second_half_avg: f64,
    pair_similarity_last_quarter_avg: f64,
    egress_deviation_quarter_avgs: [f64; 4],
    pair_similarity_quarter_avgs: [f64; 4],
    egress_pressure_first_half_avg: f64,
    egress_pressure_second_half_avg: f64,
    egress_pressure_last_quarter_avg: f64,
    evidence_first_half_avg: f64,
    evidence_second_half_avg: f64,
    evidence_last_quarter_avg: f64,
    egress_pressure_quarter_avgs: [f64; 4],
    evidence_quarter_avgs: [f64; 4],
    sufficient: bool,
}

#[derive(Clone, Debug)]
struct Row {
    clock_s: f64,
    pair_framed_byte_total: f64,
    pair_other_total: f64,
    learned_event_total: f64,
    evidence_p: f64,
    egress_pressure: f64,
    egress_deviation: f64,
    pair_similarity: f64,
}

pub fn analyze_sessions(
    cold_csv: &Path,
    train_csv: &Path,
    after_csv: &Path,
) -> std::io::Result<PostEffectReport> {
    let cold = read_stage(cold_csv)?;
    let train = read_stage(train_csv)?;
    let after = read_stage(after_csv)?;

    let clean_closure =
        cold.pair_other_delta == 0 && train.pair_other_delta == 0 && after.pair_other_delta == 0;
    let persistence_visible =
        train.learned_event_last > 0 && after.learned_event_first >= train.learned_event_last;
    let after_vs_cold_pair_rate_delta_pct = pct_delta(
        after.pair_framed_byte_per_min,
        cold.pair_framed_byte_per_min,
    );
    let after_vs_cold_evidence_last_delta = after.evidence_last - cold.evidence_last;
    let after_vs_cold_egress_pressure_last_delta =
        after.egress_pressure_last - cold.egress_pressure_last;
    let after_vs_cold_pressure_response_ratio_last_delta =
        after.pressure_response_ratio_last - cold.pressure_response_ratio_last;
    let after_vs_cold_egress_deviation_last_delta =
        after.egress_deviation_last - cold.egress_deviation_last;
    let after_vs_cold_pair_similarity_last_delta =
        after.pair_similarity_last - cold.pair_similarity_last;
    let after_vs_cold_egress_deviation_avg_delta =
        after.egress_deviation_avg - cold.egress_deviation_avg;
    let after_vs_cold_pair_similarity_avg_delta =
        after.pair_similarity_avg - cold.pair_similarity_avg;
    let after_vs_cold_egress_deviation_late_avg_delta =
        after.egress_deviation_late_avg - cold.egress_deviation_late_avg;
    let after_vs_cold_pair_similarity_late_avg_delta =
        after.pair_similarity_late_avg - cold.pair_similarity_late_avg;
    let after_vs_cold_egress_deviation_first_half_delta =
        after.egress_deviation_first_half_avg - cold.egress_deviation_first_half_avg;
    let after_vs_cold_egress_deviation_second_half_delta =
        after.egress_deviation_second_half_avg - cold.egress_deviation_second_half_avg;
    let after_vs_cold_egress_deviation_last_quarter_delta =
        after.egress_deviation_last_quarter_avg - cold.egress_deviation_last_quarter_avg;
    let after_vs_cold_pair_similarity_first_half_delta =
        after.pair_similarity_first_half_avg - cold.pair_similarity_first_half_avg;
    let after_vs_cold_pair_similarity_second_half_delta =
        after.pair_similarity_second_half_avg - cold.pair_similarity_second_half_avg;
    let after_vs_cold_pair_similarity_last_quarter_delta =
        after.pair_similarity_last_quarter_avg - cold.pair_similarity_last_quarter_avg;
    let after_vs_cold_egress_deviation_quarter_deltas = quarter_deltas(
        after.egress_deviation_quarter_avgs,
        cold.egress_deviation_quarter_avgs,
    );
    let after_vs_cold_pair_similarity_quarter_deltas = quarter_deltas(
        after.pair_similarity_quarter_avgs,
        cold.pair_similarity_quarter_avgs,
    );
    let cold_egress_deviation_quarter_span = span(cold.egress_deviation_quarter_avgs);
    let cold_pair_similarity_quarter_span = span(cold.pair_similarity_quarter_avgs);
    let after_vs_cold_egress_pressure_first_half_delta =
        after.egress_pressure_first_half_avg - cold.egress_pressure_first_half_avg;
    let after_vs_cold_egress_pressure_second_half_delta =
        after.egress_pressure_second_half_avg - cold.egress_pressure_second_half_avg;
    let after_vs_cold_egress_pressure_last_quarter_delta =
        after.egress_pressure_last_quarter_avg - cold.egress_pressure_last_quarter_avg;
    let after_vs_cold_evidence_first_half_delta =
        after.evidence_first_half_avg - cold.evidence_first_half_avg;
    let after_vs_cold_evidence_second_half_delta =
        after.evidence_second_half_avg - cold.evidence_second_half_avg;
    let after_vs_cold_evidence_last_quarter_delta =
        after.evidence_last_quarter_avg - cold.evidence_last_quarter_avg;
    let after_vs_cold_egress_pressure_quarter_deltas = quarter_deltas(
        after.egress_pressure_quarter_avgs,
        cold.egress_pressure_quarter_avgs,
    );
    let after_vs_cold_evidence_quarter_deltas =
        quarter_deltas(after.evidence_quarter_avgs, cold.evidence_quarter_avgs);
    let sufficient = cold.sufficient && train.sufficient && after.sufficient;

    let verdict = if !clean_closure {
        PostEffectVerdict::Contaminated
    } else if !sufficient || !persistence_visible {
        PostEffectVerdict::Insufficient
    } else if after_vs_cold_pair_rate_delta_pct >= 20.0 && after_vs_cold_evidence_last_delta < 0.0 {
        PostEffectVerdict::WeakPositive
    } else {
        PostEffectVerdict::PersistenceOnly
    };

    Ok(PostEffectReport {
        verdict,
        clean_closure,
        persistence_visible,
        cold_pair_framed_byte_per_min: cold.pair_framed_byte_per_min,
        train_pair_framed_byte_per_min: train.pair_framed_byte_per_min,
        after_pair_framed_byte_per_min: after.pair_framed_byte_per_min,
        after_vs_cold_pair_rate_delta_pct,
        after_vs_cold_evidence_last_delta,
        after_vs_cold_egress_pressure_last_delta,
        after_vs_cold_pressure_response_ratio_last_delta,
        after_vs_cold_egress_deviation_last_delta,
        after_vs_cold_pair_similarity_last_delta,
        after_vs_cold_egress_deviation_avg_delta,
        after_vs_cold_pair_similarity_avg_delta,
        after_vs_cold_egress_deviation_late_avg_delta,
        after_vs_cold_pair_similarity_late_avg_delta,
        after_vs_cold_egress_deviation_first_half_delta,
        after_vs_cold_egress_deviation_second_half_delta,
        after_vs_cold_egress_deviation_last_quarter_delta,
        after_vs_cold_pair_similarity_first_half_delta,
        after_vs_cold_pair_similarity_second_half_delta,
        after_vs_cold_pair_similarity_last_quarter_delta,
        after_vs_cold_egress_deviation_quarter_deltas,
        after_vs_cold_pair_similarity_quarter_deltas,
        cold_egress_deviation_quarter_span,
        cold_pair_similarity_quarter_span,
        after_vs_cold_egress_pressure_first_half_delta,
        after_vs_cold_egress_pressure_second_half_delta,
        after_vs_cold_egress_pressure_last_quarter_delta,
        after_vs_cold_evidence_first_half_delta,
        after_vs_cold_evidence_second_half_delta,
        after_vs_cold_evidence_last_quarter_delta,
        after_vs_cold_egress_pressure_quarter_deltas,
        after_vs_cold_evidence_quarter_deltas,
        cold_pair_other_delta: cold.pair_other_delta,
        train_pair_other_delta: train.pair_other_delta,
        after_pair_other_delta: after.pair_other_delta,
        train_learned_event_last: train.learned_event_last,
        after_learned_event_first: after.learned_event_first,
        after_learned_event_last: after.learned_event_last,
    })
}

impl PostEffectReport {
    pub fn lines(&self) -> Vec<String> {
        vec![
            format!("verdict={}", verdict_label(self.verdict)),
            format!("clean_closure={}", self.clean_closure),
            format!("persistence_visible={}", self.persistence_visible),
            format!(
                "cold_pair_framed_byte_per_min={:.3}",
                self.cold_pair_framed_byte_per_min
            ),
            format!(
                "train_pair_framed_byte_per_min={:.3}",
                self.train_pair_framed_byte_per_min
            ),
            format!(
                "after_pair_framed_byte_per_min={:.3}",
                self.after_pair_framed_byte_per_min
            ),
            format!(
                "after_vs_cold_pair_rate_delta_pct={:.3}",
                self.after_vs_cold_pair_rate_delta_pct
            ),
            format!(
                "after_vs_cold_evidence_last_delta={:.4}",
                self.after_vs_cold_evidence_last_delta
            ),
            format!(
                "after_vs_cold_egress_pressure_last_delta={:.4}",
                self.after_vs_cold_egress_pressure_last_delta
            ),
            format!(
                "after_vs_cold_pressure_response_ratio_last_delta={:.4}",
                self.after_vs_cold_pressure_response_ratio_last_delta
            ),
            format!(
                "after_vs_cold_egress_deviation_last_delta={:.4}",
                self.after_vs_cold_egress_deviation_last_delta
            ),
            format!(
                "after_vs_cold_pair_similarity_last_delta={:.4}",
                self.after_vs_cold_pair_similarity_last_delta
            ),
            format!(
                "after_vs_cold_egress_deviation_avg_delta={:.4}",
                self.after_vs_cold_egress_deviation_avg_delta
            ),
            format!(
                "after_vs_cold_pair_similarity_avg_delta={:.4}",
                self.after_vs_cold_pair_similarity_avg_delta
            ),
            format!(
                "after_vs_cold_egress_deviation_late_avg_delta={:.4}",
                self.after_vs_cold_egress_deviation_late_avg_delta
            ),
            format!(
                "after_vs_cold_pair_similarity_late_avg_delta={:.4}",
                self.after_vs_cold_pair_similarity_late_avg_delta
            ),
            format!(
                "after_vs_cold_egress_deviation_first_half_delta={:.4}",
                self.after_vs_cold_egress_deviation_first_half_delta
            ),
            format!(
                "after_vs_cold_egress_deviation_second_half_delta={:.4}",
                self.after_vs_cold_egress_deviation_second_half_delta
            ),
            format!(
                "after_vs_cold_egress_deviation_last_quarter_delta={:.4}",
                self.after_vs_cold_egress_deviation_last_quarter_delta
            ),
            format!(
                "after_vs_cold_pair_similarity_first_half_delta={:.4}",
                self.after_vs_cold_pair_similarity_first_half_delta
            ),
            format!(
                "after_vs_cold_pair_similarity_second_half_delta={:.4}",
                self.after_vs_cold_pair_similarity_second_half_delta
            ),
            format!(
                "after_vs_cold_pair_similarity_last_quarter_delta={:.4}",
                self.after_vs_cold_pair_similarity_last_quarter_delta
            ),
            format!(
                "after_vs_cold_egress_deviation_q1_delta={:.4}",
                self.after_vs_cold_egress_deviation_quarter_deltas[0]
            ),
            format!(
                "after_vs_cold_egress_deviation_q2_delta={:.4}",
                self.after_vs_cold_egress_deviation_quarter_deltas[1]
            ),
            format!(
                "after_vs_cold_egress_deviation_q3_delta={:.4}",
                self.after_vs_cold_egress_deviation_quarter_deltas[2]
            ),
            format!(
                "after_vs_cold_egress_deviation_q4_delta={:.4}",
                self.after_vs_cold_egress_deviation_quarter_deltas[3]
            ),
            format!(
                "after_vs_cold_pair_similarity_q1_delta={:.4}",
                self.after_vs_cold_pair_similarity_quarter_deltas[0]
            ),
            format!(
                "after_vs_cold_pair_similarity_q2_delta={:.4}",
                self.after_vs_cold_pair_similarity_quarter_deltas[1]
            ),
            format!(
                "after_vs_cold_pair_similarity_q3_delta={:.4}",
                self.after_vs_cold_pair_similarity_quarter_deltas[2]
            ),
            format!(
                "after_vs_cold_pair_similarity_q4_delta={:.4}",
                self.after_vs_cold_pair_similarity_quarter_deltas[3]
            ),
            format!(
                "cold_egress_deviation_quarter_span={:.4}",
                self.cold_egress_deviation_quarter_span
            ),
            format!(
                "cold_pair_similarity_quarter_span={:.4}",
                self.cold_pair_similarity_quarter_span
            ),
            format!(
                "after_vs_cold_egress_pressure_first_half_delta={:.4}",
                self.after_vs_cold_egress_pressure_first_half_delta
            ),
            format!(
                "after_vs_cold_egress_pressure_second_half_delta={:.4}",
                self.after_vs_cold_egress_pressure_second_half_delta
            ),
            format!(
                "after_vs_cold_egress_pressure_last_quarter_delta={:.4}",
                self.after_vs_cold_egress_pressure_last_quarter_delta
            ),
            format!(
                "after_vs_cold_evidence_first_half_delta={:.4}",
                self.after_vs_cold_evidence_first_half_delta
            ),
            format!(
                "after_vs_cold_evidence_second_half_delta={:.4}",
                self.after_vs_cold_evidence_second_half_delta
            ),
            format!(
                "after_vs_cold_evidence_last_quarter_delta={:.4}",
                self.after_vs_cold_evidence_last_quarter_delta
            ),
            format!(
                "after_vs_cold_egress_pressure_q1_delta={:.4}",
                self.after_vs_cold_egress_pressure_quarter_deltas[0]
            ),
            format!(
                "after_vs_cold_egress_pressure_q2_delta={:.4}",
                self.after_vs_cold_egress_pressure_quarter_deltas[1]
            ),
            format!(
                "after_vs_cold_egress_pressure_q3_delta={:.4}",
                self.after_vs_cold_egress_pressure_quarter_deltas[2]
            ),
            format!(
                "after_vs_cold_egress_pressure_q4_delta={:.4}",
                self.after_vs_cold_egress_pressure_quarter_deltas[3]
            ),
            format!(
                "after_vs_cold_evidence_q1_delta={:.4}",
                self.after_vs_cold_evidence_quarter_deltas[0]
            ),
            format!(
                "after_vs_cold_evidence_q2_delta={:.4}",
                self.after_vs_cold_evidence_quarter_deltas[1]
            ),
            format!(
                "after_vs_cold_evidence_q3_delta={:.4}",
                self.after_vs_cold_evidence_quarter_deltas[2]
            ),
            format!(
                "after_vs_cold_evidence_q4_delta={:.4}",
                self.after_vs_cold_evidence_quarter_deltas[3]
            ),
            format!("effect_status={}", self.effect_status()),
            format!("boundary_status={}", self.boundary_status()),
            format!("quality_status={}", self.quality_status()),
            format!("late_quality_status={}", self.late_quality_status()),
            format!("late_variance_status={}", self.late_variance_status()),
            format!("pair_rate_status={}", self.pair_rate_status()),
            format!("retention_status={}", self.retention_status()),
            format!("cold_pair_other_delta={}", self.cold_pair_other_delta),
            format!("train_pair_other_delta={}", self.train_pair_other_delta),
            format!("after_pair_other_delta={}", self.after_pair_other_delta),
            format!("train_learned_event_last={}", self.train_learned_event_last),
            format!(
                "after_learned_event_first={}",
                self.after_learned_event_first
            ),
            format!("after_learned_event_last={}", self.after_learned_event_last),
        ]
    }

    fn effect_status(&self) -> &'static str {
        if self.clean_closure
            && self.persistence_visible
            && self.after_vs_cold_egress_deviation_avg_delta < 0.0
            && self.after_vs_cold_pair_similarity_avg_delta > 0.0
            && self.after_vs_cold_pair_rate_delta_pct >= -5.0
        {
            "pass"
        } else {
            "hold"
        }
    }

    fn boundary_status(&self) -> &'static str {
        if self.clean_closure && self.persistence_visible {
            "clean"
        } else {
            "blocked"
        }
    }

    fn quality_status(&self) -> &'static str {
        if self.after_vs_cold_egress_deviation_avg_delta < 0.0
            && self.after_vs_cold_pair_similarity_avg_delta > 0.0
        {
            "avg_improved"
        } else {
            "not_stable"
        }
    }

    fn late_quality_status(&self) -> &'static str {
        if self.after_vs_cold_egress_deviation_late_avg_delta < 0.0
            && self.after_vs_cold_pair_similarity_late_avg_delta > 0.0
        {
            "late_avg_improved"
        } else {
            "late_not_stable"
        }
    }

    fn pair_rate_status(&self) -> &'static str {
        if self.after_vs_cold_pair_rate_delta_pct >= -5.0 {
            "stable"
        } else {
            "dropped"
        }
    }

    pub fn late_variance_status(&self) -> &'static str {
        if !self.clean_closure || !self.persistence_visible {
            return "blocked";
        }
        let q4_deviation_delta = self.after_vs_cold_egress_deviation_quarter_deltas[3];
        let q4_similarity_delta = self.after_vs_cold_pair_similarity_quarter_deltas[3];
        if q4_deviation_delta <= 0.0 && q4_similarity_delta >= 0.0 {
            return "improved";
        }
        if q4_deviation_delta.abs() <= self.cold_egress_deviation_quarter_span
            && q4_similarity_delta.abs() <= self.cold_pair_similarity_quarter_span
        {
            "within_cold_variability"
        } else {
            "outside_cold_variability"
        }
    }

    pub fn retention_status(&self) -> &'static str {
        if self.clean_closure
            && self.persistence_visible
            && self.after_vs_cold_egress_deviation_late_avg_delta < 0.0
            && self.after_vs_cold_pair_similarity_late_avg_delta > 0.0
            && self.after_vs_cold_egress_deviation_avg_delta <= 0.0030
            && self.after_vs_cold_pair_rate_delta_pct >= -5.0
        {
            "pass"
        } else {
            "hold"
        }
    }
}

impl PostEffectRepeatReport {
    pub fn lines(&self) -> Vec<String> {
        vec![
            format!("repeat_runs={}", self.repeat_runs),
            format!("repeat_clean_runs={}", self.repeat_clean_runs),
            format!("repeat_persistent_runs={}", self.repeat_persistent_runs),
            format!(
                "repeat_retention_pass_runs={}",
                self.repeat_retention_pass_runs
            ),
            format!(
                "repeat_late_variance_nonblocking_runs={}",
                self.repeat_late_variance_nonblocking_runs
            ),
            format!(
                "repeat_q4_egress_deviation_delta_avg={:.4}",
                self.repeat_q4_egress_deviation_delta_avg
            ),
            format!(
                "repeat_q4_egress_deviation_delta_min={:.4}",
                self.repeat_q4_egress_deviation_delta_min
            ),
            format!(
                "repeat_q4_egress_deviation_delta_max={:.4}",
                self.repeat_q4_egress_deviation_delta_max
            ),
            format!(
                "repeat_avg_egress_deviation_delta_avg={:.4}",
                self.repeat_avg_egress_deviation_delta_avg
            ),
            format!(
                "repeat_late_egress_deviation_delta_avg={:.4}",
                self.repeat_late_egress_deviation_delta_avg
            ),
            format!("repeat_status={}", self.repeat_status()),
        ]
    }

    pub fn repeat_status(&self) -> &'static str {
        if self.repeat_runs < 3 {
            "needs_more_runs"
        } else if self.repeat_clean_runs == self.repeat_runs
            && self.repeat_persistent_runs == self.repeat_runs
            && self.repeat_retention_pass_runs == self.repeat_runs
        {
            "stable_pass"
        } else if self.repeat_clean_runs == self.repeat_runs
            && self.repeat_persistent_runs == self.repeat_runs
            && self.repeat_late_variance_nonblocking_runs == self.repeat_runs
        {
            "variance_limited"
        } else {
            "unstable"
        }
    }
}

pub fn summarize_repeats(reports: &[PostEffectReport]) -> PostEffectRepeatReport {
    let repeat_runs = reports.len();
    let repeat_clean_runs = reports.iter().filter(|report| report.clean_closure).count();
    let repeat_persistent_runs = reports
        .iter()
        .filter(|report| report.persistence_visible)
        .count();
    let repeat_retention_pass_runs = reports
        .iter()
        .filter(|report| report.retention_status() == "pass")
        .count();
    let repeat_late_variance_nonblocking_runs = reports
        .iter()
        .filter(|report| {
            matches!(
                report.late_variance_status(),
                "improved" | "within_cold_variability"
            )
        })
        .count();
    let q4_deltas = reports
        .iter()
        .map(|report| report.after_vs_cold_egress_deviation_quarter_deltas[3])
        .collect::<Vec<_>>();

    PostEffectRepeatReport {
        repeat_runs,
        repeat_clean_runs,
        repeat_persistent_runs,
        repeat_retention_pass_runs,
        repeat_late_variance_nonblocking_runs,
        repeat_q4_egress_deviation_delta_avg: avg_slice(&q4_deltas),
        repeat_q4_egress_deviation_delta_min: min_slice(&q4_deltas),
        repeat_q4_egress_deviation_delta_max: max_slice(&q4_deltas),
        repeat_avg_egress_deviation_delta_avg: avg_slice(
            &reports
                .iter()
                .map(|report| report.after_vs_cold_egress_deviation_avg_delta)
                .collect::<Vec<_>>(),
        ),
        repeat_late_egress_deviation_delta_avg: avg_slice(
            &reports
                .iter()
                .map(|report| report.after_vs_cold_egress_deviation_late_avg_delta)
                .collect::<Vec<_>>(),
        ),
    }
}

fn read_stage(path: &Path) -> std::io::Result<StageMetrics> {
    let rows = read_rows(path)?;
    let first = rows
        .first()
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    let last = rows
        .last()
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    let duration_min = (last.clock_s - first.clock_s).max(0.0) / 60.0;
    let pair_framed_byte_delta = delta(last.pair_framed_byte_total, first.pair_framed_byte_total);
    let pair_other_delta = delta(last.pair_other_total, first.pair_other_total) as u64;

    Ok(StageMetrics {
        pair_framed_byte_per_min: rate(pair_framed_byte_delta, duration_min),
        pair_other_delta,
        learned_event_first: first.learned_event_total as u64,
        learned_event_last: last.learned_event_total as u64,
        evidence_last: last.evidence_p,
        egress_pressure_last: last.egress_pressure,
        egress_deviation_last: last.egress_deviation,
        pair_similarity_last: last.pair_similarity,
        pressure_response_ratio_last: ratio(last.egress_pressure, last.evidence_p),
        egress_deviation_avg: avg_by(&rows, |row| row.egress_deviation),
        pair_similarity_avg: avg_by(&rows, |row| row.pair_similarity),
        egress_deviation_late_avg: late_avg_by(&rows, |row| row.egress_deviation),
        pair_similarity_late_avg: late_avg_by(&rows, |row| row.pair_similarity),
        egress_deviation_first_half_avg: segment_avg_by(&rows, 0, 2, |row| row.egress_deviation),
        egress_deviation_second_half_avg: segment_avg_by(&rows, 1, 2, |row| row.egress_deviation),
        egress_deviation_last_quarter_avg: segment_avg_by(&rows, 3, 4, |row| row.egress_deviation),
        pair_similarity_first_half_avg: segment_avg_by(&rows, 0, 2, |row| row.pair_similarity),
        pair_similarity_second_half_avg: segment_avg_by(&rows, 1, 2, |row| row.pair_similarity),
        pair_similarity_last_quarter_avg: segment_avg_by(&rows, 3, 4, |row| row.pair_similarity),
        egress_deviation_quarter_avgs: quarter_avgs_by(&rows, |row| row.egress_deviation),
        pair_similarity_quarter_avgs: quarter_avgs_by(&rows, |row| row.pair_similarity),
        egress_pressure_first_half_avg: segment_avg_by(&rows, 0, 2, |row| row.egress_pressure),
        egress_pressure_second_half_avg: segment_avg_by(&rows, 1, 2, |row| row.egress_pressure),
        egress_pressure_last_quarter_avg: segment_avg_by(&rows, 3, 4, |row| row.egress_pressure),
        evidence_first_half_avg: segment_avg_by(&rows, 0, 2, |row| row.evidence_p),
        evidence_second_half_avg: segment_avg_by(&rows, 1, 2, |row| row.evidence_p),
        evidence_last_quarter_avg: segment_avg_by(&rows, 3, 4, |row| row.evidence_p),
        egress_pressure_quarter_avgs: quarter_avgs_by(&rows, |row| row.egress_pressure),
        evidence_quarter_avgs: quarter_avgs_by(&rows, |row| row.evidence_p),
        sufficient: rows.len() >= 2 && duration_min > 0.0,
    })
}

fn read_rows(path: &Path) -> std::io::Result<Vec<Row>> {
    let text = std::fs::read_to_string(path)?;
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let header = lines
        .next()
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    let columns = header
        .split(',')
        .enumerate()
        .map(|(i, name)| (name, i))
        .collect::<HashMap<_, _>>();

    lines
        .map(|line| parse_row(line, &columns))
        .collect::<std::io::Result<Vec<_>>>()
}

fn parse_row(line: &str, columns: &HashMap<&str, usize>) -> std::io::Result<Row> {
    let fields = line.split(',').collect::<Vec<_>>();
    Ok(Row {
        clock_s: read_f64(&fields, columns, "clock_s")?,
        pair_framed_byte_total: read_f64(&fields, columns, "pair_framed_byte_total")?,
        pair_other_total: read_f64(&fields, columns, "pair_other_total")?,
        learned_event_total: read_f64(&fields, columns, "learned_event_total")?,
        evidence_p: read_f64(&fields, columns, "evidence_p")?,
        egress_pressure: read_f64(&fields, columns, "egress_pressure")?,
        egress_deviation: read_f64(&fields, columns, "egress_deviation")?,
        pair_similarity: read_f64(&fields, columns, "pair_similarity")?,
    })
}

fn read_f64(fields: &[&str], columns: &HashMap<&str, usize>, name: &str) -> std::io::Result<f64> {
    let idx = *columns
        .get(name)
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?;
    fields
        .get(idx)
        .ok_or_else(|| Error::from(ErrorKind::InvalidData))?
        .parse::<f64>()
        .map_err(|_| Error::from(ErrorKind::InvalidData))
}

fn delta(last: f64, first: f64) -> f64 {
    (last - first).max(0.0)
}

fn rate(delta: f64, minutes: f64) -> f64 {
    if minutes <= 0.0 {
        0.0
    } else {
        delta / minutes
    }
}

fn pct_delta(after: f64, cold: f64) -> f64 {
    if cold <= 0.0 {
        0.0
    } else {
        ((after - cold) / cold) * 100.0
    }
}

fn ratio(numerator: f64, denominator: f64) -> f64 {
    if denominator.abs() <= f64::EPSILON {
        0.0
    } else {
        numerator / denominator
    }
}

fn avg_by(rows: &[Row], f: impl Fn(&Row) -> f64) -> f64 {
    if rows.is_empty() {
        0.0
    } else {
        rows.iter().map(f).sum::<f64>() / rows.len() as f64
    }
}

fn late_avg_by(rows: &[Row], f: impl Fn(&Row) -> f64) -> f64 {
    if rows.is_empty() {
        return 0.0;
    }
    let start = rows.len() / 2;
    let late = &rows[start..];
    late.iter().map(f).sum::<f64>() / late.len() as f64
}

fn segment_avg_by(
    rows: &[Row],
    segment_idx: usize,
    segment_count: usize,
    f: impl Fn(&Row) -> f64,
) -> f64 {
    if rows.is_empty() || segment_count == 0 || segment_idx >= segment_count {
        return 0.0;
    }
    let start = rows.len() * segment_idx / segment_count;
    let mut end = rows.len() * (segment_idx + 1) / segment_count;
    if segment_idx + 1 == segment_count {
        end = rows.len();
    }
    let segment = &rows[start..end.max(start + 1).min(rows.len())];
    segment.iter().map(f).sum::<f64>() / segment.len() as f64
}

fn quarter_avgs_by(rows: &[Row], f: impl Fn(&Row) -> f64 + Copy) -> [f64; 4] {
    [
        segment_avg_by(rows, 0, 4, f),
        segment_avg_by(rows, 1, 4, f),
        segment_avg_by(rows, 2, 4, f),
        segment_avg_by(rows, 3, 4, f),
    ]
}

fn quarter_deltas(after: [f64; 4], cold: [f64; 4]) -> [f64; 4] {
    [
        after[0] - cold[0],
        after[1] - cold[1],
        after[2] - cold[2],
        after[3] - cold[3],
    ]
}

fn span(values: [f64; 4]) -> f64 {
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if min.is_finite() && max.is_finite() {
        max - min
    } else {
        0.0
    }
}

fn avg_slice(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().sum::<f64>() / values.len() as f64
    }
}

fn min_slice(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().copied().fold(f64::INFINITY, f64::min)
    }
}

fn max_slice(values: &[f64]) -> f64 {
    if values.is_empty() {
        0.0
    } else {
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }
}

fn verdict_label(verdict: PostEffectVerdict) -> &'static str {
    match verdict {
        PostEffectVerdict::Contaminated => "contaminated",
        PostEffectVerdict::Insufficient => "insufficient",
        PostEffectVerdict::PersistenceOnly => "persistence_only",
        PostEffectVerdict::WeakPositive => "weak_positive",
    }
}
