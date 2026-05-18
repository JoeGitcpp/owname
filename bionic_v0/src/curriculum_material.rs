use std::collections::BTreeSet;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MaterialOriginKind {
    Synthetic = 1,
    Replay = 2,
    ProbeReturn = 3,
    LiveIngress = 4,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum MaterialCarrierKind {
    MarkBytes = 1,
    VibCurve = 2,
    PhotCurve = 3,
    FramedBytes = 4,
    SnapshotTrace = 5,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum CurriculumStageKind {
    Hold = 1,
    Variation = 2,
    Conflict = 3,
    Correction = 4,
    Reentry = 5,
    Coupling = 6,
    ActiveProbe = 7,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurriculumMaterialEntry {
    pub material_id: u64,
    pub origin_kind: MaterialOriginKind,
    pub carrier_kind: MaterialCarrierKind,
    pub stage_kind: CurriculumStageKind,
    pub stream_context_id: u64,
    pub seed: u64,
    pub event_count: u32,
    pub duration_us: u64,
    pub reproducible: bool,
    pub observer_only: bool,
    pub body_touch_points: u32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExternalCurriculumTrainingPlan {
    pub entries: Vec<CurriculumMaterialEntry>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ExternalCurriculumTrainingPlanReview {
    pub material_groups: u32,
    pub synthetic_groups: u32,
    pub replay_groups: u32,
    pub probe_return_groups: u32,
    pub live_ingress_groups: u32,
    pub carrier_kinds: u32,
    pub stage_kinds: u32,
    pub reproducible_groups: u32,
    pub observer_only_groups: u32,
    pub body_touch_points: u32,
    pub missing_required_roles: u32,
    pub principle_review_pass: u8,
}

pub fn default_external_curriculum_training_plan() -> ExternalCurriculumTrainingPlan {
    use CurriculumStageKind::*;
    use MaterialCarrierKind::*;
    use MaterialOriginKind::*;

    ExternalCurriculumTrainingPlan {
        entries: vec![
            entry(
                101, Synthetic, MarkBytes, Variation, 61, 0x1001, 24, 480_000, true,
            ),
            entry(
                102, Synthetic, VibCurve, Hold, 62, 0x1002, 16, 320_000, true,
            ),
            entry(
                103, Synthetic, PhotCurve, Hold, 63, 0x1003, 16, 320_000, true,
            ),
            entry(
                104, Synthetic, MarkBytes, Coupling, 64, 0x1004, 32, 640_000, true,
            ),
            entry(
                201,
                Replay,
                SnapshotTrace,
                Reentry,
                71,
                0x2001,
                8,
                2_200_000,
                true,
            ),
            entry(
                202,
                Replay,
                SnapshotTrace,
                Conflict,
                72,
                0x2002,
                8,
                2_200_000,
                true,
            ),
            entry(
                301,
                ProbeReturn,
                FramedBytes,
                ActiveProbe,
                81,
                0x3001,
                6,
                360_000,
                true,
            ),
            entry(
                401,
                LiveIngress,
                MarkBytes,
                Conflict,
                91,
                0x4001,
                0,
                0,
                false,
            ),
            entry(
                402,
                LiveIngress,
                VibCurve,
                Correction,
                92,
                0x4002,
                0,
                0,
                false,
            ),
            entry(
                403,
                LiveIngress,
                PhotCurve,
                Reentry,
                93,
                0x4003,
                0,
                0,
                false,
            ),
        ],
    }
}

pub fn review_external_curriculum_training_plan(
    plan: &ExternalCurriculumTrainingPlan,
) -> ExternalCurriculumTrainingPlanReview {
    let mut origin_kinds = BTreeSet::new();
    let mut carrier_kinds = BTreeSet::new();
    let mut stage_kinds = BTreeSet::new();
    let mut review = ExternalCurriculumTrainingPlanReview {
        material_groups: plan.entries.len() as u32,
        ..ExternalCurriculumTrainingPlanReview::default()
    };

    for entry in &plan.entries {
        origin_kinds.insert(entry.origin_kind);
        carrier_kinds.insert(entry.carrier_kind);
        stage_kinds.insert(entry.stage_kind);
        match entry.origin_kind {
            MaterialOriginKind::Synthetic => review.synthetic_groups += 1,
            MaterialOriginKind::Replay => review.replay_groups += 1,
            MaterialOriginKind::ProbeReturn => review.probe_return_groups += 1,
            MaterialOriginKind::LiveIngress => review.live_ingress_groups += 1,
        }
        if entry.reproducible {
            review.reproducible_groups += 1;
        }
        if entry.observer_only {
            review.observer_only_groups += 1;
        }
        review.body_touch_points = review
            .body_touch_points
            .saturating_add(entry.body_touch_points);
    }

    review.carrier_kinds = carrier_kinds.len() as u32;
    review.stage_kinds = stage_kinds.len() as u32;
    review.missing_required_roles = missing_required_roles(
        &origin_kinds,
        &carrier_kinds,
        &stage_kinds,
        review.body_touch_points,
        review.observer_only_groups,
        review.material_groups,
    );
    review.principle_review_pass = if review.missing_required_roles == 0 {
        1
    } else {
        0
    };
    review
}

fn entry(
    material_id: u64,
    origin_kind: MaterialOriginKind,
    carrier_kind: MaterialCarrierKind,
    stage_kind: CurriculumStageKind,
    stream_context_id: u64,
    seed: u64,
    event_count: u32,
    duration_us: u64,
    reproducible: bool,
) -> CurriculumMaterialEntry {
    CurriculumMaterialEntry {
        material_id,
        origin_kind,
        carrier_kind,
        stage_kind,
        stream_context_id,
        seed,
        event_count,
        duration_us,
        reproducible,
        observer_only: true,
        body_touch_points: 0,
    }
}

fn missing_required_roles(
    origin_kinds: &BTreeSet<MaterialOriginKind>,
    carrier_kinds: &BTreeSet<MaterialCarrierKind>,
    stage_kinds: &BTreeSet<CurriculumStageKind>,
    body_touch_points: u32,
    observer_only_groups: u32,
    material_groups: u32,
) -> u32 {
    let mut missing = 0u32;
    for required in [
        MaterialOriginKind::Synthetic,
        MaterialOriginKind::Replay,
        MaterialOriginKind::ProbeReturn,
        MaterialOriginKind::LiveIngress,
    ] {
        if !origin_kinds.contains(&required) {
            missing += 1;
        }
    }
    for required in [
        MaterialCarrierKind::MarkBytes,
        MaterialCarrierKind::VibCurve,
        MaterialCarrierKind::PhotCurve,
        MaterialCarrierKind::FramedBytes,
        MaterialCarrierKind::SnapshotTrace,
    ] {
        if !carrier_kinds.contains(&required) {
            missing += 1;
        }
    }
    for required in [
        CurriculumStageKind::Hold,
        CurriculumStageKind::Variation,
        CurriculumStageKind::Conflict,
        CurriculumStageKind::Correction,
        CurriculumStageKind::Reentry,
        CurriculumStageKind::Coupling,
        CurriculumStageKind::ActiveProbe,
    ] {
        if !stage_kinds.contains(&required) {
            missing += 1;
        }
    }
    if body_touch_points != 0 {
        missing += 1;
    }
    if observer_only_groups != material_groups {
        missing += 1;
    }
    missing
}
