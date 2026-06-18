//! vision_tier0 — minimal tier-0 visual frontend (Phase 3 Stage 1).
//!
//! # Why this exists
//!
//! The legacy visual stack is anthropomorphic: 4×4 rigid region grid, fake
//! PHOT bands (`brightness * 0.5` paraded as `edge_energy`), per-frame slot
//! claims tied to SLOT_COUNT=8. It pipes pixels into the categorical-object
//! slot architecture that perturbation_graph.rs replaces.
//!
//! This module is the lean replacement: edge + frame-diff motion + connected
//! components → ~30 patches/frame, each carrying a 24-D tier-0 signature
//! (HSV statistics + 4-bin gradient orientation histogram + spatial position
//! + scale + motion summary). Patches are written into a PerturbationGraph
//! via nearest-node matching: a re-recognised perturbation REFRESHES the
//! existing node (live-neuron semantics), a novel signature SEDIMENTS a new
//! one.
//!
//! # Stage 1 scope
//!
//! - The pipeline + signature layout + match-or-spawn logic.
//! - Pure CPU, no GPU dependency, no heavy crates.
//! - Unit tests on synthetic blob frames.
//! - NOT wired into body.rs tick loop yet — Stage 2 plugs it behind a feature
//!   flag in parallel with the existing slot ingress.
//!
//! Numeric/local-learning only — no labels, no backprop, no global
//! gradients. The 24-D signature axes are arbitrary tier-0 measurements; their
//! "meaning" (red, round, friendly, ...) emerges from edge statistics in the
//! perturbation_graph, not from this module.

use crate::perturbation_graph::{
    EdgeKind, NodeId, PerturbationGraph, RegionKey, GROUP_FORM_DIST,
    GROUP_FORM_MIN_NODES, PG_CROSS_MODAL_DIM, PG_NODE_SIG_DIM, PG_VITAL_DIM,
};
use crate::phot_camera::{CameraFrame, CAM_H, CAM_PIX, CAM_W};
use std::collections::HashMap;

/// Edge magnitude threshold (Sobel `(Gx² + Gy²).sqrt()`) above which a pixel
/// counts as "interesting" for connected-component segmentation. Tuned for the
/// CAM_W×CAM_H luma in [0,1]: 0.10 picks up most real-world edges while
/// rejecting sensor noise.
pub const TIER0_EDGE_THRESHOLD: f64 = 0.10;

/// Frame-difference motion threshold. Above this, a pixel is "moving".
pub const TIER0_MOTION_THRESHOLD: f64 = 0.05;

/// Minimum pixels in a connected component to spawn a patch. Smaller than
/// PHOT_REGION_MIN_PIXELS (50) so we capture small but real perturbations
/// (e.g. an eye blink, a moving fingertip). 16 ≈ a 4×4 patch.
pub const TIER0_MIN_PATCH_PIXELS: usize = 16;

/// Cap on patches per frame so a noisy/static frame can't flood the graph
/// with admission-eligible novel signatures. The perturbation_graph's own
/// node cap (PG_NODE_CAP=2048) is the lifetime backstop; this is the per-tick
/// guard. 64 leaves headroom while keeping the live-neuron update set bounded.
pub const TIER0_MAX_PATCHES_PER_FRAME: usize = 64;

/// Signature distance under which we consider a new patch to be a refresh of
/// an existing perturbation_graph node (instead of a new admission). 0.30 was
/// chosen against synthetic test fixtures — close enough that mild lighting
/// drift on the same object still matches, loose enough that two visibly
/// distinct objects sediment separately.
pub const TIER0_SIG_MATCH_DIST: f64 = 0.30;

/// EMA alpha for refresh_node when a patch matches an existing node. 0.25
/// gives a "floating truth" with a few-frame integration window — fast enough
/// to track real change, slow enough that noise doesn't drag the signature.
pub const TIER0_REFRESH_ALPHA: f64 = 0.25;

/// EMA alpha for cross_modal and vital_coupling updates. Slower than the
/// signature refresh because we want long-term correlation, not instantaneous
/// match.
pub const TIER0_COUPLING_ALPHA: f64 = 0.05;

/// Gradient orientation bin count (4: 0°, 45°, 90°, 135° — mod π so opposite
/// gradients alias to the same bin). 4 is enough to distinguish "vertical-
/// dominant" from "horizontal-dominant" textures cheaply.
const N_ORIENT_BINS: usize = 4;

/// Phase A2: visual space split into GROUP_SPATIAL_BUCKETS × GROUP_SPATIAL_BUCKETS
/// coarse cells for spatial RegionKey assignment. 16 per axis → 256 cells,
/// each ~6% of the frame in each dimension — coarse enough that small jitter
/// keeps a patch in the same bucket, fine enough that a face and a hand at
/// opposite sides of the frame are in different cells.
pub const GROUP_SPATIAL_BUCKETS: u8 = 16;

/// Phase A2: temporal RegionKey bucket size. Patches that are co-active within
/// the same 1 s window share the temporal region — coarse enough to absorb
/// per-tick noise while letting "what's on screen at the same time" emerge as
/// a group.
pub const GROUP_TEMPORAL_WINDOW_US: u64 = 1_000_000;

/// Stable signature index ranges. Pinned here so downstream code can refer
/// to specific axes by name without magic numbers. Total = PG_NODE_SIG_DIM = 24.
mod sig {
    pub const HSV_MEAN: std::ops::Range<usize> = 0..3;
    pub const HSV_VAR: std::ops::Range<usize> = 3..6;
    pub const GRAD_HIST: std::ops::Range<usize> = 6..10;
    pub const POS: std::ops::Range<usize> = 10..12;
    pub const SPAN: std::ops::Range<usize> = 12..14;
    pub const LOG_AREA: usize = 14;
    pub const ECCENTRICITY: usize = 15;
    pub const MOTION_MAG_DXDYPEAK: std::ops::Range<usize> = 16..20;
    // 20..24 reserved for downstream enrichment (depth, vital coupling cache, etc).
}

/// One patch extracted from this frame (pre-graph). Not stored — fed straight
/// into PerturbationGraph::nearest_node / refresh_node / add_node by the
/// caller and then dropped.
#[derive(Clone, Debug, PartialEq)]
pub struct Tier0Patch {
    pub sig: [f64; PG_NODE_SIG_DIM],
    pub pixel_count: usize,
    pub motion_mag_mean: f64,
    pub motion_mag_peak: f64,
    /// Patch centroid in pixels (debugging / cross-checks).
    pub centroid_px: (f64, f64),
}

