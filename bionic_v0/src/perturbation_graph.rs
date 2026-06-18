//! perturbation_graph — Phase 3: tier-0 perturbation graph that replaces the
//! slot architecture (familiar_objects + object_high_layer + SLOT_COUNT=8).
//!
//! # Why this exists
//!
//! The 8-slot categorical architecture forced every perceived "thing" through
//! a fixed taxonomy, capping the representable world at 8 objects and reducing
//! affordance to a 4-D fingerprint. The world is not 8 categorical slots; it
//! is a fuzzy nested web of perturbation patches (computer ⊃ keyboard ⊃ key)
//! whose meaning is RELATIONAL.
//!
//! This module replaces that with a SPARSE GRAPH of perturbation patches:
//!   - **Nodes** hold tier-0 multi-axis statistics (signature + marginal stats).
//!     Affordances ("can move", "can sound", "friendly") EMERGE from those
//!     statistics as marginal correlations — they are not labelled enums.
//!   - **Edges** hold relations (co-occurrence, nesting, causal precedence,
//!     cross-modal coupling). Object boundaries are implicit in the edge web,
//!     not hardcoded by slot index.
//!   - **Capacity is SOFT**: caps exist as engineering bounds, but eviction
//!     is by bind_strength × recency × age-protection — no fixed N-slot rule.
//!   - **Nodes are CONTINUOUSLY updated while active** (live-neuron semantics):
//!     the truth of a node FLOATS while it is in view (or in mind), instead of
//!     being frozen on first sight.
//!
//! # Stage rollout
//!
//! Stage 0 (this file): data structures + persistence + merge + tests. No
//! main-loop wiring yet — runs alongside the old slot stack until Stage 1
//! plugs the new visual frontend into it.
//!
//! Stage 1: minimal visual frontend (edges + flow + connected components) emits
//! tier-0 patches into this graph.
//!
//! Stage 2: prediction/cross-layer/drive systems consume the graph; old
//! familiar_objects deleted.
//!
//! # Persistence model
//!
//! Sidecar `perturbation_graph.bin` (NOT the global snapshot). Its own
//! magic+version so a schema bump never resets every lineage's maturity
//! (mirrors the proven vocal_sequence.bin / vocal_target_recall.bin pattern).
//!
//! Numeric/local-learning only. No backprop, no labels, no global gradients —
//! respects the emergence contract.

/// Tier-0 signature dimensionality per node. Carries the "what this perturbation
/// LOOKS like" axes that distinguish one patch from another (HSV statistics,
/// gradient histogram, motion summary). The exact axis assignment is the
/// visual/auditory frontend's job — this module just carries the bytes.
///
/// Picked at 24 because that's roughly the middle of the user's 20–30 target:
/// HSV mean+var (6) + 4-bin gradient histogram (4) + spatial (4) + scale (2)
/// + persistence (1) + motion summary (4) + headroom (3). Reserved axes left
/// for the frontend to slot in later (zero is a safe default).
pub const PG_NODE_SIG_DIM: usize = 24;

/// Cross-modal correlation slots. At minimum: per-vocal-band coupling so a
/// node that reliably co-vibrates with our voice grows a "can sound" affordance
/// as a marginal statistic, not a label. 8 is enough headroom for the vocal
/// mel summary; additional modalities can co-exist via signature axes.
pub const PG_CROSS_MODAL_DIM: usize = 8;

/// Vital coupling slots: correlation with interoceptive axes (attachment,
/// anxiety, agency, pain, presence, learning_progress). The substrate from
/// which "friendly/dangerous" affordances emerge as marginal statistics.
pub const PG_VITAL_DIM: usize = 6;

/// Soft cap on lifetime nodes. Way larger than SLOT_COUNT=8 — the whole point
/// of the cutover is that the world is bigger than 8 things. 8192 was chosen
/// after the first 5-minute real visitor call filled 1545 nodes (~5 min × ~5
/// nodes/s), with the prior 2048 already hitting eviction at ~3 min: 8192
/// gives several hours of headroom before the first eviction. Eviction is
/// age-protected so rare-but-old nodes survive a burst of new ones.
pub const PG_NODE_CAP: usize = 8192;

/// Soft cap on edges. The 5-minute visitor data shows the previous 16384 cap
/// saturates after ~3 min — the cooccurrence layer between every pair of
/// in-frame patches is dense (TIER0_MAX_PATCHES_PER_FRAME² edges per tick).
/// 65536 gives ~4× headroom and keeps the per-call edge eviction infrequent
/// enough that the strongest relations stabilise before being squeezed out.
pub const PG_EDGE_CAP: usize = 65536;

/// Window after `last_active_t_us` during which a node is considered "live"
/// and keeps getting tier-0 statistic updates (the active-neuron semantics).
/// 2 s is the typical persistence of an in-view patch across several frames.
pub const PG_ACTIVE_WINDOW_US: u64 = 2_000_000;

/// Eligibility-trace time constant for edge credit. Sub-second so a short
/// chain of co-occurrences can propagate credit a few hops upstream.
pub const PG_ELIG_TAU_US: f64 = 600_000.0;

/// Age-protection window — a node that has survived this long earns the full
/// `PG_AGE_PROTECT_MAX_BONUS` to its eviction-defence score. 6 h ≈ a sleep
/// boundary; long enough to span sessions, short enough that genuinely cold
/// nodes still age out.
pub const PG_AGE_PROTECT_US: f64 = 6.0 * 3600.0 * 1_000_000.0;
const PG_AGE_PROTECT_MAX_BONUS: f64 = 0.30;

