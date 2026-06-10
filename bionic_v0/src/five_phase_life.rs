use crate::substrate::unit;
use crate::live_human_nursery::summarize_live_human_nursery;
use crate::snapshot::SnapshotData;
use crate::WRITE_EPOCH_HISTORY_CAP;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FivePhaseLifeEvidence {
    pub deterministic_fixture: u8,
    pub runtime_snapshot_read: u8,
    pub snapshots_seen: u32,
    pub wood_balance: f64,
    pub fire_balance: f64,
    pub earth_balance: f64,
    pub metal_balance: f64,
    pub water_balance: f64,
    pub wood_insufficient: u8,
    pub fire_insufficient: u8,
    pub earth_insufficient: u8,
    pub metal_insufficient: u8,
    pub water_insufficient: u8,
    pub wood_excess: u8,
    pub fire_excess: u8,
    pub earth_excess: u8,
    pub metal_excess: u8,
    pub water_excess: u8,
    pub fire_live_closure_gate_pass: u8,
    pub wood_fire_non_suppression: u8,
    pub principle_guard_pass: u8,
}

pub fn summarize_five_phase_life<'a>(
    snapshots: impl IntoIterator<Item = &'a SnapshotData>,
    deterministic_fixture: u8,
    runtime_snapshot_read: u8,
) -> FivePhaseLifeEvidence {
    let snapshots: Vec<&SnapshotData> = snapshots.into_iter().collect();
    let live = summarize_live_human_nursery(snapshots.iter().copied());
    let mut evidence = FivePhaseLifeEvidence {
        deterministic_fixture,
        runtime_snapshot_read,
        snapshots_seen: live.snapshots_seen,
        ..Default::default()
    };

    for snap in snapshots {
        evidence.wood_balance = evidence.wood_balance.max(wood_score(snap));
        evidence.fire_balance = evidence.fire_balance.max(fire_score(snap));
        evidence.earth_balance = evidence.earth_balance.max(earth_score(snap));
        evidence.metal_balance = evidence.metal_balance.max(metal_score(snap));
        evidence.water_balance = evidence.water_balance.max(water_score(snap));
    }

    evidence.wood_insufficient = low(evidence.wood_balance);
    evidence.fire_insufficient = low(evidence.fire_balance);
    evidence.earth_insufficient = low(evidence.earth_balance);
    evidence.metal_insufficient = low(evidence.metal_balance);
    evidence.water_insufficient = low(evidence.water_balance);
    evidence.wood_excess = excess(evidence.wood_balance);
    evidence.fire_excess = excess(evidence.fire_balance);
    evidence.earth_excess = excess(evidence.earth_balance);
    evidence.metal_excess = excess(evidence.metal_balance);
    evidence.water_excess = excess(evidence.water_balance);

    evidence.fire_live_closure_gate_pass = u8::from(
        runtime_snapshot_read == 1
            && deterministic_fixture == 0
            && live.body_write_out_observed == 1
            && live.vocal_write_epochs > 0
            && live.return_windows > 0
            && live.temporal_pairing_observed == 1
            && live.closure_or_reentry_observed == 1,
    );
    evidence.wood_fire_non_suppression = u8::from(
        evidence.metal_balance < 0.92 || evidence.wood_balance.max(evidence.fire_balance) >= 0.20,
    );
    evidence.principle_guard_pass = u8::from(all_bounded(&evidence));
    evidence
}

fn wood_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.active_reasoning_gap
            .gap_pressure
            .max(snap.probe_law.intensity)
            .max(snap.global_reasoning_memory.novelty_score)
            .max(snap.self_development.exploratory_pressure),
    )
}

fn fire_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.vocal_target
            .active_energy
            .max(snap.vocal_organ.breath_drive)
            .max(ratio(
                snap.action_output.n_write_epochs,
                WRITE_EPOCH_HISTORY_CAP,
            ))
            .max(snap.framed_byte_egress.output_pressure)
            .max(snap.probe_law.output_intensity),
    )
}

fn earth_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.active_reasoning_gap
            .closure_score
            .max(snap.self_development.familiarity_pressure)
            .max(snap.self_development.stable_recurrence)
            .max(snap.global_reasoning_memory.routing_stability_score),
    )
}

fn metal_score(snap: &SnapshotData) -> f64 {
    unit(snap.probe_law.suppression)
}

fn water_score(snap: &SnapshotData) -> f64 {
    unit(
        snap.active_reasoning_gap
            .reentry_score
            .max(snap.self_development.solitary_reentry_score)
            .max(snap.self_development.quiet_replay_pressure)
            .max(snap.global_reasoning_memory.persistence_score),
    )
}

fn low(value: f64) -> u8 {
    u8::from(value < 0.05)
}

fn excess(value: f64) -> u8 {
    u8::from(value > 0.92)
}

fn ratio(value: u32, cap: usize) -> f64 {
    if cap == 0 {
        0.0
    } else {
        unit(value as f64 / cap as f64)
    }
}

fn all_bounded(evidence: &FivePhaseLifeEvidence) -> bool {
    [
        evidence.wood_balance,
        evidence.fire_balance,
        evidence.earth_balance,
        evidence.metal_balance,
        evidence.water_balance,
    ]
    .into_iter()
    .all(|value| value.is_finite() && (0.0..=1.0).contains(&value))
}