/// Stateful tier-0 visual frontend. Keeps the previous frame's luma so motion
/// can be diffed. Pure CPU.
pub struct VisionTier0 {
    prev_luma: Option<Vec<f64>>,
}

impl Default for VisionTier0 {
    fn default() -> Self {
        Self::new()
    }
}

impl VisionTier0 {
    pub fn new() -> Self {
        Self { prev_luma: None }
    }

    /// Reset (used when the upstream provider faults — we don't want to diff
    /// against a stale frame after the recovery).
    pub fn reset(&mut self) {
        self.prev_luma = None;
    }

    pub fn has_prev(&self) -> bool {
        self.prev_luma.is_some()
    }

    /// Extract patches from this frame WITHOUT updating the perturbation_graph
    /// (mostly useful for tests + telemetry). The graph-update path is
    /// `process_frame_into_graph` below.
    pub fn extract_patches(&mut self, frame: &CameraFrame) -> Vec<Tier0Patch> {
        debug_assert_eq!(frame.luma.len(), CAM_PIX);
        debug_assert_eq!(frame.r.len(), CAM_PIX);
        debug_assert_eq!(frame.g.len(), CAM_PIX);
        debug_assert_eq!(frame.b.len(), CAM_PIX);

        let edge_mag = sobel_magnitude(&frame.luma, CAM_W, CAM_H);
        let edge_orient = sobel_orientation(&frame.luma, CAM_W, CAM_H);

        let motion = match self.prev_luma.as_ref() {
            Some(prev) => {
                let mut m = vec![0.0_f64; CAM_PIX];
                for i in 0..CAM_PIX {
                    m[i] = (frame.luma[i] - prev[i]).abs();
                }
                m
            }
            None => vec![0.0; CAM_PIX],
        };
        self.prev_luma = Some(frame.luma.clone());

        // Mask = (edge OR motion). Either source of "this pixel is interesting"
        // gets it into a connected component. A purely-static scene with rich
        // texture still produces patches (via edges). A clear flat object that
        // moves still produces patches (via motion).
        let mask: Vec<bool> = edge_mag
            .iter()
            .zip(motion.iter())
            .map(|(&e, &m)| e >= TIER0_EDGE_THRESHOLD || m >= TIER0_MOTION_THRESHOLD)
            .collect();

        let components = label_connected_components(&mask, CAM_W, CAM_H);
        let mut patches: Vec<Tier0Patch> = components
            .into_iter()
            .filter(|c| c.len() >= TIER0_MIN_PATCH_PIXELS)
            .map(|pixels| {
                build_patch(
                    &pixels,
                    frame,
                    &edge_mag,
                    &edge_orient,
                    &motion,
                    CAM_W,
                    CAM_H,
                )
            })
            .collect();

        // Energy-descending cap: prefer the loudest patches when over the cap.
        patches.sort_by(|a, b| {
            let ea = a.motion_mag_peak + a.pixel_count as f64 * 1e-6;
            let eb = b.motion_mag_peak + b.pixel_count as f64 * 1e-6;
            eb.partial_cmp(&ea).unwrap_or(std::cmp::Ordering::Equal)
        });
        patches.truncate(TIER0_MAX_PATCHES_PER_FRAME);
        patches
    }

    /// Process one camera frame: extract patches, then for each patch either
    /// refresh the matching perturbation_graph node (signature distance ≤
    /// TIER0_SIG_MATCH_DIST) or sediment a new node. Optional coupling
    /// readings update the cross-modal / vital axes on refreshed nodes only —
    /// novel nodes start clean so the coupling stats reflect genuine
    /// accumulated evidence on identified patches.
    ///
    /// Returns the ids of every node that was refreshed or admitted this
    /// frame (the "active set" for downstream observers).
    pub fn process_frame_into_graph(
        &mut self,
        frame: &CameraFrame,
        now_us: u64,
        graph: &mut PerturbationGraph,
        cross_modal: Option<&[f64; PG_CROSS_MODAL_DIM]>,
        vital: Option<&[f64; PG_VITAL_DIM]>,
    ) -> Vec<NodeId> {
        let patches = self.extract_patches(frame);
        let mut active: Vec<NodeId> = Vec::with_capacity(patches.len());
        for p in patches {
            let matched = graph.nearest_node(&p.sig, TIER0_SIG_MATCH_DIST);
            let id = match matched {
                Some((existing, _)) => {
                    graph.refresh_node(existing, now_us, &p.sig, TIER0_REFRESH_ALPHA);
                    if let Some(cm) = cross_modal {
                        graph.update_cross_modal(existing, cm, TIER0_COUPLING_ALPHA);
                    }
                    if let Some(v) = vital {
                        graph.update_vital_coupling(existing, v, TIER0_COUPLING_ALPHA);
                    }
                    if let Some(n) = graph.node_mut(existing) {
                        // EMA the marginal stats so affordances ("can move",
                        // typical size) accumulate on the right node.
                        let m = p.motion_mag_mean;
                        n.motion_var = 0.9 * n.motion_var + 0.1 * (m * m);
                        // Cap size_ema at CAM_PIX (the entire frame) so a
                        // central anchor patch can't grow without bound and
                        // become a "vacuum node" that swallows every frame's
                        // center region — the first post-Phase-A diagnostic
                        // found id=5 with size_ema=16735, well above any
                        // single real patch.
                        let new_size = 0.9 * n.size_ema + 0.1 * (p.pixel_count as f64);
                        n.size_ema = new_size.min(CAM_PIX as f64);
                        // persistence rises every time this node is refreshed
                        // and decays elsewhere (caller's decay tick — Stage 2).
                        n.persistence_ema =
                            (n.persistence_ema + 0.1 * (1.0 - n.persistence_ema))
                                .clamp(0.0, 1.0);
                    }
                    existing
                }
                None => {
                    let new_id = graph.add_node(now_us, p.sig);
                    if let Some(n) = graph.node_mut(new_id) {
                        n.motion_var = p.motion_mag_mean * p.motion_mag_mean;
                        n.size_ema = (p.pixel_count as f64).min(CAM_PIX as f64);
                        n.persistence_ema = 0.1;
                    }
                    new_id
                }
            };
            active.push(id);
        }
        // Co-occurrence edges between every pair of patches present THIS
        // frame: simultaneous activation is the cheapest possible "these
        // belong to the same scene" hint, and EMA strength lets the genuine
        // relations dominate the spurious ones across many frames. O(N²) per
        // frame but N ≤ TIER0_MAX_PATCHES_PER_FRAME, so worst-case 64² = 4096
        // edge calls per frame — well below PG_EDGE_CAP per-call cost.
        for i in 0..active.len() {
            for j in (i + 1)..active.len() {
                graph.observe_edge(now_us, active[i], active[j], EdgeKind::Cooccur);
                graph.observe_edge(now_us, active[j], active[i], EdgeKind::Cooccur);
            }
        }
        // ALSO promote the SAME co-occurrence into the first-class Relation
        // layer for the TOP K most-bound active patches this frame. The first
        // post-Phase-A diagnostic showed zero visual relations (and therefore
        // zero context drift into the Cooccur primitive RelationKind) because
        // vision_tier0 was only emitting low-level edges. Capping to the top
        // K=8 active patches keeps the per-frame cost bounded at K(K-1)=56
        // observe_relation calls while still giving the high-bind patches a
        // path into the relation substrate that downstream FastPath /
        // RelationKind / mark queries actually walk.
        const TIER0_TOP_K_RELATIONS: usize = 8;
        let mut top_active: Vec<(NodeId, f64)> = active
            .iter()
            .filter_map(|&nid| {
                graph
                    .node(nid)
                    .map(|n| (nid, n.bind_strength * (1.0 + n.persistence_ema)))
            })
            .collect();
        top_active.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        top_active.truncate(TIER0_TOP_K_RELATIONS);
        for i in 0..top_active.len() {
            for j in 0..top_active.len() {
                if i == j {
                    continue;
                }
                graph.observe_relation(
                    now_us,
                    crate::perturbation_graph::Endpoint::Node(top_active[i].0),
                    crate::perturbation_graph::Endpoint::Node(top_active[j].0),
                    EdgeKind::Cooccur,
                );
            }
        }
        // Phase A2: form / refresh region-local NodeGroups from the active set.
        // Each tick, active patches sort themselves into spatial buckets and a
        // shared temporal window; the streaming pass either grows an existing
        // group (refresh) or spawns a new one (when a cohort of ≥ MIN_NODES
        // co-located patches has no existing match). Per-tick decay sweeps
        // stale members. This is the substrate that lets a "neuron" emerge as
        // the SOFT MEMBERSHIP of a region-local perturbation cluster.
        refresh_active_groups(now_us, &active, graph);
        graph.decay_group_members(now_us);
        active
    }
}