/// EMA seed for a newly-observed edge so a fresh edge is NOT zero (avoids the
/// 0×x bootstrap deadlock that would prevent any later multiplicative biasing
/// from EVER kicking in for that edge). Mirrors SEQ_COUNT_ALPHA in
/// vocal_sequence.
const PG_EDGE_SEED: f64 = 0.2;

/// 1-minute idle decay constant used in the eviction-defence calculation. A
/// node that has been silent for ~60 s loses most of its recency bonus.
const PG_IDLE_DECAY_US: f64 = 60_000_000.0;

/// Stable per-session node identifier. Edges reference nodes by id, NOT vector
/// index, so eviction can move nodes around without silently breaking edges.
/// Id 0 is the "no-node" sentinel — never assigned by `add_node`.
pub type NodeId = u32;

/// Edge kinds. Discriminants are stable for persistence.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// Both nodes active in the same time window.
    Cooccur = 0,
    /// One node's bounds inside the other's (computer ⊃ keyboard).
    Nest = 1,
    /// A's change reliably preceded B's change.
    Causal = 2,
    /// Co-active across modalities (visual patch ↔ vocal signature).
    CrossModal = 3,
}

impl EdgeKind {
    pub fn from_u8(b: u8) -> EdgeKind {
        match b {
            1 => EdgeKind::Nest,
            2 => EdgeKind::Causal,
            3 => EdgeKind::CrossModal,
            // Defensive: unknown bytes fall back to the most generic relation.
            _ => EdgeKind::Cooccur,
        }
    }
    pub fn as_u8(self) -> u8 {
        self as u8
    }
}

/// A perturbation patch in the world. Tier-0 statistics + marginal stats; no
/// labels, no slot index, no semantic type. Affordances are READINGS of the
/// marginal stats — they are not stored.
#[derive(Clone, Debug, PartialEq)]
pub struct PerturbationNode {
    pub id: NodeId,
    /// Tier-0 identity signature: what THIS perturbation looks like.
    pub sig: [f64; PG_NODE_SIG_DIM],
    /// Cross-modal correlations (e.g. vocal mel coupling). "Can sound" is the
    /// reading: cross_modal.iter().map(abs).sum() being non-trivially large.
    pub cross_modal: [f64; PG_CROSS_MODAL_DIM],
    /// Vital coupling: how this node's presence covaries with interoceptive
    /// axes. "Friendly" / "dangerous" are READINGS of the sign + magnitude on
    /// the relevant axes (attachment positive, anxiety negative → friendly).
    pub vital_coupling: [f64; PG_VITAL_DIM],
    /// Variance of position over time. "Can move" reading.
    pub motion_var: f64,
    /// EMA of the patch's spatial extent.
    pub size_ema: f64,
    /// Fraction of recent ticks this node was active (in [0,1]).
    pub persistence_ema: f64,
    /// Count of "this node's change preceded another's". "Has force" reading.
    pub causal_outflow: f64,
    /// Overall reliability score in [0,1]. Drives eviction defence + downstream
    /// "is this real" gating. Rises with continued evidence (saturating EMA).
    pub bind_strength: f64,
    /// Born timestamp (for age protection).
    pub age_us: u64,
    /// Last frame this node was perceived.
    pub last_active_t_us: u64,
}

/// A directed, typed relation between two nodes. Multiple kinds may co-exist
/// for the same (from,to) pair (e.g. both Cooccur and Causal) — they are
/// distinct edges.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PerturbationEdge {
    pub from: NodeId,
    pub to: NodeId,
    /// EdgeKind discriminant (use EdgeKind::from_u8).
    pub kind_byte: u8,
    /// EMA of co-occurrence/co-activation strength in [0,1].
    pub strength: f64,
    /// Eligibility trace: 1.0 right after traversal, decays with PG_ELIG_TAU_US.
    pub elig: f64,
    pub last_t_us: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PerturbationGraph {
    pub nodes: Vec<PerturbationNode>,
    pub edges: Vec<PerturbationEdge>,
    /// Next id to hand out — never resets to 0 (the sentinel) and never reuses
    /// an evicted node's id within the same session.
    pub next_id: u32,
}

impl Default for PerturbationGraph {
    fn default() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            next_id: 1, // 0 reserved as the "no-node" sentinel
        }
    }
}

impl PerturbationGraph {
    // ── Node ops ─────────────────────────────────────────────────────────────

    /// Spawn a new node with the given signature and current timestamp. Returns
    /// the new node's id. If the lifetime cap is hit, the weakest existing node
    /// is evicted first (and any edges referencing it are dropped).
    pub fn add_node(
        &mut self,
        now_us: u64,
        sig: [f64; PG_NODE_SIG_DIM],
    ) -> NodeId {
        if self.nodes.len() >= PG_NODE_CAP {
            self.evict_weakest_node(now_us);
        }
        let id = self.alloc_id();
        self.nodes.push(PerturbationNode {
            id,
            sig,
            cross_modal: [0.0; PG_CROSS_MODAL_DIM],
            vital_coupling: [0.0; PG_VITAL_DIM],
            motion_var: 0.0,
            size_ema: 0.0,
            persistence_ema: 0.0,
            causal_outflow: 0.0,
            bind_strength: PG_EDGE_SEED, // bootstrap-deadlock guard
            age_us: now_us,
            last_active_t_us: now_us,
        });
        id
    }

