use crate::substrate::unit;
use crate::snapshot::SnapshotData;
use crate::{CONSEQUENCE_WINDOW_CAP, PAIR_HISTORY_CAP, WRITE_EPOCH_HISTORY_CAP};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AllostaticStabilityEvidence {
    pub deterministic_fixture: u8,
    pub runtime_snapshot_read: u8,
    pub snapshots_seen: u32,
    pub arousal_load: f64,
    pub reserve_level: f64,
    pub recovery_debt: f64,
    pub bearing_capacity: f64,
    pub exploration_allowance: f64,
    pub expression_gain: f64,
    pub protective_limit: f64,
    pub overdrive_window: f64,
    pub downshift_pressure: f64,
    pub principle_guard_pass: u8,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
struct AllostaticSample {
    arousal_load: f64,
    reserve_level: f64,
    recovery_debt: f64,
    bearing_capacity: f64,
    exploration_allowance: f64,
    expression_gain: f64,
    protective_limit: f64,
    overdrive_window: f64,
    downshift_pressure: f64,
}

pub fn summarize_allostatic_stability<'a>(
    snapshots: impl IntoIterator<Item = &'a SnapshotData>,
    deterministic_fixture: u8,
    runtime_snapshot_read: u8,
) -> AllostaticStabilityEvidence {
    let mut evidence = AllostaticStabilityEvidence {
        deterministic_fixture,
        runtime_snapshot_read,
        ..Default::default()
    };

    for snap in snapshots {
        evidence.snapshots_seen = evidence.snapshots_seen.saturating_add(1);
        let sample = sample_snapshot(snap);
        evidence.arousal_load = evidence.arousal_load.max(sample.arousal_load);
        evidence.reserve_level = evidence.reserve_level.max(sample.reserve_level);
        evidence.recovery_debt = evidence.recovery_debt.max(sample.recovery_debt);
        evidence.bearing_capacity = evidence.bearing_capacity.max(sample.bearing_capacity);
        evidence.exploration_allowance = evidence
            .exploration_allowance
            .max(sample.exploration_allowance);
        evidence.expression_gain = evidence.expression_gain.max(sample.expression_gain);
        evidence.protective_limit = evidence.protective_limit.max(sample.protective_limit);
        evidence.overdrive_window = evidence.overdrive_window.max(sample.overdrive_window);
        evidence.downshift_pressure = evidence.downshift_pressure.max(sample.downshift_pressure);
    }

    evidence.principle_guard_pass = u8::from(evidence.snapshots_seen > 0 && all_bounded(&evidence));
    evidence
}

fn sample_snapshot(snap: &SnapshotData) -> AllostaticSample {
    let demand = demand(snap);
    let support = support(snap);
    let stress_load = unit(demand * (1.0 - support));
    let loop_load = load_ratio(snap);
    let correction_load = unit(snap.framed_byte_egress.correction_deviation * loop_load.max(0.25));
    let recovery_debt = unit(
        snap.self_development
            .repetition_pressure
            .max(correction_load)
            .max(stress_load * demand),
    );
    let reserve_level = unit(1.0 - recovery_debt.max(stress_load * 0.70));
    let bearing_capacity = support;
    let protective_limit = unit(
        snap.probe_law
            .suppression
            .max(recovery_debt * 0.72 + stress_load * 0.28),
    );
    let expression_gain = unit(
        demand * reserve_level * (0.35 + 0.65 * support)
            + snap.probe_law.output_intensity * reserve_level * (1.0 - protective_limit * 0.25),
    );
    let exploration_seed = snap
        .self_development
        .exploratory_pressure
        .max(snap.active_reasoning_gap.gap_pressure)
        .max(snap.global_reasoning_memory.novelty_score);
    let exploration_allowance =
        unit(exploration_seed * reserve_level * (1.0 - protective_limit * 0.50));
    let overdrive_window =
        if demand >= 0.45 && support >= 0.35 && reserve_level >= 0.35 && protective_limit < 0.75 {
            unit(demand * support * reserve_level * (1.0 - protective_limit * 0.50))
        } else {
            0.0
        };
    let downshift_pressure = unit(
        recovery_debt.max(stress_load) * (0.50 + 0.50 * protective_limit) * (1.0 - support * 0.35),
    );

    AllostaticSample {
        arousal_load: demand,
        reserve_level,
        recovery_debt,
        bearing_capacity,
        exploration_allowance,
        expression_gain,
        protective_limit,
        overdrive_window,
        downshift_pressure,
    }
}

fn demand(snap: &SnapshotData) -> f64 {
    unit(
        snap.evidence_tension
            .unresolved_pressure
            .max(snap.active_reasoning_gap.gap_pressure)
            .max(snap.global_reasoning.global_surprise_load)
            .max(relation_surprise(snap))
            .max(snap.framed_byte_egress.correction_deviation)
            .max(snap.probe_law.intensity)
            .max(snap.probe_law.output_intensity)
            .max(snap.framed_byte_egress.output_pressure),
    )
}

fn support(snap: &SnapshotData) -> f64 {
    unit(
        snap.active_reasoning_gap
            .closure_score
            .max(latest_pair_similarity(snap))
            .max(snap.self_development.familiarity_pressure)
            .max(snap.self_development.stable_recurrence)
            .max(snap.self_development.room_familiarity)
            .max(snap.global_reasoning_memory.routing_stability_score)
            .max(snap.global_reasoning_memory.pattern_recurrence_score),
    )
}

fn relation_surprise(snap: &SnapshotData) -> f64 {
    snap.global_relations
        .edges
        .iter()
        .filter(|edge| edge.active)
        .map(|edge| unit(edge.pred.relation_surprise))
        .fold(0.0, f64::max)
}

fn latest_pair_similarity(snap: &SnapshotData) -> f64 {
    snap.action_consequence
        .pairs
        .iter()
        .rev()
        .take(8)
        .map(|pair| unit(pair.curve_similarity))
        .fold(0.0, f64::max)
}

fn load_ratio(snap: &SnapshotData) -> f64 {
    unit(
        ratio(snap.action_output.n_write_epochs, WRITE_EPOCH_HISTORY_CAP)
            .max(ratio(
                snap.action_consequence.n_windows,
                CONSEQUENCE_WINDOW_CAP,
            ))
            .max(ratio(snap.action_consequence.n_pairs, PAIR_HISTORY_CAP))
            .max(ratio(
                snap.framed_byte_egress.n_frames,
                WRITE_EPOCH_HISTORY_CAP,
            ))
            .max(ratio(
                snap.framed_byte_egress.n_returns,
                CONSEQUENCE_WINDOW_CAP,
            )),
    )
}

fn ratio(value: u32, cap: usize) -> f64 {
    if cap == 0 {
        0.0
    } else {
        unit(value as f64 / cap as f64)
    }
}

fn all_bounded(evidence: &AllostaticStabilityEvidence) -> bool {
    [
        evidence.arousal_load,
        evidence.reserve_level,
        evidence.recovery_debt,
        evidence.bearing_capacity,
        evidence.exploration_allowance,
        evidence.expression_gain,
        evidence.protective_limit,
        evidence.overdrive_window,
        evidence.downshift_pressure,
    ]
    .into_iter()
    .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
}