/// Phase A2 group-formation pass. Given the set of patch node ids that were
/// touched this frame, sort them into spatial buckets + a shared temporal
/// window region, and either refresh an existing group in each region or spawn
/// a new one when the cohort is large enough.
///
/// This runs at the frame rate; a 30 Hz camera fires ~30 group passes per
/// second. Cost is O(active × log groups) per call — bounded because active is
/// capped at TIER0_MAX_PATCHES_PER_FRAME (64) and the per-region group count
/// is capped at PG_GROUP_CAP / N_REGIONS (much less than 64 in practice).
fn refresh_active_groups(
    now_us: u64,
    active: &[NodeId],
    graph: &mut PerturbationGraph,
) {
    if active.is_empty() {
        return;
    }
    // === Spatial buckets ===
    // Use the patches' tier-0 pos_x / pos_y (sig::POS = 10..12) to bucket
    // each active node into a coarse spatial cell. A node may not resolve
    // (e.g. it was evicted mid-frame); silently skip those.
    let mut spatial_buckets: HashMap<RegionKey, Vec<NodeId>> =
        HashMap::with_capacity(active.len().min(16));
    for &nid in active {
        if let Some(n) = graph.node(nid) {
            let pos_x = n.sig[sig::POS.start].clamp(0.0, 1.0);
            let pos_y = n.sig[sig::POS.start + 1].clamp(0.0, 1.0);
            let bx = (pos_x * GROUP_SPATIAL_BUCKETS as f64) as u8;
            let by = (pos_y * GROUP_SPATIAL_BUCKETS as f64) as u8;
            let bx = bx.min(GROUP_SPATIAL_BUCKETS - 1);
            let by = by.min(GROUP_SPATIAL_BUCKETS - 1);
            spatial_buckets
                .entry(RegionKey::Spatial(bx, by))
                .or_default()
                .push(nid);
        }
    }
    for (region, cohort) in &spatial_buckets {
        admit_cohort_to_region(now_us, *region, cohort, graph);
    }
    // === Temporal window ===
    // All patches active in this tick share a temporal region — independent of
    // their spatial position. This is what lets a face + a voice + a touch
    // that co-occur every Tuesday afternoon eventually live in the same group
    // even though their spatial regions are unrelated.
    let temporal_region = RegionKey::TemporalWindow(now_us / GROUP_TEMPORAL_WINDOW_US);
    admit_cohort_to_region(now_us, temporal_region, active, graph);
}

/// Cohort-into-region helper: refresh into an existing matching group when
/// there is one, otherwise spawn a fresh group provided the cohort is large
/// enough to clear GROUP_FORM_MIN_NODES.
fn admit_cohort_to_region(
    now_us: u64,
    region: RegionKey,
    cohort: &[NodeId],
    graph: &mut PerturbationGraph,
) {
    if cohort.len() < GROUP_FORM_MIN_NODES {
        // Below the minimum cohort threshold: don't spawn a group, but if one
        // already exists in this region, refresh its members anyway — a sparse
        // group that loses bodies temporarily shouldn't be abandoned.
        let centroid = cohort_centroid_sig(cohort, graph);
        if let Some((gid, _)) = graph.nearest_group(region, &centroid, GROUP_FORM_DIST) {
            for &nid in cohort {
                graph.refresh_group(gid, nid, now_us);
            }
        }
        return;
    }
    let centroid = cohort_centroid_sig(cohort, graph);
    if let Some((gid, _)) = graph.nearest_group(region, &centroid, GROUP_FORM_DIST) {
        for &nid in cohort {
            graph.refresh_group(gid, nid, now_us);
        }
    } else {
        graph.add_group(now_us, region, cohort);
    }
}