    fn alloc_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        if self.next_id == 0 {
            self.next_id = 1; // never roll back to the sentinel
        }
        id
    }

    /// Find an existing node whose signature is within `max_dist` of `sig`.
    /// Returns the closest match (lowest distance).
    pub fn nearest_node(
        &self,
        sig: &[f64; PG_NODE_SIG_DIM],
        max_dist: f64,
    ) -> Option<(NodeId, f64)> {
        let mut best: Option<(NodeId, f64)> = None;
        for n in &self.nodes {
            let d = sig_distance(&n.sig, sig);
            if d <= max_dist && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((n.id, d));
            }
        }
        best
    }

    pub fn node(&self, id: NodeId) -> Option<&PerturbationNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: NodeId) -> Option<&mut PerturbationNode> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn has_node(&self, id: NodeId) -> bool {
        self.nodes.iter().any(|n| n.id == id)
    }

    /// Mark a node as still-perceived this tick. EMA-blend the incoming
    /// signature into its stored signature so the node's truth FLOATS (live
    /// neuron) instead of being frozen on first sight. Renews `last_active_t_us`
    /// (and so the eviction defence) and accumulates bind_strength toward 1.0.
    pub fn refresh_node(
        &mut self,
        id: NodeId,
        now_us: u64,
        sig: &[f64; PG_NODE_SIG_DIM],
        alpha: f64,
    ) {
        let alpha = alpha.clamp(0.0, 1.0);
        if let Some(n) = self.node_mut(id) {
            for i in 0..PG_NODE_SIG_DIM {
                n.sig[i] = (1.0 - alpha) * n.sig[i] + alpha * sig[i];
            }
            n.last_active_t_us = now_us;
            // bind_strength rises with continued evidence (saturating).
            n.bind_strength =
                (n.bind_strength + alpha * (1.0 - n.bind_strength)).clamp(0.0, 1.0);
        }
    }

    /// EMA-blend a cross-modal correlation reading into a node. Caller is
    /// responsible for picking `alpha` (smaller = slower accumulation).
    pub fn update_cross_modal(
        &mut self,
        id: NodeId,
        readings: &[f64; PG_CROSS_MODAL_DIM],
        alpha: f64,
    ) {
        let a = alpha.clamp(0.0, 1.0);
        if let Some(n) = self.node_mut(id) {
            for i in 0..PG_CROSS_MODAL_DIM {
                n.cross_modal[i] = (1.0 - a) * n.cross_modal[i] + a * readings[i];
            }
        }
    }

    /// EMA-blend a vital coupling reading into a node.
    pub fn update_vital_coupling(
        &mut self,
        id: NodeId,
        readings: &[f64; PG_VITAL_DIM],
        alpha: f64,
    ) {
        let a = alpha.clamp(0.0, 1.0);
        if let Some(n) = self.node_mut(id) {
            for i in 0..PG_VITAL_DIM {
                n.vital_coupling[i] = (1.0 - a) * n.vital_coupling[i] + a * readings[i];
            }
        }
    }

    /// True if the node has been refreshed within `PG_ACTIVE_WINDOW_US`.
    pub fn is_active(&self, id: NodeId, now_us: u64) -> bool {
        self.node(id)
            .map(|n| now_us.saturating_sub(n.last_active_t_us) <= PG_ACTIVE_WINDOW_US)
            .unwrap_or(false)
    }

    /// Iterator over currently-active node ids.
    pub fn active_ids<'a>(&'a self, now_us: u64) -> impl Iterator<Item = NodeId> + 'a {
        self.nodes
            .iter()
            .filter(move |n| {
                now_us.saturating_sub(n.last_active_t_us) <= PG_ACTIVE_WINDOW_US
            })
            .map(|n| n.id)
    }

    /// Eviction defence score. Higher = more protected. Combines accumulated
    /// reliability, age protection, and a recency component.
    fn eviction_defence(node: &PerturbationNode, now_us: u64) -> f64 {
        let age = now_us.saturating_sub(node.age_us) as f64;
        let age_bonus = (age / PG_AGE_PROTECT_US).min(1.0) * PG_AGE_PROTECT_MAX_BONUS;
        let idle = now_us.saturating_sub(node.last_active_t_us) as f64;
        let recency_penalty = 1.0 - (-idle / PG_IDLE_DECAY_US).exp(); // [0, 1]
        node.bind_strength + age_bonus - 0.1 * recency_penalty
    }

    fn evict_weakest_node(&mut self, now_us: u64) {
        let mut weakest_idx = 0usize;
        let mut weakest_def = f64::INFINITY;
        for (i, n) in self.nodes.iter().enumerate() {
            let d = Self::eviction_defence(n, now_us);
            if d < weakest_def {
                weakest_def = d;
                weakest_idx = i;
            }
        }
        if self.nodes.is_empty() {
            return;
        }
        let evicted_id = self.nodes[weakest_idx].id;
        self.nodes.swap_remove(weakest_idx);
        // Drop edges referencing the evicted node — otherwise they would
        // silently point at a non-existent id and inflate edge count.
        self.edges.retain(|e| e.from != evicted_id && e.to != evicted_id);
    }

    // ── Edge ops ─────────────────────────────────────────────────────────────

    /// Bump the (from→to, kind) edge: create if missing, EMA the strength,
    /// reset eligibility. No-op if from==to (no self-loops), either id is
    /// the sentinel 0, or either id is unknown.
    pub fn observe_edge(
        &mut self,
        now_us: u64,
        from: NodeId,
        to: NodeId,
        kind: EdgeKind,
    ) {
        if from == to || from == 0 || to == 0 {
            return;
        }
        if !self.has_node(from) || !self.has_node(to) {
            return;
        }
        let k = kind.as_u8();
        if let Some(e) = self
            .edges
            .iter_mut()
            .find(|e| e.from == from && e.to == to && e.kind_byte == k)
        {
            e.strength =
                (e.strength + PG_EDGE_SEED * (1.0 - e.strength)).clamp(0.0, 1.0);
            e.elig = 1.0;
            e.last_t_us = now_us;
        } else {
            if self.edges.len() >= PG_EDGE_CAP {
                self.evict_weakest_edge();
            }
            self.edges.push(PerturbationEdge {
                from,
                to,
                kind_byte: k,
                strength: PG_EDGE_SEED,
                elig: 1.0,
                last_t_us: now_us,
            });
        }
    }

    fn evict_weakest_edge(&mut self) {
        if let Some((idx, _)) = self
            .edges
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| a.strength.partial_cmp(&b.strength).unwrap())
        {
            self.edges.swap_remove(idx);
        }
    }

    /// Decay edge eligibility traces (pure function of time-since-last-traversal).
    /// Idempotent — safe to call every tick.
    pub fn decay_eligibility(&mut self, now_us: u64) {
        for e in self.edges.iter_mut() {
            let dt = now_us.saturating_sub(e.last_t_us) as f64;
            e.elig = (-dt / PG_ELIG_TAU_US).exp();
        }
    }

    /// Outgoing edges of a node (read-only iterator).
    pub fn out_edges<'a>(
        &'a self,
        from: NodeId,
    ) -> impl Iterator<Item = &'a PerturbationEdge> + 'a {
        self.edges.iter().filter(move |e| e.from == from)
    }

    /// The strongest outgoing edge of a node (for predict_next-style queries).
    pub fn strongest_out(&self, from: NodeId) -> Option<&PerturbationEdge> {
        self.out_edges(from)
            .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap())
    }

    /// The strongest outgoing edge FILTERED by a particular kind.
    pub fn strongest_out_kind(
        &self,
        from: NodeId,
        kind: EdgeKind,
    ) -> Option<&PerturbationEdge> {
        let k = kind.as_u8();
        self.edges
            .iter()
            .filter(|e| e.from == from && e.kind_byte == k)
            .max_by(|a, b| a.strength.partial_cmp(&b.strength).unwrap())
    }

    /// The node with the highest bind_strength: the closest analog of the old
    /// "most-familiar object" / "responsible person" emergent reading. None
    /// until something has accumulated enough evidence to stand out.
    pub fn most_bound(&self) -> Option<&PerturbationNode> {
        self.nodes
            .iter()
            .max_by(|a, b| a.bind_strength.partial_cmp(&b.bind_strength).unwrap())
    }

    // ── Distributed-training merge ───────────────────────────────────────────

    /// Fold another life's graph into this one. Node IDs are session-relative,
    /// so we match across lineages by signature DISTANCE (≤ `merge_dist`), EMA-
    /// blending matched-node statistics (favouring the local lineage 0.7/0.3).
    /// Unmatched other-nodes are imported under FRESH local IDs; their edges
    /// are remapped accordingly. Mirrors the structure-aware merge of
    /// VocalTargetRecall / VocalSequence.
    pub fn merge(&mut self, other: &PerturbationGraph, merge_dist: f64) {
        let mut id_map: std::collections::HashMap<NodeId, NodeId> =
            std::collections::HashMap::with_capacity(other.nodes.len());
        for on in &other.nodes {
            if let Some((local_id, _)) = self.nearest_node(&on.sig, merge_dist) {
                id_map.insert(on.id, local_id);
                if let Some(n) = self.node_mut(local_id) {
                    // EMA blend, favouring the local lineage's accumulated state.
                    for i in 0..PG_NODE_SIG_DIM {
                        n.sig[i] = 0.7 * n.sig[i] + 0.3 * on.sig[i];
                    }
                    for i in 0..PG_CROSS_MODAL_DIM {
                        n.cross_modal[i] =
                            0.7 * n.cross_modal[i] + 0.3 * on.cross_modal[i];
                    }
                    for i in 0..PG_VITAL_DIM {
                        n.vital_coupling[i] =
                            0.7 * n.vital_coupling[i] + 0.3 * on.vital_coupling[i];
                    }
                    n.motion_var = 0.7 * n.motion_var + 0.3 * on.motion_var;
                    n.size_ema = 0.7 * n.size_ema + 0.3 * on.size_ema;
                    n.persistence_ema =
                        0.7 * n.persistence_ema + 0.3 * on.persistence_ema;
                    n.causal_outflow =
                        0.7 * n.causal_outflow + 0.3 * on.causal_outflow;
                    // bind_strength: take the max — evidence is monotonic, not
                    // averagable. The maturer of the two wins.
                    n.bind_strength = n.bind_strength.max(on.bind_strength);
                    n.last_active_t_us = n.last_active_t_us.max(on.last_active_t_us);
                }
            } else if self.nodes.len() < PG_NODE_CAP {
                // Novel node: import under fresh local id. Don't EMA-blend marginal
                // stats here — they only matter as ACCUMULATED evidence; halving
                // them on import would silently penalise novel knowledge.
                let now_us = on.last_active_t_us.max(on.age_us);
                let new_id = self.add_node(now_us, on.sig);
                if let Some(n) = self.node_mut(new_id) {
                    n.cross_modal = on.cross_modal;
                    n.vital_coupling = on.vital_coupling;
                    n.motion_var = on.motion_var;
                    n.size_ema = on.size_ema;
                    n.persistence_ema = on.persistence_ema;
                    n.causal_outflow = on.causal_outflow;
                    n.bind_strength = on.bind_strength;
                    n.age_us = on.age_us;
                }
                id_map.insert(on.id, new_id);
            }
            // else: lifetime cap reached, no room — silently skip. Caller can
            // bump PG_NODE_CAP if this fires under expected workloads.
        }
        // Second pass: import edges, remapping endpoints; keep the stronger
        // edge per (from,to,kind), union novel ones (respecting the cap).
        for oe in &other.edges {
            let lf = match id_map.get(&oe.from) {
                Some(&v) => v,
                None => continue, // endpoint wasn't imported (cap-skipped)
            };
            let lt = match id_map.get(&oe.to) {
                Some(&v) => v,
                None => continue,
            };
            if let Some(e) = self.edges.iter_mut().find(|e| {
                e.from == lf && e.to == lt && e.kind_byte == oe.kind_byte
            }) {
                if oe.strength > e.strength {
                    e.strength = oe.strength;
                    e.last_t_us = e.last_t_us.max(oe.last_t_us);
                }
            } else if self.edges.len() < PG_EDGE_CAP {
                self.edges.push(PerturbationEdge {
                    from: lf,
                    to: lt,
                    kind_byte: oe.kind_byte,
                    strength: oe.strength,
                    elig: 0.0,
                    last_t_us: oe.last_t_us,
                });
            } else {
                // Cap reached: only displace our weakest edge if the incoming
                // is stronger (mirrors VocalSequence::merge).
                if let Some((idx, ws)) = self
                    .edges
                    .iter()
                    .enumerate()
                    .map(|(i, e)| (i, e.strength))
                    .min_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                {
                    if ws < oe.strength {
                        self.edges.swap_remove(idx);
                        self.edges.push(PerturbationEdge {
                            from: lf,
                            to: lt,
                            kind_byte: oe.kind_byte,
                            strength: oe.strength,
                            elig: 0.0,
                            last_t_us: oe.last_t_us,
                        });
                    }
                }
            }
        }
    }

    // ── Persistence ──────────────────────────────────────────────────────────
    //
    // Sidecar `perturbation_graph.bin` — own magic+version. Bumping the global
    // SNAPSHOT_VERSION would nuke every lineage's maturity; this file stays on
    // its own version line (mirrors vocal_sequence.bin / vocal_target_recall.bin).
    //
    // Layout v1:
    //   [4]  magic b"PGRA"
    //   [1]  version=0x01
    //   [4]  next_id u32-le
    //   [8]  n_nodes u64-le
    //   per node: NODE_BYTES (fixed stride — JS parsers can rely on it)
    //   [8]  n_edges u64-le
    //   per edge: EDGE_BYTES (fixed stride)
    //
    // The fixed-stride layout is intentional: gateway/training_pool.mjs once
    // silently misread familiar_objects.bin because two language-distinct
    // parsers disagreed on stride (52 vs 84). Keep the strides simple and
    // pinned via the constants below.

    const MAGIC: [u8; 4] = *b"PGRA";
    const VERSION: u8 = 0x01;

    /// Bytes per node on disk. f64 = 8 bytes; u32 = 4; u64 = 8.
    const NODE_BYTES: usize = 4 // id u32
        + 8 * PG_NODE_SIG_DIM
        + 8 * PG_CROSS_MODAL_DIM
        + 8 * PG_VITAL_DIM
        + 8 * 4 // motion_var, size_ema, persistence_ema, causal_outflow
        + 8 // bind_strength
        + 8 // age_us
        + 8; // last_active_t_us

    /// Bytes per edge on disk: from u32 + to u32 + kind u8 + strength f64 +
    /// elig f64 + last_t_us u64 = 33.
    const EDGE_BYTES: usize = 4 + 4 + 1 + 8 + 8 + 8;

    pub fn serialize(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(
            4 + 1
                + 4
                + 8
                + self.nodes.len() * Self::NODE_BYTES
                + 8
                + self.edges.len() * Self::EDGE_BYTES,
        );
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::VERSION);
        out.extend_from_slice(&self.next_id.to_le_bytes());
        out.extend_from_slice(&(self.nodes.len() as u64).to_le_bytes());
        for n in &self.nodes {
            out.extend_from_slice(&n.id.to_le_bytes());
            for v in &n.sig {
                out.extend_from_slice(&v.to_le_bytes());
            }
            for v in &n.cross_modal {
                out.extend_from_slice(&v.to_le_bytes());
            }
            for v in &n.vital_coupling {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&n.motion_var.to_le_bytes());
            out.extend_from_slice(&n.size_ema.to_le_bytes());
            out.extend_from_slice(&n.persistence_ema.to_le_bytes());
            out.extend_from_slice(&n.causal_outflow.to_le_bytes());
            out.extend_from_slice(&n.bind_strength.to_le_bytes());
            out.extend_from_slice(&n.age_us.to_le_bytes());
            out.extend_from_slice(&n.last_active_t_us.to_le_bytes());
        }
        out.extend_from_slice(&(self.edges.len() as u64).to_le_bytes());
        for e in &self.edges {
            out.extend_from_slice(&e.from.to_le_bytes());
            out.extend_from_slice(&e.to.to_le_bytes());
            out.push(e.kind_byte);
            out.extend_from_slice(&e.strength.to_le_bytes());
            out.extend_from_slice(&e.elig.to_le_bytes());
            out.extend_from_slice(&e.last_t_us.to_le_bytes());
        }
        out
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        // Header: magic(4) + version(1) + next_id(4) + n_nodes(8) = 17
        if data.len() < 17 || data[0..4] != Self::MAGIC {
            return None;
        }
        if data[4] != Self::VERSION {
            return None;
        }
        let mut off = 5usize;
        let next_id = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
        off += 4;
        let n_nodes = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
        off += 8;
        if n_nodes > PG_NODE_CAP {
            return None;
        }
        // Need at least the node block plus the n_edges u64.
        if data.len() < off + n_nodes * Self::NODE_BYTES + 8 {
            return None;
        }
        let mut nodes = Vec::with_capacity(n_nodes);
        for _ in 0..n_nodes {
            let id = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
            off += 4;
            let mut sig = [0.0f64; PG_NODE_SIG_DIM];
            for v in sig.iter_mut() {
                *v = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
            }
            let mut cross_modal = [0.0f64; PG_CROSS_MODAL_DIM];
            for v in cross_modal.iter_mut() {
                *v = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
            }
            let mut vital_coupling = [0.0f64; PG_VITAL_DIM];
            for v in vital_coupling.iter_mut() {
                *v = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
            }
            let motion_var = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let size_ema = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let persistence_ema =
                f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let causal_outflow =
                f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let bind_strength =
                f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let age_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let last_active_t_us =
                u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            nodes.push(PerturbationNode {
                id,
                sig,
                cross_modal,
                vital_coupling,
                motion_var,
                size_ema,
                persistence_ema,
                causal_outflow,
                bind_strength,
                age_us,
                last_active_t_us,
            });
        }
        let n_edges = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
        off += 8;
        if n_edges > PG_EDGE_CAP {
            return None;
        }
        if data.len() < off + n_edges * Self::EDGE_BYTES {
            return None;
        }
        let mut edges = Vec::with_capacity(n_edges);
        for _ in 0..n_edges {
            let from = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
            off += 4;
            let to = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
            off += 4;
            let kind_byte = data[off];
            off += 1;
            let strength = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let elig = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            let last_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
            off += 8;
            edges.push(PerturbationEdge {
                from,
                to,
                kind_byte,
                strength,
                elig,
                last_t_us,
            });
        }
        Some(PerturbationGraph {
            nodes,
            edges,
            next_id,
        })
    }
}

