use std::sync::Arc;

use crate::action_output::{ActionOutputEvent, ActionOutputFeedback, OutputChannelKind};
use crate::snapshot::{write_snapshot, SnapshotPaths};
use crate::viewport::{GlobalRelationState, RelationEdgeState, Viewport};
use crate::{MOD_PHOT, MOD_VIB, PHOT_BANDS, VIB_BANDS};

#[derive(Clone, Copy, Debug)]
pub struct CurriculumConfig {
    pub start_t_us: u64,
    pub step_us: u64,
    pub steps: u32,
    pub magnitude: f64,
}

impl Default for CurriculumConfig {
    fn default() -> Self {
        Self {
            start_t_us: 1_000_000,
            step_us: 20_000,
            steps: 12,
            magnitude: 0.4,
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CurriculumSummary {
    pub steps_run: u32,
    pub worldline_updates: u32,
    pub final_worldline_err: f64,
    pub initial_relation_distance_score: f64,
    pub final_relation_distance_score: f64,
    pub initial_relation_overlap_score: f64,
    pub final_relation_overlap_score: f64,
    pub pairs_formed: u32,
    pub learned_events: u32,
    pub learned_max_abs_delta: f64,
    pub snapshots_written: u32,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CurriculumSuiteSummary {
    pub courses_run: u32,
    pub total_steps_run: u32,
    pub worldline_updates: u32,
    pub final_worldline_err: f64,
    pub initial_relation_distance_score: f64,
    pub final_relation_distance_score: f64,
    pub initial_relation_overlap_score: f64,
    pub final_relation_overlap_score: f64,
    pub pairs_formed: u32,
    pub learned_events: u32,
    pub learned_max_abs_delta: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct CurriculumEpisodeTrend {
    pub episodes_run: u32,
    pub courses_run: u32,
    pub total_steps_run: u32,
    pub first_worldline_err: f64,
    pub last_worldline_err: f64,
    pub mean_worldline_err: f64,
    pub total_pairs_formed: u32,
    pub total_learned_events: u32,
    pub max_learned_abs_delta: f64,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PersistentCurriculumSessionSummary {
    pub episodes_run: u32,
    pub courses_run: u32,
    pub total_steps_run: u32,
    pub worldline_updates: u32,
    pub first_worldline_err: f64,
    pub last_worldline_err: f64,
    pub mean_worldline_err: f64,
    pub total_pairs_formed: u32,
    pub final_pairs: u32,
    pub final_learned_events: u32,
    pub max_learned_abs_delta: f64,
}

pub fn run_persistent_curriculum_session(
    config: CurriculumConfig,
    episodes: u32,
) -> std::io::Result<PersistentCurriculumSessionSummary> {
    let episodes = episodes.max(1);
    let vp = Arc::new(Viewport::new());
    let motion_slot = vp
        .allocate_slot(MOD_PHOT, config.start_t_us, 1)
        .expect("motion slot");
    let relation_a = vp
        .allocate_slot(MOD_PHOT, config.start_t_us, 2)
        .expect("relation slot a");
    let relation_b = vp
        .allocate_slot(MOD_PHOT, config.start_t_us, 3)
        .expect("relation slot b");
    let episode_stride_us =
        config.step_us.max(1) * config.steps.max(2) as u64 + config.step_us.max(1) * 4;

    let mut summary = PersistentCurriculumSessionSummary {
        episodes_run: episodes,
        courses_run: episodes * 3,
        ..PersistentCurriculumSessionSummary::default()
    };
    let mut err_sum = 0.0;

    for episode in 0..episodes {
        let start_t_us = config.start_t_us + episode as u64 * episode_stride_us;
        let episode_config = CurriculumConfig {
            start_t_us,
            ..config
        };
        let motion = run_motion_on_slot(&vp, motion_slot, episode_config);
        let relation = run_relation_on_slots(&vp, relation_a, relation_b, episode_config);
        let action = run_action_pulse_on_viewport(&vp, episode_config);

        if episode == 0 {
            summary.first_worldline_err = motion.final_worldline_err;
        }
        summary.last_worldline_err = motion.final_worldline_err;
        summary.worldline_updates = motion.worldline_updates;
        summary.total_steps_run += motion.steps_run + relation.steps_run + action.steps_run;
        summary.total_pairs_formed += action.pairs_formed;
        err_sum += motion.final_worldline_err;
    }

    let learned = vp.read_learned_bias().summary();
    summary.mean_worldline_err = err_sum / episodes as f64;
    summary.final_pairs = vp.read_action_consequence().pairs.len() as u32;
    summary.final_learned_events = learned.n_recent_events;
    summary.max_learned_abs_delta = learned.max_abs_delta;
    Ok(summary)
}

pub fn run_curriculum_episodes(
    config: CurriculumConfig,
    episodes: u32,
) -> std::io::Result<CurriculumEpisodeTrend> {
    let episodes = episodes.max(1);
    let episode_stride_us =
        config.step_us.max(1) * config.steps.max(2) as u64 + config.step_us.max(1);
    let mut trend = CurriculumEpisodeTrend {
        episodes_run: episodes,
        ..CurriculumEpisodeTrend::default()
    };
    let mut err_sum = 0.0;

    for episode in 0..episodes {
        let episode_config = CurriculumConfig {
            start_t_us: config.start_t_us + episode as u64 * episode_stride_us,
            ..config
        };
        let summary = run_curriculum_suite(episode_config)?;
        if episode == 0 {
            trend.first_worldline_err = summary.final_worldline_err;
        }
        trend.last_worldline_err = summary.final_worldline_err;
        trend.courses_run += summary.courses_run;
        trend.total_steps_run += summary.total_steps_run;
        trend.total_pairs_formed += summary.pairs_formed;
        trend.total_learned_events += summary.learned_events;
        trend.max_learned_abs_delta = trend
            .max_learned_abs_delta
            .max(summary.learned_max_abs_delta);
        err_sum += summary.final_worldline_err;
    }

    trend.mean_worldline_err = err_sum / episodes as f64;
    Ok(trend)
}

pub fn run_curriculum_suite(config: CurriculumConfig) -> std::io::Result<CurriculumSuiteSummary> {
    let motion_vp = Arc::new(Viewport::new());
    let relation_vp = Arc::new(Viewport::new());
    let action_vp = Arc::new(Viewport::new());

    let motion = run_single_object_motion_course(&motion_vp, config, None)?;
    let relation = run_two_object_relation_course(&relation_vp, config, None)?;
    let action = run_action_consequence_pulse_course(&action_vp, config, None)?;

    Ok(CurriculumSuiteSummary {
        courses_run: 3,
        total_steps_run: motion.steps_run + relation.steps_run + action.steps_run,
        worldline_updates: motion.worldline_updates,
        final_worldline_err: motion.final_worldline_err,
        initial_relation_distance_score: relation.initial_relation_distance_score,
        final_relation_distance_score: relation.final_relation_distance_score,
        initial_relation_overlap_score: relation.initial_relation_overlap_score,
        final_relation_overlap_score: relation.final_relation_overlap_score,
        pairs_formed: action.pairs_formed,
        learned_events: action.learned_events,
        learned_max_abs_delta: action.learned_max_abs_delta,
    })
}

pub fn run_single_object_motion_course(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    snapshot_paths: Option<&SnapshotPaths>,
) -> std::io::Result<CurriculumSummary> {
    let slot = vp
        .allocate_slot(MOD_PHOT, config.start_t_us, 1)
        .expect("curriculum slot");
    let mut summary = run_motion_on_slot(vp, slot, config);
    summary.snapshots_written = write_optional_snapshot(vp, snapshot_paths)?;
    Ok(summary)
}

fn run_motion_on_slot(
    vp: &Arc<Viewport>,
    slot: usize,
    config: CurriculumConfig,
) -> CurriculumSummary {
    let steps = config.steps.max(2);
    for step in 0..steps {
        let t_us = config.start_t_us + step as u64 * config.step_us.max(1);
        let fraction = step as f64 / (steps - 1) as f64;
        vp.write_slot_spatial(
            slot,
            MOD_PHOT,
            t_us,
            0.10 + 0.50 * fraction,
            0.20 + 0.20 * fraction,
            0.12,
            0.12,
            0.8,
        );
        seed_attended_slot(vp, slot, t_us);
        vp.refresh_global_attention(t_us);
        vp.refresh_global_relations(t_us);
    }

    let slot_snapshot = vp.read_slot(slot).expect("slot snapshot");
    let worldline = slot_snapshot.spatial_worldline.unwrap_or_default();

    CurriculumSummary {
        steps_run: steps,
        worldline_updates: worldline.n_updates,
        final_worldline_err: worldline.err_norm,
        ..CurriculumSummary::default()
    }
}

pub fn run_two_object_relation_course(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    snapshot_paths: Option<&SnapshotPaths>,
) -> std::io::Result<CurriculumSummary> {
    let start = config.start_t_us;
    let a = vp.allocate_slot(MOD_PHOT, start, 1).expect("slot a");
    let b = vp.allocate_slot(MOD_PHOT, start, 2).expect("slot b");
    let mut summary = run_relation_on_slots(vp, a, b, config);
    summary.snapshots_written = write_optional_snapshot(vp, snapshot_paths)?;
    Ok(summary)
}

fn run_relation_on_slots(
    vp: &Arc<Viewport>,
    a: usize,
    b: usize,
    config: CurriculumConfig,
) -> CurriculumSummary {
    let start = config.start_t_us;
    let later = start + config.step_us.max(1);
    seed_attended_slot(vp, a, start);
    seed_attended_slot(vp, b, start);
    vp.write_slot_spatial(a, MOD_PHOT, start, 0.12, 0.12, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(b, MOD_PHOT, start, 0.84, 0.84, 0.10, 0.10, 0.9);
    vp.refresh_global_attention(start);
    vp.refresh_global_relations(start);
    let initial = relation_scores(&vp.read_global_relations(), a, b);

    seed_attended_slot(vp, a, later);
    seed_attended_slot(vp, b, later);
    vp.write_slot_spatial(a, MOD_PHOT, later, 0.12, 0.12, 0.20, 0.20, 0.9);
    vp.write_slot_spatial(b, MOD_PHOT, later, 0.18, 0.18, 0.16, 0.16, 0.9);
    vp.refresh_global_attention(later);
    vp.refresh_global_relations(later);
    let final_scores = relation_scores(&vp.read_global_relations(), a, b);

    CurriculumSummary {
        steps_run: 2,
        initial_relation_distance_score: initial.0,
        final_relation_distance_score: final_scores.0,
        initial_relation_overlap_score: initial.1,
        final_relation_overlap_score: final_scores.1,
        ..CurriculumSummary::default()
    }
}

pub fn run_action_consequence_pulse_course(
    vp: &Arc<Viewport>,
    config: CurriculumConfig,
    snapshot_paths: Option<&SnapshotPaths>,
) -> std::io::Result<CurriculumSummary> {
    let summary = run_action_pulse_on_viewport(vp, config);
    let snapshots_written = write_optional_snapshot(vp, snapshot_paths)?;
    Ok(CurriculumSummary {
        snapshots_written,
        ..summary
    })
}

fn run_action_pulse_on_viewport(vp: &Arc<Viewport>, config: CurriculumConfig) -> CurriculumSummary {
    let magnitude = config.magnitude.clamp(0.0, 1.0);
    let write_t = config.start_t_us;
    let return_t = write_t + config.step_us.max(1);
    let before_pairs = vp.read_action_consequence().pairs.len();

    vp.apply_action_output_event(ActionOutputEvent {
        channel_kind: OutputChannelKind::GenericWrite,
        feedback: ActionOutputFeedback::Begin,
        magnitude,
        event_t_us: write_t,
    });
    vp.accumulate_ingress_change(return_t, magnitude);
    vp.try_auto_pair_latest_epoch_to_window(return_t);
    if let Some(window) = vp.read_action_consequence().windows.last().cloned() {
        vp.close_action_consequence_window(window.window_id, return_t + 500);
    }
    vp.tick_life_closure_producers(return_t + 1_000);

    let action_consequence = vp.read_action_consequence();
    let learned = vp.read_learned_bias().summary();

    CurriculumSummary {
        steps_run: 1,
        pairs_formed: action_consequence.pairs.len().saturating_sub(before_pairs) as u32,
        learned_events: learned.n_recent_events,
        learned_max_abs_delta: learned.max_abs_delta,
        ..CurriculumSummary::default()
    }
}

fn seed_attended_slot(vp: &Viewport, slot_id: usize, t_us: u64) {
    let vib_obs = [0.1f64; VIB_BANDS];
    let vib_pred = [0.09f64; VIB_BANDS];
    let vib_err = [0.01f64; VIB_BANDS];
    let vib_surprise = [0.5f64; VIB_BANDS];
    vp.write_slot_vib(slot_id, t_us, &vib_obs, &vib_pred, &vib_err, &vib_surprise);
    vp.refresh_slot_fusion(slot_id, MOD_VIB, t_us);

    let phot_obs = vec![0.2f64; PHOT_BANDS];
    let phot_pred = vec![0.18f64; PHOT_BANDS];
    let phot_err = vec![0.02f64; PHOT_BANDS];
    let phot_surprise = vec![0.4f64; PHOT_BANDS];
    vp.write_slot_phot(
        slot_id,
        t_us,
        &phot_obs,
        &phot_pred,
        &phot_err,
        &phot_surprise,
    );
    vp.refresh_slot_fusion(slot_id, MOD_PHOT, t_us);
    vp.refresh_slot_high_layer(slot_id, t_us);
}

fn relation_scores(relations: &GlobalRelationState, src: usize, dst: usize) -> (f64, f64) {
    relation_edge(relations, src, dst)
        .map(|edge| (edge.spatial_distance_score, edge.spatial_overlap_score))
        .unwrap_or((0.0, 0.0))
}

fn relation_edge(
    relations: &GlobalRelationState,
    src: usize,
    dst: usize,
) -> Option<&RelationEdgeState> {
    relations
        .edges
        .iter()
        .find(|edge| edge.src_slot_id as usize == src && edge.dst_slot_id as usize == dst)
}

fn write_optional_snapshot(
    vp: &Arc<Viewport>,
    snapshot_paths: Option<&SnapshotPaths>,
) -> std::io::Result<u32> {
    if let Some(paths) = snapshot_paths {
        write_snapshot(vp, paths)?;
        Ok(1)
    } else {
        Ok(0)
    }
}