/// Unweighted mean of a cohort's sigs. Used for nearest_group lookups so the
/// region match key is the cohort's collective fingerprint, not any one
/// member's. Returns the zero sig if no member resolves to a live node.
fn cohort_centroid_sig(
    cohort: &[NodeId],
    graph: &PerturbationGraph,
) -> [f64; PG_NODE_SIG_DIM] {
    let mut centroid = [0.0_f64; PG_NODE_SIG_DIM];
    let mut count = 0usize;
    for &nid in cohort {
        if let Some(n) = graph.node(nid) {
            for i in 0..PG_NODE_SIG_DIM {
                centroid[i] += n.sig[i];
            }
            count += 1;
        }
    }
    if count > 0 {
        let denom = count as f64;
        for v in centroid.iter_mut() {
            *v /= denom;
        }
    }
    centroid
}

// ── Patch builder ──────────────────────────────────────────────────────────

fn build_patch(
    pixels: &[usize],
    frame: &CameraFrame,
    edge_mag: &[f64],
    edge_orient: &[f64],
    motion: &[f64],
    w: usize,
    h: usize,
) -> Tier0Patch {
    let n = pixels.len();
    debug_assert!(n > 0);

    // Centroid + spans (luma-weighted for stability — pixels at the patch
    // boundary that just barely cleared the threshold contribute less).
    let mut sum_w = 0.0_f64;
    let mut sum_wx = 0.0_f64;
    let mut sum_wy = 0.0_f64;
    let mut peak_m = 0.0_f64;
    let mut sum_m = 0.0_f64;
    for &i in pixels {
        let wgt = edge_mag[i].max(motion[i]).max(1e-6);
        let px = (i % w) as f64;
        let py = (i / w) as f64;
        sum_w += wgt;
        sum_wx += px * wgt;
        sum_wy += py * wgt;
        sum_m += motion[i];
        if motion[i] > peak_m {
            peak_m = motion[i];
        }
    }
    let cx = sum_wx / sum_w;
    let cy = sum_wy / sum_w;

    // Second moments → spans + eccentricity.
    let mut vx = 0.0_f64;
    let mut vy = 0.0_f64;
    let mut vxy = 0.0_f64;
    for &i in pixels {
        let wgt = edge_mag[i].max(motion[i]).max(1e-6);
        let dx = (i % w) as f64 - cx;
        let dy = (i / w) as f64 - cy;
        vx += dx * dx * wgt;
        vy += dy * dy * wgt;
        vxy += dx * dy * wgt;
    }
    vx = (vx / sum_w).max(0.0).sqrt();
    vy = (vy / sum_w).max(0.0).sqrt();
    let _ = vxy; // available for elongation direction if needed later

    // HSV stats over the patch pixels.
    let mut hsv_sum = [0.0_f64; 3];
    let mut hsv_sq = [0.0_f64; 3];
    for &i in pixels {
        let hsv = rgb_to_hsv(frame.r[i], frame.g[i], frame.b[i]);
        for k in 0..3 {
            hsv_sum[k] += hsv[k];
            hsv_sq[k] += hsv[k] * hsv[k];
        }
    }
    let nf = n as f64;
    let mut hsv_mean = [0.0_f64; 3];
    let mut hsv_var = [0.0_f64; 3];
    for k in 0..3 {
        let m = hsv_sum[k] / nf;
        hsv_mean[k] = m;
        hsv_var[k] = (hsv_sq[k] / nf - m * m).max(0.0).sqrt();
    }

    // Gradient orientation 4-bin histogram, weighted by edge magnitude so flat
    // patches don't bias the distribution.
    let mut hist = [0.0_f64; N_ORIENT_BINS];
    let mut hist_sum = 0.0_f64;
    for &i in pixels {
        let m = edge_mag[i];
        if m < 1e-6 {
            continue;
        }
        // edge_orient is in [0, π); bin = floor(orient * N / π).
        let bin = ((edge_orient[i] * N_ORIENT_BINS as f64) / std::f64::consts::PI)
            .floor() as usize;
        let bin = bin.min(N_ORIENT_BINS - 1);
        hist[bin] += m;
        hist_sum += m;
    }
    if hist_sum > 1e-9 {
        for v in &mut hist {
            *v /= hist_sum;
        }
    }

    // Normalized spatial coords + spans (axis values are scale-invariant).
    let pos_x = (cx / (w as f64 - 1.0)).clamp(0.0, 1.0);
    let pos_y = (cy / (h as f64 - 1.0)).clamp(0.0, 1.0);
    let span_x = (vx / w as f64).clamp(0.0, 1.0);
    let span_y = (vy / h as f64).clamp(0.0, 1.0);

    let log_area = ((n as f64).ln() / 12.0).clamp(0.0, 1.0); // ln(CAM_PIX) ≈ 11.3
    let eccentricity = if vx > 1e-9 || vy > 1e-9 {
        ((vx - vy).abs() / (vx + vy + 1e-9)).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let motion_mag_mean = sum_m / nf;

    let mut sig = [0.0_f64; PG_NODE_SIG_DIM];
    for k in 0..3 {
        sig[sig::HSV_MEAN.start + k] = hsv_mean[k];
        sig[sig::HSV_VAR.start + k] = hsv_var[k];
    }
    for k in 0..N_ORIENT_BINS {
        sig[sig::GRAD_HIST.start + k] = hist[k];
    }
    sig[sig::POS.start] = pos_x;
    sig[sig::POS.start + 1] = pos_y;
    sig[sig::SPAN.start] = span_x;
    sig[sig::SPAN.start + 1] = span_y;
    sig[sig::LOG_AREA] = log_area;
    sig[sig::ECCENTRICITY] = eccentricity;
    // Motion summary in sig[16..20]: (mean_mag, peak_mag, dx, dy).
    // Stage 1: dx, dy are zero — full optical flow lands in Stage 2.
    sig[sig::MOTION_MAG_DXDYPEAK.start] = motion_mag_mean.clamp(0.0, 1.0);
    sig[sig::MOTION_MAG_DXDYPEAK.start + 1] = peak_m.clamp(0.0, 1.0);
    sig[sig::MOTION_MAG_DXDYPEAK.start + 2] = 0.0;
    sig[sig::MOTION_MAG_DXDYPEAK.start + 3] = 0.0;
    // sig[20..24] left zero — reserved for downstream enrichment.

    Tier0Patch {
        sig,
        pixel_count: n,
        motion_mag_mean,
        motion_mag_peak: peak_m,
        centroid_px: (cx, cy),
    }
}

// ── Sobel edge magnitude + orientation ─────────────────────────────────────

/// Sobel gradient magnitude, mirror-padded at borders. Output length = w*h.
pub fn sobel_magnitude(luma: &[f64], w: usize, h: usize) -> Vec<f64> {
    debug_assert_eq!(luma.len(), w * h);
    let mut out = vec![0.0_f64; w * h];
    // For each interior pixel, compute Gx, Gy; borders mirror.
    let at = |x: isize, y: isize| -> f64 {
        let xc = x.clamp(0, w as isize - 1) as usize;
        let yc = y.clamp(0, h as isize - 1) as usize;
        luma[yc * w + xc]
    };
    for y in 0..h as isize {
        for x in 0..w as isize {
            // Gx = (a02 + 2a12 + a22) - (a00 + 2a10 + a20)
            // Gy = (a20 + 2a21 + a22) - (a00 + 2a01 + a02)
            let a00 = at(x - 1, y - 1);
            let a01 = at(x, y - 1);
            let a02 = at(x + 1, y - 1);
            let a10 = at(x - 1, y);
            let a12 = at(x + 1, y);
            let a20 = at(x - 1, y + 1);
            let a21 = at(x, y + 1);
            let a22 = at(x + 1, y + 1);
            let gx = (a02 + 2.0 * a12 + a22) - (a00 + 2.0 * a10 + a20);
            let gy = (a20 + 2.0 * a21 + a22) - (a00 + 2.0 * a01 + a02);
            out[(y as usize) * w + (x as usize)] = (gx * gx + gy * gy).sqrt();
        }
    }
    out
}

/// Sobel gradient orientation in `[0, π)` (orientations mod π — opposite
/// gradients alias to the same value, which is what we want for edges).
pub fn sobel_orientation(luma: &[f64], w: usize, h: usize) -> Vec<f64> {
    debug_assert_eq!(luma.len(), w * h);
    let mut out = vec![0.0_f64; w * h];
    let at = |x: isize, y: isize| -> f64 {
        let xc = x.clamp(0, w as isize - 1) as usize;
        let yc = y.clamp(0, h as isize - 1) as usize;
        luma[yc * w + xc]
    };
    for y in 0..h as isize {
        for x in 0..w as isize {
            let a00 = at(x - 1, y - 1);
            let a01 = at(x, y - 1);
            let a02 = at(x + 1, y - 1);
            let a10 = at(x - 1, y);
            let a12 = at(x + 1, y);
            let a20 = at(x - 1, y + 1);
            let a21 = at(x, y + 1);
            let a22 = at(x + 1, y + 1);
            let gx = (a02 + 2.0 * a12 + a22) - (a00 + 2.0 * a10 + a20);
            let gy = (a20 + 2.0 * a21 + a22) - (a00 + 2.0 * a01 + a02);
            let theta = gy.atan2(gx);
            // Map (-π, π] → [0, π) by mod π.
            let mut t = theta;
            if t < 0.0 {
                t += std::f64::consts::PI;
            }
            if t >= std::f64::consts::PI {
                t -= std::f64::consts::PI;
            }
            out[(y as usize) * w + (x as usize)] = t;
        }
    }
    out
}

// ── HSV ────────────────────────────────────────────────────────────────────

/// Convert one RGB pixel in `[0,1]` to HSV in `[0,1]³`. Standard formula; H
/// is normalised so the value space matches the other axes (no special
/// circular handling — Stage 1 keeps signatures Euclidean for the
/// nearest_node distance).
pub fn rgb_to_hsv(r: f64, g: f64, b: f64) -> [f64; 3] {
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let v = max;
    let d = max - min;
    let s = if max > 1e-9 { d / max } else { 0.0 };
    let h_deg = if d < 1e-9 {
        0.0
    } else if (max - r).abs() < 1e-9 {
        60.0 * (((g - b) / d) % 6.0)
    } else if (max - g).abs() < 1e-9 {
        60.0 * (((b - r) / d) + 2.0)
    } else {
        60.0 * (((r - g) / d) + 4.0)
    };
    let h_deg = if h_deg < 0.0 { h_deg + 360.0 } else { h_deg };
    [h_deg / 360.0, s, v]
}

// ── Connected components (4-connected BFS) ─────────────────────────────────

/// Label 4-connected above-mask pixels into components. Mirrors the existing
/// `extract_regions_from_motion` BFS but takes an arbitrary boolean mask
/// (edge OR motion, not just motion) so the same tier-0 primitive serves
/// both texture and movement.
fn label_connected_components(mask: &[bool], w: usize, h: usize) -> Vec<Vec<usize>> {
    let n = w * h;
    debug_assert_eq!(mask.len(), n);
    let mut visited = vec![false; n];
    let mut components: Vec<Vec<usize>> = Vec::new();
    for start in 0..n {
        if !mask[start] || visited[start] {
            continue;
        }
        let mut region = Vec::new();
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(start);
        visited[start] = true;
        while let Some(idx) = queue.pop_front() {
            region.push(idx);
            let px = idx % w;
            let py = idx / w;
            if px > 0 {
                let nb = idx - 1;
                if mask[nb] && !visited[nb] {
                    visited[nb] = true;
                    queue.push_back(nb);
                }
            }
            if px + 1 < w {
                let nb = idx + 1;
                if mask[nb] && !visited[nb] {
                    visited[nb] = true;
                    queue.push_back(nb);
                }
            }
            if py > 0 {
                let nb = idx - w;
                if mask[nb] && !visited[nb] {
                    visited[nb] = true;
                    queue.push_back(nb);
                }
            }
            if py + 1 < h {
                let nb = idx + w;
                if mask[nb] && !visited[nb] {
                    visited[nb] = true;
                    queue.push_back(nb);
                }
            }
        }
        components.push(region);
    }
    components
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a small test frame whose luma+RGB carry one or more bright disks.
    fn frame_with_disks(disks: &[(f64, f64, f64, [f64; 3])]) -> CameraFrame {
        // (cx, cy, radius_px, rgb)
        let mut f = CameraFrame::zeros(0);
        for y in 0..CAM_H {
            for x in 0..CAM_W {
                let idx = y * CAM_W + x;
                for &(cx, cy, r, rgb) in disks {
                    let dx = x as f64 - cx;
                    let dy = y as f64 - cy;
                    if dx * dx + dy * dy <= r * r {
                        f.luma[idx] = (rgb[0] + rgb[1] + rgb[2]) / 3.0;
                        f.r[idx] = rgb[0];
                        f.g[idx] = rgb[1];
                        f.b[idx] = rgb[2];
                    }
                }
            }
        }
        f
    }

    #[test]
    fn sobel_magnitude_fires_on_a_step_edge() {
        // Vertical step at x = w/2: left = 0, right = 1.
        let w = 32;
        let h = 8;
        let mut luma = vec![0.0; w * h];
        for y in 0..h {
            for x in 0..w {
                luma[y * w + x] = if x >= w / 2 { 1.0 } else { 0.0 };
            }
        }
        let mag = sobel_magnitude(&luma, w, h);
        // The edge column should have the strongest magnitudes.
        let edge_col_max = (0..h)
            .map(|y| mag[y * w + (w / 2 - 1)])
            .fold(0.0_f64, f64::max);
        let interior_max = (0..h)
            .map(|y| mag[y * w + 2])
            .fold(0.0_f64, f64::max);
        assert!(
            edge_col_max > 0.5,
            "Sobel must fire strongly on a step edge, got {edge_col_max}"
        );
        assert!(
            interior_max < 0.1,
            "Sobel must not fire on a flat region, got {interior_max}"
        );
    }

    #[test]
    fn rgb_to_hsv_canonical_corners() {
        assert!((rgb_to_hsv(1.0, 0.0, 0.0)[0] - 0.0).abs() < 1e-9);
        assert!((rgb_to_hsv(0.0, 1.0, 0.0)[0] - (120.0 / 360.0)).abs() < 1e-9);
        assert!((rgb_to_hsv(0.0, 0.0, 1.0)[0] - (240.0 / 360.0)).abs() < 1e-9);
        // Grey: zero saturation regardless of value.
        let g = rgb_to_hsv(0.5, 0.5, 0.5);
        assert!(g[1] < 1e-9 && (g[2] - 0.5).abs() < 1e-9);
    }

    #[test]
    fn extract_patches_finds_two_distinct_patches_at_disk_locations() {
        // Two disks at clearly separable positions. The patch signatures
        // CAN'T be checked by hue alone — Stage 1's tier-0 segmentation
        // captures the EDGE RING of each disk, so HSV-mean over a 1–2 px ring
        // mixes the disk's interior colour with the surrounding black. A
        // future Stage 1.5 hole-fill pass would clean this up; for now the
        // correctness contract is (a) two patches exist, (b) they're at the
        // two disk locations, and (c) their signatures are clearly distinct.
        let frame = frame_with_disks(&[
            (80.0, 120.0, 20.0, [1.0, 0.2, 0.2]),
            (240.0, 120.0, 20.0, [0.2, 0.2, 1.0]),
        ]);
        let mut v = VisionTier0::new();
        let patches = v.extract_patches(&frame);
        assert!(
            patches.len() >= 2,
            "expected ≥2 patches for two disks, got {}",
            patches.len()
        );
        // (b) Centroids should bracket the two disk positions.
        let near = |a: f64, b: f64, tol: f64| (a - b).abs() < tol;
        let left = patches
            .iter()
            .any(|p| near(p.centroid_px.0, 80.0, 8.0) && near(p.centroid_px.1, 120.0, 8.0));
        let right = patches
            .iter()
            .any(|p| near(p.centroid_px.0, 240.0, 8.0) && near(p.centroid_px.1, 120.0, 8.0));
        assert!(left, "no patch near the left disk centroid");
        assert!(right, "no patch near the right disk centroid");
        // (c) The two strongest patches must have visibly different signatures
        //     — purely-distinct red vs blue must NOT collapse to one node.
        let mut top: Vec<&Tier0Patch> =
            patches.iter().take(8).collect(); // top by energy-sort
        top.sort_by(|a, b| {
            b.pixel_count
                .cmp(&a.pixel_count)
        });
        let a = top[0];
        let b = top[1];
        let dist = crate::perturbation_graph::sig_distance(&a.sig, &b.sig);
        assert!(
            dist > TIER0_SIG_MATCH_DIST,
            "two distinctly-coloured disks must produce signatures farther apart than the match distance: got {dist}"
        );
    }

    #[test]
    fn frame_to_frame_match_refreshes_existing_node() {
        // Two frames with the same disk; the second frame should refresh, not
        // sediment a new node. Confirms the nearest_node match works against
        // a real tier-0 signature.
        let f1 = frame_with_disks(&[(160.0, 120.0, 30.0, [1.0, 0.4, 0.4])]);
        let f2 = frame_with_disks(&[(160.0, 120.0, 30.0, [1.0, 0.4, 0.4])]);
        let mut v = VisionTier0::new();
        let mut g = PerturbationGraph::default();
        let active1 = v.process_frame_into_graph(&f1, 1_000, &mut g, None, None);
        assert!(!active1.is_empty(), "frame 1 must produce at least one node");
        let nodes_after_f1 = g.nodes.len();
        let active2 = v.process_frame_into_graph(&f2, 2_000, &mut g, None, None);
        assert!(!active2.is_empty(), "frame 2 must produce at least one node");
        let nodes_after_f2 = g.nodes.len();
        // The dominant patch on both frames is the same disk; nodes count
        // should be stable (allowing edge-pixel jitter to maybe add 1).
        assert!(
            nodes_after_f2 <= nodes_after_f1 + 1,
            "matching disk should refresh, not re-sediment: f1={nodes_after_f1}, f2={nodes_after_f2}"
        );
    }

    #[test]
    fn distinct_disks_sediment_separate_nodes() {
        // A red disk and a blue disk are visually distinct in HSV space →
        // they must produce DIFFERENT nodes after both are processed.
        let red = frame_with_disks(&[(160.0, 120.0, 30.0, [1.0, 0.2, 0.2])]);
        let blue = frame_with_disks(&[(160.0, 120.0, 30.0, [0.2, 0.2, 1.0])]);
        let mut v = VisionTier0::new();
        let mut g = PerturbationGraph::default();
        v.process_frame_into_graph(&red, 1_000, &mut g, None, None);
        let nodes_after_red = g.nodes.len();
        v.process_frame_into_graph(&blue, 2_000, &mut g, None, None);
        let nodes_after_blue = g.nodes.len();
        assert!(
            nodes_after_blue > nodes_after_red,
            "the visually distinct blue disk must NOT match the red node: \
             red={nodes_after_red}, blue={nodes_after_blue}"
        );
    }

    #[test]
    fn cooccurrence_edges_form_between_simultaneous_patches() {
        // Two disks in the same frame must produce a Cooccur edge between
        // their nodes (both directions).
        let f = frame_with_disks(&[
            (80.0, 120.0, 20.0, [1.0, 0.2, 0.2]),
            (240.0, 120.0, 20.0, [0.2, 0.2, 1.0]),
        ]);
        let mut v = VisionTier0::new();
        let mut g = PerturbationGraph::default();
        let active = v.process_frame_into_graph(&f, 1_000, &mut g, None, None);
        assert!(active.len() >= 2, "two disks must give ≥2 nodes");
        let cooc_count = g
            .edges
            .iter()
            .filter(|e| e.kind_byte == EdgeKind::Cooccur.as_u8())
            .count();
        assert!(cooc_count >= 2, "simultaneous patches must form Cooccur edges in both directions");
    }

    #[test]
    fn novel_patches_keep_marginal_stats_clean() {
        // A freshly-admitted node must have its marginal stats reflect the
        // patch (not stay at the default seed). Confirms the post-add update
        // path in process_frame_into_graph is wired.
        let f = frame_with_disks(&[(160.0, 120.0, 30.0, [1.0, 0.5, 0.5])]);
        let mut v = VisionTier0::new();
        let mut g = PerturbationGraph::default();
        v.process_frame_into_graph(&f, 1_000, &mut g, None, None);
        assert!(!g.nodes.is_empty());
        // size_ema should be roughly the disk area (π × 30² ≈ 2827) but at
        // CAM scale we don't need precision — just non-trivial > 0 confirms
        // wiring.
        let any_node_has_size = g.nodes.iter().any(|n| n.size_ema > 100.0);
        assert!(any_node_has_size, "novel node must record its pixel_count as size_ema");
    }

    #[test]
    fn cross_modal_and_vital_couplings_only_attach_to_matched_nodes() {
        // Couplings are passed to refresh_node only — novel nodes start with
        // zero coupling so the stats reflect genuine accumulated co-occurrence.
        let f = frame_with_disks(&[(160.0, 120.0, 30.0, [1.0, 0.4, 0.4])]);
        let mut v = VisionTier0::new();
        let mut g = PerturbationGraph::default();
        let cm = [0.7; PG_CROSS_MODAL_DIM];
        let vc = [0.5; PG_VITAL_DIM];
        // First call: all patches NEW → no coupling applied (yet).
        v.process_frame_into_graph(&f, 1_000, &mut g, Some(&cm), Some(&vc));
        for n in &g.nodes {
            for &c in &n.cross_modal {
                assert!(c.abs() < 1e-9, "novel node must start with zero cross_modal");
            }
            for &c in &n.vital_coupling {
                assert!(c.abs() < 1e-9, "novel node must start with zero vital_coupling");
            }
        }
        // Second call: same disk → REFRESH → coupling now accumulates.
        v.process_frame_into_graph(&f, 2_000, &mut g, Some(&cm), Some(&vc));
        let any_nonzero_cm =
            g.nodes.iter().any(|n| n.cross_modal.iter().any(|&c| c > 1e-9));
        assert!(any_nonzero_cm, "refreshed node must accumulate cross_modal");
    }

    #[test]
    fn empty_mask_produces_no_patches() {
        let f = CameraFrame::zeros(0); // flat dark frame, no edges, no motion
        let mut v = VisionTier0::new();
        let patches = v.extract_patches(&f);
        assert!(patches.is_empty(), "flat frame must produce zero patches");
    }

    #[test]
    fn reset_clears_motion_diff_state() {
        let f1 = frame_with_disks(&[(160.0, 120.0, 30.0, [1.0, 0.5, 0.5])]);
        let mut v = VisionTier0::new();
        v.extract_patches(&f1);
        assert!(v.has_prev());
        v.reset();
        assert!(!v.has_prev());
    }

    #[test]
    fn signature_axes_layout_is_pinned() {
        // The signature layout is a public contract; pinning the ranges here
        // makes any reshuffle a test failure (forces deliberate updates of
        // downstream readers).
        assert_eq!(sig::HSV_MEAN, 0..3);
        assert_eq!(sig::HSV_VAR, 3..6);
        assert_eq!(sig::GRAD_HIST, 6..10);
        assert_eq!(sig::POS, 10..12);
        assert_eq!(sig::SPAN, 12..14);
        assert_eq!(sig::LOG_AREA, 14);
        assert_eq!(sig::ECCENTRICITY, 15);
        assert_eq!(sig::MOTION_MAG_DXDYPEAK, 16..20);
        // 20..24 reserved.
        assert_eq!(PG_NODE_SIG_DIM, 24);
    }

    // ── Phase A2 tests: group formation pass ───────────────────────────────

    #[test]
    fn three_disks_in_one_frame_form_a_temporal_group() {
        // Three well-separated disks anywhere in the frame all activate within
        // the SAME tick → they share a TemporalWindow region. The spatial
        // buckets may differ (16 buckets each axis = ~20 px wide cells, so
        // separated disks bucket independently) but the temporal window
        // catches them all.
        let mut frame = CameraFrame::zeros(0);
        // Radius 20 (matches the canonical two-disk test); spread horizontally
        // so each fall in a distinct spatial bucket but share the temporal one.
        for &(cx, cy) in &[(60.0_f64, 120.0), (160.0, 120.0), (260.0, 120.0)] {
            for y in 0..CAM_H {
                for x in 0..CAM_W {
                    let dx = x as f64 - cx;
                    let dy = y as f64 - cy;
                    if dx * dx + dy * dy <= 400.0 {
                        let idx = y * CAM_W + x;
                        frame.luma[idx] = 0.9;
                        frame.r[idx] = 0.9;
                        frame.g[idx] = 0.3;
                        frame.b[idx] = 0.3;
                    }
                }
            }
        }
        let mut v = VisionTier0::new();
        let mut g = PerturbationGraph::default();
        let active = v.process_frame_into_graph(&frame, 1_000, &mut g, None, None);
        assert!(
            active.len() >= 3,
            "expected ≥3 patches from three well-separated disks (got {})",
            active.len()
        );
        // Exactly one TemporalWindow group should exist this tick, with all
        // three (or more) patches as members.
        let temporal_groups: Vec<_> = g
            .groups
            .iter()
            .filter(|gp| matches!(gp.region, RegionKey::TemporalWindow(_)))
            .collect();
        assert_eq!(temporal_groups.len(), 1, "exactly one temporal group per tick");
        assert!(
            temporal_groups[0].members.len() >= 3,
            "temporal group must contain ≥3 cohort members (got {})",
            temporal_groups[0].members.len()
        );
    }

    #[test]
    fn second_frame_refreshes_not_re_creates() {
        // Identical frame twice: the spatial group from frame 1 should be
        // REFRESHED on frame 2, not re-spawned. So groups.len() must stay flat
        // or rise by ≤ 1 (the temporal window may roll over to a new bucket).
        let frame = {
            let mut f = CameraFrame::zeros(0);
            for &(cx, cy) in &[(160.0_f64, 120.0), (155.0, 125.0), (165.0, 115.0)] {
                for y in 0..CAM_H {
                    for x in 0..CAM_W {
                        let dx = x as f64 - cx;
                        let dy = y as f64 - cy;
                        if dx * dx + dy * dy <= 100.0 {
                            let idx = y * CAM_W + x;
                            f.luma[idx] = 0.7;
                            f.r[idx] = 0.7;
                            f.g[idx] = 0.6;
                            f.b[idx] = 0.4;
                        }
                    }
                }
            }
            f
        };
        let mut v = VisionTier0::new();
        let mut g = PerturbationGraph::default();
        v.process_frame_into_graph(&frame, 1_000, &mut g, None, None);
        let after_f1 = g.groups.len();
        v.process_frame_into_graph(&frame, 2_000, &mut g, None, None);
        let after_f2 = g.groups.len();
        // Allow ≤ 1 extra in case a patch nudges into an adjacent bucket on
        // the second frame's slightly different motion.
        assert!(
            after_f2 <= after_f1 + 1,
            "groups should be refreshed, not re-created (f1={after_f1}, f2={after_f2})"
        );
    }

    #[test]
    fn no_groups_form_below_cohort_minimum() {
        // A single small disk yields ≤ 2 patches (interior + a few edge
        // pixels). Below GROUP_FORM_MIN_NODES → no group spawned in the
        // spatial bucket. (The temporal-window region also won't spawn one
        // because the cohort < MIN.)
        let mut frame = CameraFrame::zeros(0);
        for y in 0..CAM_H {
            for x in 0..CAM_W {
                let dx = x as f64 - 160.0;
                let dy = y as f64 - 120.0;
                if dx * dx + dy * dy <= 36.0 {
                    let idx = y * CAM_W + x;
                    frame.luma[idx] = 0.9;
                    frame.r[idx] = 0.9;
                    frame.g[idx] = 0.9;
                    frame.b[idx] = 0.9;
                }
            }
        }
        let mut v = VisionTier0::new();
        let mut g = PerturbationGraph::default();
        v.process_frame_into_graph(&frame, 1_000, &mut g, None, None);
        // Most frames produce a few patches even from one disk (edge ring).
        // If that count happens to clear the threshold, the test would spawn
        // a group; below threshold none. Assert the conditional behaviour by
        // counting how many active patches there were vs how many groups.
        let n_active = g.nodes.len();
        if n_active < GROUP_FORM_MIN_NODES {
            assert_eq!(g.groups.len(), 0, "no group below cohort minimum");
        }
    }

    #[test]
    fn temporal_window_groups_share_across_frames_in_same_bucket() {
        // Two frames with very similar content within the same temporal
        // window (now_us / GROUP_TEMPORAL_WINDOW_US is the same bucket) →
        // one shared TemporalWindow group, not two.
        let frame = {
            let mut f = CameraFrame::zeros(0);
            for &(cx, cy) in &[(100.0_f64, 100.0), (200.0, 100.0), (160.0, 200.0)] {
                for y in 0..CAM_H {
                    for x in 0..CAM_W {
                        let dx = x as f64 - cx;
                        let dy = y as f64 - cy;
                        if dx * dx + dy * dy <= 144.0 {
                            let idx = y * CAM_W + x;
                            f.luma[idx] = 0.8;
                            f.r[idx] = 0.8;
                            f.g[idx] = 0.4;
                            f.b[idx] = 0.8;
                        }
                    }
                }
            }
            f
        };
        let mut v = VisionTier0::new();
        let mut g = PerturbationGraph::default();
        // Two frames at t=10_000 and t=50_000 — same temporal bucket
        // because GROUP_TEMPORAL_WINDOW_US = 1_000_000.
        v.process_frame_into_graph(&frame, 10_000, &mut g, None, None);
        v.process_frame_into_graph(&frame, 50_000, &mut g, None, None);
        let temporal_groups: Vec<_> = g
            .groups
            .iter()
            .filter(|gp| matches!(gp.region, RegionKey::TemporalWindow(_)))
            .collect();
        assert_eq!(
            temporal_groups.len(),
            1,
            "two ticks in same temporal bucket must share one group"
        );
    }

    #[test]
    fn spatial_bucket_quantisation_keeps_jitter_in_same_cell() {
        // pos_x ∈ [0.0625*k, 0.0625*(k+1)) all bucket to k. Confirm via the
        // function via constants: (0.05 * 16) as u8 = 0; (0.1 * 16) as u8 = 1.
        // This pins the bucketing convention before any consumer depends on it.
        let bx0 = (0.05_f64 * GROUP_SPATIAL_BUCKETS as f64) as u8;
        let bx1 = (0.10_f64 * GROUP_SPATIAL_BUCKETS as f64) as u8;
        assert_eq!(bx0, 0);
        assert_eq!(bx1, 1);
        let bx_edge = (0.999_f64 * GROUP_SPATIAL_BUCKETS as f64) as u8;
        assert_eq!(
            bx_edge.min(GROUP_SPATIAL_BUCKETS - 1),
            GROUP_SPATIAL_BUCKETS - 1
        );
    }
}