/// Euclidean distance between two signatures.
pub fn sig_distance(
    a: &[f64; PG_NODE_SIG_DIM],
    b: &[f64; PG_NODE_SIG_DIM],
) -> f64 {
    let mut s = 0.0;
    for i in 0..PG_NODE_SIG_DIM {
        let d = a[i] - b[i];
        s += d * d;
    }
    s.sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sig_at(v: f64) -> [f64; PG_NODE_SIG_DIM] {
        let mut s = [0.0; PG_NODE_SIG_DIM];
        for (i, x) in s.iter_mut().enumerate() {
            *x = v + 0.001 * i as f64;
        }
        s
    }

    #[test]
    fn add_node_returns_unique_ids_starting_above_sentinel() {
        let mut g = PerturbationGraph::default();
        let id_a = g.add_node(1_000, sig_at(0.1));
        let id_b = g.add_node(2_000, sig_at(0.5));
        assert_ne!(id_a, id_b);
        assert!(id_a >= 1, "id 0 is the no-node sentinel and must never be handed out");
        assert!(id_b >= 1);
        assert_eq!(g.nodes.len(), 2);
    }

    #[test]
    fn nearest_node_finds_within_distance() {
        let mut g = PerturbationGraph::default();
        let id_a = g.add_node(1_000, sig_at(0.1));
        let _id_b = g.add_node(2_000, sig_at(0.5));
        let (near, _) = g
            .nearest_node(&sig_at(0.105), 0.1)
            .expect("the close match exists");
        assert_eq!(near, id_a);
        assert!(
            g.nearest_node(&sig_at(10.0), 0.1).is_none(),
            "an out-of-range signature must not match"
        );
    }

    #[test]
    fn refresh_node_blends_signature_renews_timestamp_and_strengthens_bind() {
        let mut g = PerturbationGraph::default();
        let id = g.add_node(1_000, sig_at(0.0));
        let before_bind = g.node(id).unwrap().bind_strength;
        g.refresh_node(id, 50_000, &sig_at(1.0), 0.5);
        let n = g.node(id).unwrap();
        // alpha=0.5 → 50/50 blend on axis 0 of 0.0 and 1.0.
        assert!((n.sig[0] - 0.5).abs() < 1e-9);
        assert_eq!(n.last_active_t_us, 50_000);
        assert!(n.bind_strength > before_bind);
        assert!(n.bind_strength <= 1.0);
    }

    #[test]
    fn observe_edge_seeds_and_strengthens_same_kind() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.0));
        let b = g.add_node(0, sig_at(0.5));
        g.observe_edge(1_000, a, b, EdgeKind::Cooccur);
        let s1 = g.edges[0].strength;
        g.observe_edge(2_000, a, b, EdgeKind::Cooccur);
        let s2 = g.edges[0].strength;
        assert!(s2 > s1, "repeated observation must strengthen the edge");
        assert_eq!(g.edges.len(), 1, "same (from,to,kind) must not duplicate");
    }

    #[test]
    fn observe_edge_distinguishes_kinds() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.0));
        let b = g.add_node(0, sig_at(0.5));
        g.observe_edge(1_000, a, b, EdgeKind::Cooccur);
        g.observe_edge(1_000, a, b, EdgeKind::Causal);
        assert_eq!(g.edges.len(), 2, "different kinds = different edges");
    }

    #[test]
    fn no_self_loops_and_no_sentinel_endpoints() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.0));
        g.observe_edge(1_000, a, a, EdgeKind::Cooccur);
        assert!(g.edges.is_empty(), "self-loops must be rejected");
        g.observe_edge(1_000, 0, a, EdgeKind::Cooccur);
        g.observe_edge(1_000, a, 0, EdgeKind::Cooccur);
        assert!(g.edges.is_empty(), "id 0 (sentinel) must be rejected");
        g.observe_edge(1_000, a, 9999, EdgeKind::Cooccur);
        assert!(g.edges.is_empty(), "unknown endpoint must be rejected");
    }

    #[test]
    fn most_bound_wins_after_repeated_refresh() {
        let mut g = PerturbationGraph::default();
        let caregiver = g.add_node(0, sig_at(0.1));
        let _stranger = g.add_node(0, sig_at(0.5));
        for k in 0..20 {
            g.refresh_node(caregiver, 1000 * k, &sig_at(0.1), 0.2);
        }
        let top = g.most_bound().expect("someone is most bound");
        assert_eq!(top.id, caregiver, "the repeatedly-refreshed node emerges as most bound");
    }

    #[test]
    fn active_window_filters_dormant_nodes() {
        let mut g = PerturbationGraph::default();
        let live = g.add_node(0, sig_at(0.0));
        let dormant = g.add_node(0, sig_at(0.5));
        // Bring `live` forward in time; `dormant` keeps last_active_t_us at 0.
        let now = PG_ACTIVE_WINDOW_US * 2;
        g.refresh_node(live, now, &sig_at(0.0), 0.2);
        let active: Vec<NodeId> = g.active_ids(now).collect();
        assert!(active.contains(&live), "recently-refreshed node must be active");
        assert!(!active.contains(&dormant), "stale node must NOT be active");
        assert!(g.is_active(live, now));
        assert!(!g.is_active(dormant, now));
    }

    #[test]
    fn evict_weakest_node_protects_higher_bind_strength() {
        let mut g = PerturbationGraph::default();
        let weak = g.add_node(0, sig_at(0.0));
        let strong = g.add_node(0, sig_at(0.5));
        // Strongly accumulate on `strong` so it survives the eviction.
        for k in 0..30 {
            g.refresh_node(strong, 1000 * k, &sig_at(0.5), 0.3);
        }
        let now = 100_000;
        g.evict_weakest_node(now);
        assert!(g.has_node(strong), "high-bind node must survive eviction");
        assert!(!g.has_node(weak), "low-bind node must be evicted");
    }

    #[test]
    fn evicting_a_node_drops_its_edges() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.0));
        let b = g.add_node(0, sig_at(0.5));
        let c = g.add_node(0, sig_at(0.9));
        // Build edges involving `a`.
        g.observe_edge(1_000, a, b, EdgeKind::Cooccur);
        g.observe_edge(1_000, a, c, EdgeKind::Causal);
        // And one not involving `a`.
        g.observe_edge(1_000, b, c, EdgeKind::Cooccur);
        assert_eq!(g.edges.len(), 3);
        // Raise b and c's bind so `a` becomes weakest.
        for k in 0..30 {
            g.refresh_node(b, 1000 * k, &sig_at(0.5), 0.3);
            g.refresh_node(c, 1000 * k, &sig_at(0.9), 0.3);
        }
        g.evict_weakest_node(100_000);
        assert!(!g.has_node(a), "weakest evicted");
        assert!(g.edges.iter().all(|e| e.from != a && e.to != a),
            "evicting a node must drop ALL edges referencing it");
        assert_eq!(g.edges.len(), 1, "only b→c survives");
    }

    #[test]
    fn decay_eligibility_drops_old_edges() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.0));
        let b = g.add_node(0, sig_at(0.5));
        g.observe_edge(1_000, a, b, EdgeKind::Cooccur);
        assert!((g.edges[0].elig - 1.0).abs() < 1e-9);
        // Decay several tau later — elig should drop to nearly 0.
        g.decay_eligibility(1_000 + (PG_ELIG_TAU_US as u64) * 10);
        assert!(g.edges[0].elig < 0.01, "elig must decay over many tau, got {}", g.edges[0].elig);
    }

    #[test]
    fn serialize_roundtrip_preserves_full_state() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(1_000, sig_at(0.1));
        let b = g.add_node(2_000, sig_at(0.5));
        g.refresh_node(a, 5_000, &sig_at(0.11), 0.3);
        let cm = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8];
        g.update_cross_modal(a, &cm, 1.0);
        let vc = [0.9, -0.4, 0.2, 0.0, 0.1, -0.3];
        g.update_vital_coupling(a, &vc, 1.0);
        g.observe_edge(3_000, a, b, EdgeKind::Cooccur);
        g.observe_edge(4_000, a, b, EdgeKind::Causal);
        g.observe_edge(5_000, b, a, EdgeKind::CrossModal);
        let bytes = g.serialize();
        let back =
            PerturbationGraph::deserialize(&bytes).expect("roundtrip must succeed");
        assert_eq!(back.nodes.len(), g.nodes.len());
        assert_eq!(back.edges.len(), g.edges.len());
        assert_eq!(back.next_id, g.next_id);
        for (x, y) in g.nodes.iter().zip(back.nodes.iter()) {
            assert_eq!(x.id, y.id);
            for i in 0..PG_NODE_SIG_DIM {
                assert!((x.sig[i] - y.sig[i]).abs() < 1e-12);
            }
            for i in 0..PG_CROSS_MODAL_DIM {
                assert!((x.cross_modal[i] - y.cross_modal[i]).abs() < 1e-12);
            }
            for i in 0..PG_VITAL_DIM {
                assert!((x.vital_coupling[i] - y.vital_coupling[i]).abs() < 1e-12);
            }
            assert!((x.bind_strength - y.bind_strength).abs() < 1e-12);
            assert_eq!(x.age_us, y.age_us);
            assert_eq!(x.last_active_t_us, y.last_active_t_us);
        }
        for (x, y) in g.edges.iter().zip(back.edges.iter()) {
            assert_eq!(x.from, y.from);
            assert_eq!(x.to, y.to);
            assert_eq!(x.kind_byte, y.kind_byte);
            assert!((x.strength - y.strength).abs() < 1e-12);
            assert_eq!(x.last_t_us, y.last_t_us);
        }
    }

    #[test]
    fn deserialize_rejects_corrupt() {
        assert!(PerturbationGraph::deserialize(&[]).is_none());
        assert!(PerturbationGraph::deserialize(b"JUNKjunkjunk").is_none());
        let mut g = PerturbationGraph::default();
        g.add_node(0, sig_at(0.0));
        let bytes = g.serialize();
        // Truncate near the end of the byte stream: must reject (not commit
        // partial state).
        assert!(PerturbationGraph::deserialize(&bytes[..bytes.len() - 1]).is_none());
        // Empty (default) round-trips to empty.
        let empty =
            PerturbationGraph::deserialize(&PerturbationGraph::default().serialize());
        assert!(empty
            .map(|g| g.nodes.is_empty() && g.edges.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn merge_blends_matching_nodes_and_imports_novel_ones() {
        let mut mainline = PerturbationGraph::default();
        let m_a = mainline.add_node(0, sig_at(0.10));
        let m_b = mainline.add_node(0, sig_at(0.50));
        mainline.observe_edge(1_000, m_a, m_b, EdgeKind::Cooccur);

        let mut branch = PerturbationGraph::default();
        let b_a = branch.add_node(0, sig_at(0.11)); // close to m_a (sig_distance < merge_dist)
        let b_x = branch.add_node(0, sig_at(0.90)); // novel — no local match
        // Build evidence on branch.
        for _ in 0..10 {
            branch.refresh_node(b_a, 1_000, &sig_at(0.11), 0.3);
        }
        branch.observe_edge(2_000, b_a, b_x, EdgeKind::Cooccur);

        let nodes_before = mainline.nodes.len();
        mainline.merge(&branch, 0.05); // generous match distance for sig_at offsets
        assert!(
            mainline.nodes.len() > nodes_before,
            "novel branch node must be imported under a fresh local id"
        );
        // Edge from branch's b_a → b_x should now exist remapped on mainline,
        // with `m_a` as its source (because b_a matched m_a).
        assert!(
            mainline.edges.iter().any(|e| {
                e.kind_byte == EdgeKind::Cooccur.as_u8()
                    && e.from == m_a
                    && e.to != m_a
                    && e.to != m_b
            }),
            "branch edge must be remapped through the local id of the matched node"
        );
    }

    #[test]
    fn edge_kind_enum_roundtrip_with_defensive_fallback() {
        for &k in &[
            EdgeKind::Cooccur,
            EdgeKind::Nest,
            EdgeKind::Causal,
            EdgeKind::CrossModal,
        ] {
            assert_eq!(EdgeKind::from_u8(k.as_u8()), k);
        }
        // Unknown bytes fall back to the most generic relation (defensive).
        assert_eq!(EdgeKind::from_u8(255), EdgeKind::Cooccur);
    }

    #[test]
    fn node_bytes_stride_is_fixed_and_known() {
        // The persistence map flagged a historical 52→84 stride bug in the JS
        // parser. Pin the stride here so any change to NODE_BYTES becomes a
        // test failure visible at compile-time of the test suite.
        // 4 + 8*24 + 8*8 + 8*6 + 8*4 + 8 + 8 + 8 = 4 + 192 + 64 + 48 + 32 + 24 = 364
        assert_eq!(PerturbationGraph::NODE_BYTES, 364);
        assert_eq!(PerturbationGraph::EDGE_BYTES, 33);
    }
}
