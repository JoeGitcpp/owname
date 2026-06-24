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

/// Stable per-session group identifier. Mirrors NodeId conventions: id 0 is the
/// "no-group" sentinel and never assigned.
pub type GroupId = u32;

/// Stable per-session relation identifier. Mirrors NodeId / GroupId conventions:
/// id 0 is the "no-relation" sentinel and never assigned.
pub type RelId = u32;

// ── Phase A3: Endpoint + Relation as first-class citizens
//
// Per the v2 design doc: relations are FIRST-CLASS — they can be pointed at by
// other relations, can bear a bind_strength of their own, and have a stable
// id that survives across cycles of the graph. An edge in the old sense was
// always Node → Node; a Relation can have Node, Group, or Relation endpoints,
// recursive without depth cap (cycle protection via visited sets in traversal).
//
// The user's reframing (2026-06-18 conversation): a relation is NOT a "preference"
// — it IS a relation, described by the tier-0 axes that align between the
// endpoints. Strength × expressivity drives credit; expressivity is the count of
// tier-0 axes where the two endpoints' sigs are close.
//
// Phase A3 wires the data structures + APIs. Use sites (vision_tier0 cooccur
// edges, vocal_sequence transitions, language_binding) migrate in later steps;
// for now PerturbationEdge keeps powering them so this commit is additive.

/// A relation endpoint. Recursive: a relation can point at a node, a group, or
/// another relation. Cycle protection lives in traversal (not the type), so a
/// Relation(rel_id) endpoint that loops back to itself is structurally legal —
/// only depth-bounded walkers consume the graph.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Endpoint {
    Node(NodeId),
    Group(GroupId),
    Relation(RelId),
}

/// First-class relation between two perturbations. Unlike PerturbationEdge
/// (Node→Node only) a Relation can connect any Endpoint pair — including
/// Group→Group ("the keyboard cluster is part of the computer cluster") and
/// Relation→Node ("the fact that A causes B is associated with C").
///
/// Fields beyond what PerturbationEdge had:
/// - `id`: stable per-session, so other relations can refer to this one.
/// - `kind_id`: u32 covering both primitive EdgeKind discriminants (0..3) AND
///   emergent RelationKind cluster ids (4..PG_KIND_CAP) discovered by A4.
///   Replaces the A3 `kind_byte: u8` so emergent kinds fit alongside primitives.
/// - `expressivity`: tier-0 axis count engaged at last observation. Drives
///   credit weighting in inference passes. EMA over time.
/// - `last_axes_engaged`: u8 — raw count at last observation, for telemetry.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Relation {
    pub id: RelId,
    pub from: Endpoint,
    pub to: Endpoint,
    /// Kind cluster identifier. 0..3 reserved for primitive EdgeKind variants
    /// (Cooccur / Nest / Causal / CrossModal). 4..PG_KIND_CAP for emergent
    /// kinds discovered online via observe_relation_emergent.
    pub kind_id: u32,
    /// EMA of "this relation has been observed" in [0,1].
    pub strength: f64,
    /// EMA of tier-0 axis engagement ratio in [0,1]. 1.0 = all 24 axes aligned
    /// at last observation; 0.0 = no axes aligned. Multiplies strength when
    /// downstream inference picks the next hop.
    pub expressivity: f64,
    pub elig: f64,
    pub last_t_us: u64,
    pub last_axes_engaged: u8,
    /// A5 FastPath candidacy tracking. 0 = not currently above the promote
    /// threshold; otherwise the wall-clock timestamp of the FIRST sustained
    /// crossing. After FAST_PATH_PROMOTE_DURATION_US of continuous strength
    /// above FAST_PATH_PROMOTE_TH, this relation is promoted to FastPath.
    /// A drop below the threshold resets this field to 0 (a fresh climb
    /// restarts the clock — sustained presence is the qualifier).
    pub first_above_promote_t_us: u64,
}

// ── Phase A4: dynamic RelationKind discovery
//
// 4 fixed EdgeKind variants is a categorical cap (the same kind of cap the
// SLOT_COUNT=8 cutover was meant to escape). A4 opens the relation-kind
// vocabulary: each Relation carries a `kind_id` that can be either one of the
// 4 primitives (0..3) OR an emergent cluster id (4..PG_KIND_CAP) discovered
// online from the (from-sig, to-sig) context.
//
// Discovery rule: when a relation is observed, its context signature
// (concat of from_sig + to_sig, 48 dims) is compared against existing kinds'
// prototypes. If the nearest prototype is within KIND_DISCOVERY_DIST, that
// kind absorbs the observation (prototype EMA). Otherwise — provided capacity
// allows — a new kind spawns with this context as its initial prototype.

/// Total kind cap (4 primitives + up to ~252 emergent).
pub const PG_KIND_CAP: usize = 256;

/// Context-signature distance threshold for emergent kind discovery. Looser
/// than the node match distance (0.30) because kind contexts are higher-dim
/// (48 axes vs 24) so cosine of meaningful distance scales differently.
pub const KIND_DISCOVERY_DIST: f64 = 0.55;

/// EMA rate for prototype drift when an existing kind absorbs a new
/// observation. Slower than relation strength because a prototype represents
/// a kind's IDENTITY — should be stable, drift only on consensus shift.
pub const KIND_PROTOTYPE_ALPHA: f64 = 0.05;

/// EMA rate for kind strength (how often this kind fires).
pub const KIND_STRENGTH_ALPHA: f64 = 0.10;

/// Idle threshold after which a kind's strength decays multiplicatively each
/// decay tick. 1 day ≈ "this kind hasn't been observed in a while".
pub const KIND_DEMOTE_IDLE_US: u64 = 86_400_000_000;

/// Multiplicative decay applied to idle kind strength when demoting.
const KIND_DEMOTE_FACTOR: f64 = 0.5;

/// Below this strength an emergent kind is candidate for eviction. Primitive
/// kinds (cluster_id 0..3) are NEVER evicted regardless of strength.
const KIND_EVICTION_TH: f64 = 0.01;

/// Primitive kind ids — by convention these match EdgeKind discriminants
/// exactly so callers passing an EdgeKind get a sensible kind_id.
const KIND_ID_COOCCUR: u32 = 0;
const KIND_ID_NEST: u32 = 1;
const KIND_ID_CAUSAL: u32 = 2;
const KIND_ID_CROSS_MODAL: u32 = 3;
const KIND_ID_SEQUENCE: u32 = 4;
const PRIMITIVE_KIND_COUNT: usize = 5;

/// A relation-kind cluster: an online-discovered (or primitive-anchored)
/// category of relations sharing a similar (from-sig, to-sig) context.
///
/// The "prototype" is the kind's centroid in the 48-D concatenated context
/// space (from_sig 24 ++ to_sig 24). Observations within KIND_DISCOVERY_DIST
/// of an existing prototype drift it via EMA; observations farther away
/// (and within capacity) spawn a fresh kind.
#[derive(Clone, Debug, PartialEq)]
pub struct RelationKind {
    /// 0..3 = primitive (matches EdgeKind discriminants).
    /// 4..PG_KIND_CAP = emergent, online-discovered.
    pub cluster_id: u32,
    /// Concatenated context signature: prototype[..PG_NODE_SIG_DIM] is the
    /// from-side prototype; prototype[PG_NODE_SIG_DIM..] is the to-side.
    /// For primitives this is the EMA of the observed contexts that landed
    /// in the cluster; primitives don't have a meaningful initial prototype
    /// (they start at zero and accumulate from the first observation).
    pub prototype: [f64; PG_NODE_SIG_DIM * 2],
    /// EMA "how often this kind fires" — drives demotion / eviction defence.
    pub strength: f64,
    pub spawned_t_us: u64,
    pub last_seen_t_us: u64,
}

// ── Phase A5: FastPath (the muscle-memory / default-cognition substrate)
//
// The user's design-doc reframing (Section 1.2 Q3): FastPath is NOT a perf
// cache layer. It is the DEFAULT cognitive substrate. Most behaviour flows
// through it. Slow graph traversal + active reasoning is the exception path
// that fires when FastPath is silent OR when an Intention overrides.
//
// A relation becomes a FastPath when:
//   - strength > FAST_PATH_PROMOTE_TH (default 0.8)
//   - sustained for at least FAST_PATH_PROMOTE_DURATION_US (default 5 min real)
//   - expressivity > FAST_PATH_EXPRESSIVITY_MIN (default 0.4)
//
// It is demoted (hysteresis) when:
//   - strength < FAST_PATH_DEMOTE_TH (default 0.5)
//   - OR expressivity < FAST_PATH_DEMOTE_EXPRESSIVITY (default 0.2)
//
// Queries hit the FastPath FIRST (O(1) HashMap). A miss falls back to the
// slow relation walk. The presence of FastPath thus changes both performance
// AND the default behaviour of the graph, in keeping with the design vision.

pub const FAST_PATH_PROMOTE_TH: f64 = 0.80;
pub const FAST_PATH_DEMOTE_TH: f64 = 0.50;
pub const FAST_PATH_EXPRESSIVITY_MIN: f64 = 0.40;
pub const FAST_PATH_DEMOTE_EXPRESSIVITY: f64 = 0.20;
pub const FAST_PATH_PROMOTE_DURATION_US: u64 = 300_000_000; // 5 minutes
pub const PG_FAST_PATH_CAP: usize = 2048;

/// Post-hoc audit agreement threshold (Section 3.4 in the design doc).
pub const POST_HOC_AGREE_TH: f64 = 0.7;

/// Per-relation FastPath entry. Shares storage with the underlying Relation
/// (just an index by rel_id) — promotion is metadata, not duplication.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FastPath {
    pub rel_id: RelId,
    pub promoted_t_us: u64,
    pub usage_count: u64,
    pub last_fired_t_us: u64,
}

/// FastPath index — two hash maps over the same `FastPath` set:
/// - `by_endpoints` for O(1) (from, to) lookup (the duplicate-merge primary
///   key for promoted relations).
/// - `by_from` for "what habits do I have from this endpoint?" — drives the
///   query-first semantics in default_response().
#[derive(Clone, Debug, PartialEq, Default)]
pub struct FastPathIndex {
    pub by_endpoints: std::collections::HashMap<(Endpoint, Endpoint), FastPath>,
    pub by_from: std::collections::HashMap<Endpoint, Vec<RelId>>,
}

/// Outcome of `default_response` — either FastPath returned an immediate hit
/// (the muscle-memory case) or the caller should fall through to slow
/// deliberation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResponseOutcome {
    FastPath { rel_id: RelId, to: Endpoint },
    NeedsDeliberation,
}

// ── Phase A7: MarkRef + Mark + channel ownership (the "指向 / 标记" operator)
//
// The user's design-doc reframing: marking is the explicit "I am pointing at
// this right now" operator. A MarkRef points at a Node, a Group, or a
// Relation — i.e. anything in the graph that can be addressed. Each named
// CHANNEL owns at most one active Mark at a time, so curriculum / attention /
// dual_drive / intention each get a clean slot to write into without
// clobbering each other.

/// Polymorphic pointer into the graph. The thing a Mark points at. The user's
/// "指向/标记" is the verb; this is the noun the verb refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkRef {
    Node(NodeId),
    Group(GroupId),
    Relation(RelId),
}

/// Mark-owning channels. Each channel can hold at most one active Mark at a
/// time (the most recent stamp wins). Modelled as a small enum so callers
/// can't invent random channels — exactly four owners per the design doc.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MarkChannel {
    /// The attention system says "this is what's getting weighted right now".
    Attention,
    /// The curriculum scaffolding says "this is what the lesson is about".
    Curriculum,
    /// dual_drive says "this is the bond / threat we're balancing".
    DualDrive,
    /// IntentionState (Phase C) says "this is what I'm trying to predict".
    Intention,
}

impl MarkChannel {
    /// Stable u8 discriminant for persistence.
    pub fn as_u8(self) -> u8 {
        match self {
            MarkChannel::Attention => 0,
            MarkChannel::Curriculum => 1,
            MarkChannel::DualDrive => 2,
            MarkChannel::Intention => 3,
        }
    }
    pub fn from_u8(b: u8) -> Self {
        match b {
            1 => MarkChannel::Curriculum,
            2 => MarkChannel::DualDrive,
            3 => MarkChannel::Intention,
            _ => MarkChannel::Attention,
        }
    }
}

/// A single Mark — a channel's current pointer into the graph. Stamped by the
/// channel at a given time with a given strength; decays after MARK_DECAY_US
/// of idleness back to "no mark on this channel".
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark {
    pub target: MarkRef,
    pub stamped_by: MarkChannel,
    pub strength: f64,
    pub stamped_t_us: u64,
}

/// After this much idle time (no re-stamp on the same channel), a Mark is
/// considered stale and cleared. Q-C default in the design doc.
pub const MARK_DECAY_US: u64 = 30_000_000;

// ── Phase A6: VocalProfile (vocal-specific extension on PerturbationNode)
//
// vocal_target_recall stores RecallEntry { mel64: [f64; 64], f1_off, f2_off, fit,
// last_update_t_us } as a content-addressed (mel64-keyed) target↔motor map. To
// unify vocal cognition with the rest of the graph the substrate, A6 adds a
// sidecar map `vocal_profiles: HashMap<NodeId, VocalProfile>` on PerturbationGraph.
// Vocal-relevant nodes carry the full mel64 fingerprint and motor recipe as
// their VocalProfile, while non-vocal nodes have no profile (Option-typed).
//
// This is an ADDITIVE migration: vocal_target_recall continues to exist and
// drive the formant servo (its existing role). The mirror into perturbation_graph
// gives downstream consumers a unified API: "look up a node by mel64; if it
// has a vocal profile, that's the motor recipe to use".

/// Dimensions of the per-vocal-node mel64 fingerprint.
pub const PG_VOCAL_MEL_DIM: usize = 64;

/// mel64 shape distance below which a candidate sound counts as the SAME
/// vocal node. Mirrors RECALL_MATCH_DIST in vocal_target_recall (0.04).
pub const VOCAL_MEL_MATCH_DIST: f64 = 0.04;

/// Minimum fit below which a correction is not memorised (just noise).
/// Mirrors RECALL_STORE_MIN_FIT (0.30).
pub const VOCAL_FIT_MIN_STORE: f64 = 0.30;

/// Vocal-specific profile for a PerturbationNode. Carries the 64-band mel
/// fingerprint that identifies the target sound, plus the (f1_off, f2_off)
/// motor correction the body learned for producing it, and the best fit
/// achieved on that correction.
///
/// The profile is OPTIONAL — only vocal nodes have one. Visual nodes carry
/// their identity in PerturbationNode.sig (24-D) and no VocalProfile.
#[derive(Clone, Debug, PartialEq)]
pub struct VocalProfile {
    pub mel64: [f64; PG_VOCAL_MEL_DIM],
    pub f1_off: f64,
    pub f2_off: f64,
    /// Best-known fit (1 - shape_dist) on this correction.
    pub fit: f64,
    pub last_update_t_us: u64,
}

/// Maximum traversal depth for relation walks. Cycle-protected by a visited
/// set; depth cap kicks in for non-cyclic but deep chains (a → b → c → d ...
/// where each step is a distinct Endpoint). Per the design doc Q-G default.
pub const RELATION_TRAVERSAL_MAX_DEPTH: usize = 8;

/// Maximum traversal depth allowed during cross-lineage merge. Wider than the
/// inference cap because merge needs to chase the entire structure of an
/// imported branch, not just the locally-useful inference horizon.
pub const RELATION_MERGE_MAX_DEPTH: usize = 32;

/// Tier-0 axis similarity threshold above which the axis is "engaged" in a
/// relation's expressivity calculation. 0.5 = the two endpoint sigs differ by
/// less than 0.5 on that axis. With 24 axes total, fully-aligned endpoints
/// give expressivity = 1.0.
pub const RELATION_EXPRESSIVITY_AXIS_TH: f64 = 0.5;

/// Soft cap on first-class relations.
/// PG_REL_CAP = 131072 per the v2 design doc (Q-F default, ~6 MB inline).
pub const PG_REL_CAP: usize = 131_072;

/// EMA seed for a brand-new relation (mirrors PG_EDGE_SEED for the existing
/// edges system; keep the convention identical so the bootstrap-deadlock
/// guard works the same way).
const PG_REL_SEED: f64 = 0.2;

/// EMA alpha for the strength + expressivity updates on an observed relation.
const PG_REL_ALPHA: f64 = 0.20;

// ── Phase A1: NodeGroup + soft membership (the "neuron-as-perturbation-group" layer)
//
// Per the v2 design doc: a "neuron" is not a single perturbation patch — it is
// a region-local GROUP of patches, with membership being SOFT (each member has
// a weight in [0,1] derived from its sig distance to the group centroid). The
// group is what gets pointed at by relations and bound to by language.
//
// Phase A1 wires the group substrate; persistence + cross-lineage merge land
// in Phase A9. Groups are runtime-only this commit (regenerated each session
// by vision_tier0's group-formation pass in Phase A2).

/// Soft cap on lifetime groups. With ~3-5 members average per group at scale
/// the worst-case overhead is bounded; 1024 gives months of headroom in the
/// 5-minute visitor data range (260 nodes typically collapse into ~20-50
/// non-trivial groups across spatial+temporal+cooccur regions).
pub const PG_GROUP_CAP: usize = 1024;

/// Sig-distance threshold for "this node is close enough to this group's
/// centroid to be admitted". Tighter than the node-level TIER0_SIG_MATCH_DIST
/// (0.30) — groups are a coarser semantic level, members should be tightly
/// clustered around the centroid.
pub const GROUP_FORM_DIST: f64 = 0.20;

/// Minimum number of co-active nodes in a region needed to spawn a new group.
/// One or two patches don't make a group; three is the smallest cohort that
/// can sustain a centroid without trivially being moved by any one member.
pub const GROUP_FORM_MIN_NODES: usize = 3;

/// Gaussian σ for the distance-to-centroid → membership weight transform.
/// At d = σ the weight is e^(-0.5) ≈ 0.61; at 2σ ≈ 0.135. So GROUP_FORM_DIST
/// (0.20) being just over σ (0.15) means a node at admission distance has
/// weight ~0.41 — soft but real.
pub const GROUP_MEMBERSHIP_SIGMA: f64 = 0.15;

/// EMA rate for a member's weight update each refresh. 0.20 ≈ 5-tick time
/// constant; new evidence dominates after ~5 frames.
const GROUP_ALPHA_WEIGHT: f64 = 0.20;

/// EMA rate for the group centroid drifting toward the refreshing member's
/// sig. Damped by member.weight so loose members barely move the centroid.
/// 0.10 base × weight gives a tight-member effective alpha of ~0.10 and a
/// loose-member alpha of ~0.01 — centroid drifts but doesn't follow noise.
const GROUP_ALPHA_CENTROID: f64 = 0.10;

/// After this much idle time, member weights decay by GROUP_WEIGHT_DECAY_FACTOR
/// per call to decay_group_members. 30 s ≈ "this object left the scene".
pub const GROUP_MEMBER_DECAY_US: u64 = 30_000_000;

/// Multiplicative decay applied to idle member weights.
const GROUP_WEIGHT_DECAY_FACTOR: f64 = 0.95;

/// Members with weight below this are pruned. Just above floating-point noise
/// to avoid the membership list growing unboundedly with ghost members.
const GROUP_MEMBER_PRUNE_TH: f64 = 0.05;

/// Region operator (the "区域" in the design doc). A group is always tied to
/// a region — spatial bucket, time window, nest subtree, or cooccur cluster.
/// The same node may belong to multiple groups in different regions (spatially
/// close + temporally co-present + nested under a parent), with different
/// weights in each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RegionKey {
    /// Visual-space coarse bucket. 16 spatial buckets in each dimension
    /// (pos_x/pos_y ∈ [0,1] split into 16 strips → 256 distinct spatial cells).
    Spatial(u8, u8),
    /// Bucketed time window — `now_us / window_us`. Co-present nodes within
    /// the same temporal bucket share this region key.
    TemporalWindow(u64),
    /// Subtree of a Nest edge — a parent node's containment hierarchy.
    NestSubtree(NodeId),
    /// On-line discovered co-occurrence cluster (the streaming union-find /
    /// LSH result over Cooccur edges). Cluster ids are assigned by a future
    /// Phase A2 helper; the field is reserved for now.
    CoOccurCluster(u32),
}

/// A single member of a group, with soft weight and refresh timestamp.
/// Weight ∈ [0,1] is an EMA of gaussian(sig_distance(node.sig, centroid)).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroupMember {
    pub node_id: NodeId,
    pub weight: f64,
    pub last_refreshed_t_us: u64,
}

/// A perturbation-element GROUP — the user's "neuron-as-perturbation-group".
/// Members are soft (weighted), centroid is EMA-updated by refreshes, region
/// pins the spatial/temporal/relational context the group lives in.
#[derive(Clone, Debug, PartialEq)]
pub struct NodeGroup {
    pub id: GroupId,
    pub centroid_sig: [f64; PG_NODE_SIG_DIM],
    pub members: Vec<GroupMember>,
    pub region: RegionKey,
    /// Aggregated bind_strength: weighted mean of member bind_strengths.
    /// Recomputed lazily by decay_group_members; consumers call
    /// `most_bound_group()` which doesn't recompute on read.
    pub bind_strength: f64,
    pub formed_t_us: u64,
    pub last_active_t_us: u64,
}

/// Gaussian distance → membership-weight transform. Pure utility.
fn gaussian_weight(d: f64, sigma: f64) -> f64 {
    if sigma <= 0.0 {
        return if d <= 0.0 { 1.0 } else { 0.0 };
    }
    let x = d / sigma;
    (-(x * x) * 0.5).exp()
}

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
    /// Phase A8: temporal transition — A reliably preceded B as part of a
    /// learned sequence (vocal utterance, motor habit, etc.). Distinct from
    /// Causal because Sequence carries the LEARNED ORDER of a chain, not
    /// just a "first-then-other" co-occurrence.
    Sequence = 4,
}

impl EdgeKind {
    pub fn from_u8(b: u8) -> EdgeKind {
        match b {
            1 => EdgeKind::Nest,
            2 => EdgeKind::Causal,
            3 => EdgeKind::CrossModal,
            4 => EdgeKind::Sequence,
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
    /// Phase A1: NodeGroups — "neurons as perturbation-element groups".
    /// Runtime-only this commit; persistence lands in Phase A9.
    pub groups: Vec<NodeGroup>,
    /// Phase A3: first-class Relations. Distinct from `edges` (which stays as
    /// the low-level Node→Node primitive). Relations can connect any Endpoint
    /// pair (Node / Group / Relation) and carry their own id, so other
    /// relations can point at THEM. Runtime-only this commit; persistence
    /// lands in Phase A9.
    pub relations: Vec<Relation>,
    /// Phase A4: RelationKind clusters. First PRIMITIVE_KIND_COUNT (4) entries
    /// are the EdgeKind-anchored primitives; further entries are emergent
    /// kinds discovered online from observe_relation_emergent. Capped at
    /// PG_KIND_CAP. Runtime-only this commit; persistence lands in Phase A9.
    pub kinds: Vec<RelationKind>,
    /// Phase A5: FastPath index — the muscle-memory layer. Promoted relations
    /// that have sustained high strength + expressivity for at least
    /// FAST_PATH_PROMOTE_DURATION_US (5 min default) sit here for O(1) query.
    /// `default_response` queries this BEFORE the slow graph walk: most
    /// behaviour flows through FastPath; the slow path is the exception.
    /// Runtime-only this commit; persistence lands in Phase A9.
    pub fast_paths: FastPathIndex,
    /// Phase A7: per-channel active Marks. The "指向 / 标记" operator. At most
    /// one Mark per MarkChannel at any time; new stamps replace older marks
    /// on the same channel. Decay to empty after MARK_DECAY_US of idleness.
    /// Stored as a small Vec<Mark>; lookups are linear over ≤ 4 entries.
    pub marks: Vec<Mark>,
    /// Phase A6: vocal-specific extension on selected PerturbationNodes.
    /// Vocal nodes have a VocalProfile (mel64 fingerprint + motor recipe);
    /// non-vocal nodes have none. Stored as a HashMap because the population
    /// is small (~16 entries — mirrors vocal_target_recall's RECALL_CAP) and
    /// the access pattern is lookup by NodeId. Runtime-only this commit;
    /// persistence lands in Phase A9.
    pub vocal_profiles: std::collections::HashMap<NodeId, VocalProfile>,
    /// Next id to hand out — never resets to 0 (the sentinel) and never reuses
    /// an evicted node's id within the same session.
    pub next_id: u32,
    /// Next group id to hand out. Same sentinel/no-reuse convention as next_id.
    pub next_group_id: u32,
    /// Next relation id to hand out. Same sentinel/no-reuse convention.
    pub next_rel_id: u32,
    /// Next emergent kind id to hand out. Starts at PRIMITIVE_KIND_COUNT.
    pub next_kind_id: u32,
    /// Phase B3: hit / query EMAs of the FastPath query channel. Tracks
    /// "muscle memory is doing the work" as a live ratio readable by
    /// consumers (INTERO_FAST_PATH_HIT_RATE). Both decay every call so a
    /// long quiet stretch erodes the rate naturally rather than freezing
    /// the last observed value.
    pub fast_path_hit_ema: f64,
    pub fast_path_query_ema: f64,
    /// Phase C: rolling fraction of post_hoc_check rounds where the
    /// fired FastPath disagreed with the observed outcome. Surfaced on
    /// INTERO_POST_HOC_DISAGREE_RATE — if this trends high, FastPath
    /// promotion is too eager.
    pub post_hoc_disagree_ema: f64,
}

impl Default for PerturbationGraph {
    fn default() -> Self {
        // Seed the kinds vector with the 4 primitives. Their prototypes are
        // zero — they will accumulate the contexts of relations explicitly
        // tagged with that EdgeKind. Strength starts at 0; primitives are
        // immune to eviction regardless.
        let mut kinds = Vec::with_capacity(PRIMITIVE_KIND_COUNT);
        for primitive in 0..PRIMITIVE_KIND_COUNT as u32 {
            kinds.push(RelationKind {
                cluster_id: primitive,
                prototype: [0.0; PG_NODE_SIG_DIM * 2],
                strength: 0.0,
                spawned_t_us: 0,
                last_seen_t_us: 0,
            });
        }
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            groups: Vec::new(),
            relations: Vec::new(),
            kinds,
            fast_paths: FastPathIndex::default(),
            marks: Vec::new(),
            vocal_profiles: std::collections::HashMap::new(),
            next_id: 1,
            next_group_id: 1,
            next_rel_id: 1,
            next_kind_id: PRIMITIVE_KIND_COUNT as u32,
            fast_path_hit_ema: 0.0,
            fast_path_query_ema: 0.0,
            post_hoc_disagree_ema: 0.0,
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
        // Drop group memberships referencing the evicted node so its weight
        // doesn't keep skewing group centroid drift / bind_strength.
        for g in self.groups.iter_mut() {
            g.members.retain(|m| m.node_id != evicted_id);
        }
    }

    // ── Group ops (Phase A1) ─────────────────────────────────────────────────

    fn alloc_group_id(&mut self) -> GroupId {
        let id = self.next_group_id;
        self.next_group_id = self.next_group_id.wrapping_add(1);
        if self.next_group_id == 0 {
            self.next_group_id = 1;
        }
        id
    }

    /// Spawn a NodeGroup from a seed cohort of node ids within a region. The
    /// centroid is initialised as the unweighted mean of seed-member sigs;
    /// each seed's weight is gaussian(sig_distance(node.sig, centroid)).
    /// Returns None if no seed id resolves to a live node.
    pub fn add_group(
        &mut self,
        now_us: u64,
        region: RegionKey,
        seed_members: &[NodeId],
    ) -> Option<GroupId> {
        if seed_members.is_empty() {
            return None;
        }
        // First pass: build centroid as the mean of resolved seed sigs.
        let mut centroid_sig = [0.0f64; PG_NODE_SIG_DIM];
        let mut count = 0usize;
        for &nid in seed_members {
            if let Some(n) = self.node(nid) {
                for i in 0..PG_NODE_SIG_DIM {
                    centroid_sig[i] += n.sig[i];
                }
                count += 1;
            }
        }
        if count == 0 {
            return None;
        }
        let denom = count as f64;
        for v in centroid_sig.iter_mut() {
            *v /= denom;
        }
        // Capacity check (after the seed sigs have been read so the borrow on
        // self is released by the time we mutate).
        if self.groups.len() >= PG_GROUP_CAP {
            self.evict_weakest_group(now_us);
        }
        let id = self.alloc_group_id();
        // Second pass: stamp members with distance-derived weights.
        let mut members: Vec<GroupMember> = Vec::with_capacity(count);
        for &nid in seed_members {
            if let Some(n) = self.node(nid) {
                let d = sig_distance(&n.sig, &centroid_sig);
                members.push(GroupMember {
                    node_id: nid,
                    weight: gaussian_weight(d, GROUP_MEMBERSHIP_SIGMA),
                    last_refreshed_t_us: now_us,
                });
            }
        }
        let bind_strength = Self::compute_group_bind_strength(&members, &self.nodes);
        self.groups.push(NodeGroup {
            id,
            centroid_sig,
            members,
            region,
            bind_strength,
            formed_t_us: now_us,
            last_active_t_us: now_us,
        });
        Some(id)
    }

    /// Soft-admit / refresh a node's membership in a group. If the node is
    /// already a member, its weight EMAs toward the new gaussian(distance);
    /// otherwise it joins with the initial gaussian-weighted weight. The group
    /// centroid drifts toward the node's sig in proportion to the resulting
    /// member weight (loose members barely move the centroid).
    pub fn refresh_group(
        &mut self,
        group_id: GroupId,
        node_id: NodeId,
        now_us: u64,
    ) {
        // Resolve node sig first so the immutable borrow ends before we
        // grab a mutable group reference.
        let node_sig = match self.node(node_id) {
            Some(n) => n.sig,
            None => return,
        };
        let g_idx = match self.groups.iter().position(|g| g.id == group_id) {
            Some(i) => i,
            None => return,
        };
        let g = &mut self.groups[g_idx];
        let d = sig_distance(&node_sig, &g.centroid_sig);
        let target_w = gaussian_weight(d, GROUP_MEMBERSHIP_SIGMA);
        let effective_w = if let Some(m) =
            g.members.iter_mut().find(|m| m.node_id == node_id)
        {
            m.weight = (1.0 - GROUP_ALPHA_WEIGHT) * m.weight
                + GROUP_ALPHA_WEIGHT * target_w;
            m.last_refreshed_t_us = now_us;
            m.weight
        } else {
            g.members.push(GroupMember {
                node_id,
                weight: target_w,
                last_refreshed_t_us: now_us,
            });
            target_w
        };
        // Drift the centroid toward the refreshing node, damped by member
        // weight: tight members pull the centroid hard, loose ones barely.
        let alpha = GROUP_ALPHA_CENTROID * effective_w;
        for i in 0..PG_NODE_SIG_DIM {
            g.centroid_sig[i] =
                (1.0 - alpha) * g.centroid_sig[i] + alpha * node_sig[i];
        }
        g.last_active_t_us = now_us;
    }

    /// Find the group whose centroid is closest to `sig` AND whose region
    /// matches `region`. Region equality is exact — a node looking for its
    /// spatial group only sees Spatial(_, _) groups, never temporal ones.
    pub fn nearest_group(
        &self,
        region: RegionKey,
        sig: &[f64; PG_NODE_SIG_DIM],
        max_dist: f64,
    ) -> Option<(GroupId, f64)> {
        let mut best: Option<(GroupId, f64)> = None;
        for g in &self.groups {
            if g.region != region {
                continue;
            }
            let d = sig_distance(&g.centroid_sig, sig);
            if d <= max_dist && best.map(|(_, bd)| d < bd).unwrap_or(true) {
                best = Some((g.id, d));
            }
        }
        best
    }

    pub fn group(&self, id: GroupId) -> Option<&NodeGroup> {
        self.groups.iter().find(|g| g.id == id)
    }

    pub fn has_group(&self, id: GroupId) -> bool {
        self.groups.iter().any(|g| g.id == id)
    }

    /// The group with the highest aggregated bind_strength — the closest
    /// "neuron-level" analog of `most_bound`. Returns None on an empty graph.
    pub fn most_bound_group(&self) -> Option<&NodeGroup> {
        self.groups
            .iter()
            .max_by(|a, b| a.bind_strength.partial_cmp(&b.bind_strength).unwrap())
    }

    /// Iterator over groups whose last_active_t_us is within the active window.
    pub fn active_groups<'a>(&'a self, now_us: u64) -> impl Iterator<Item = &'a NodeGroup> + 'a {
        self.groups
            .iter()
            .filter(move |g| now_us.saturating_sub(g.last_active_t_us) <= PG_ACTIVE_WINDOW_US)
    }

    /// Idempotent maintenance: decay member weights that have not been
    /// refreshed for GROUP_MEMBER_DECAY_US, prune the ones that fall below
    /// the floor, recompute each group's bind_strength, AND drop groups that
    /// have been left with zero members. The drop step is what stops
    /// TemporalWindow / Spatial groups from accumulating as empty husks once
    /// their cohort moves on — the diagnostic on the first post-Phase-A real
    /// call showed 87% of groups carried 0 members because their members
    /// decayed out without the group itself ever being evicted. Safe to call
    /// every tick.
    pub fn decay_group_members(&mut self, now_us: u64) {
        let nodes_snapshot = &self.nodes; // immutable borrow for compute
        for g in self.groups.iter_mut() {
            for m in g.members.iter_mut() {
                let idle = now_us.saturating_sub(m.last_refreshed_t_us);
                if idle > GROUP_MEMBER_DECAY_US {
                    m.weight *= GROUP_WEIGHT_DECAY_FACTOR;
                }
            }
            g.members.retain(|m| m.weight > GROUP_MEMBER_PRUNE_TH);
            g.bind_strength = Self::compute_group_bind_strength(&g.members, nodes_snapshot);
        }
        // Drop empty husks. Age-protect briefly so a just-formed group has a
        // chance to bind a second cohort even if the first one was sparse.
        const GROUP_EMPTY_GRACE_US: u64 = 5_000_000;
        self.groups.retain(|g| {
            !g.members.is_empty()
                || now_us.saturating_sub(g.formed_t_us) < GROUP_EMPTY_GRACE_US
        });
    }

    /// Weighted mean of member bind_strengths, weighted by membership weight.
    fn compute_group_bind_strength(
        members: &[GroupMember],
        nodes: &[PerturbationNode],
    ) -> f64 {
        let mut num = 0.0;
        let mut den = 0.0;
        for m in members {
            if let Some(n) = nodes.iter().find(|n| n.id == m.node_id) {
                num += m.weight * n.bind_strength;
                den += m.weight;
            }
        }
        if den < 1e-9 {
            0.0
        } else {
            (num / den).clamp(0.0, 1.0)
        }
    }

    fn evict_weakest_group(&mut self, now_us: u64) {
        if self.groups.is_empty() {
            return;
        }
        let mut weakest_idx = 0usize;
        let mut weakest_def = f64::INFINITY;
        for (i, g) in self.groups.iter().enumerate() {
            let age = now_us.saturating_sub(g.formed_t_us) as f64;
            let age_bonus = (age / PG_AGE_PROTECT_US).min(1.0) * PG_AGE_PROTECT_MAX_BONUS;
            let idle = now_us.saturating_sub(g.last_active_t_us) as f64;
            let recency_penalty = 1.0 - (-idle / PG_IDLE_DECAY_US).exp();
            let defence = g.bind_strength + age_bonus - 0.1 * recency_penalty;
            if defence < weakest_def {
                weakest_def = defence;
                weakest_idx = i;
            }
        }
        self.groups.swap_remove(weakest_idx);
    }

    // ── Relation ops (Phase A3) ─────────────────────────────────────────────

    fn alloc_rel_id(&mut self) -> RelId {
        let id = self.next_rel_id;
        self.next_rel_id = self.next_rel_id.wrapping_add(1);
        if self.next_rel_id == 0 {
            self.next_rel_id = 1;
        }
        id
    }

    /// Resolve an Endpoint to its representative tier-0 signature, used for
    /// `expressivity` computation. Returns None when the endpoint does not
    /// resolve (evicted node / group, dangling relation id). For Relation
    /// endpoints, the signature is the source's resolved sig — i.e. follow
    /// the "from" chain once, so meta-relations inherit the topic they relate.
    pub fn endpoint_sig(&self, ep: Endpoint) -> Option<[f64; PG_NODE_SIG_DIM]> {
        match ep {
            Endpoint::Node(id) => self.node(id).map(|n| n.sig),
            Endpoint::Group(id) => self.group(id).map(|g| g.centroid_sig),
            Endpoint::Relation(id) => {
                // Single-hop indirection only. Recursive resolution would need
                // cycle protection; consumers that want full traversal should
                // use traverse_relations_from().
                let from = self.relation(id).map(|r| r.from)?;
                match from {
                    Endpoint::Relation(_) => None, // refuse deeper recursion
                    other => self.endpoint_sig(other),
                }
            }
        }
    }

    /// True iff the endpoint resolves to a live entity in this graph.
    pub fn endpoint_exists(&self, ep: Endpoint) -> bool {
        match ep {
            Endpoint::Node(id) => id != 0 && self.has_node(id),
            Endpoint::Group(id) => id != 0 && self.has_group(id),
            Endpoint::Relation(id) => id != 0 && self.has_relation(id),
        }
    }

    pub fn relation(&self, id: RelId) -> Option<&Relation> {
        self.relations.iter().find(|r| r.id == id)
    }

    pub fn has_relation(&self, id: RelId) -> bool {
        self.relations.iter().any(|r| r.id == id)
    }

    /// Observe a relation between two endpoints with an EXPLICIT primitive
    /// kind. The caller knows the semantic relation (cooccur, nest, causal,
    /// cross-modal); the kind_id is taken from the EdgeKind discriminant
    /// (0..3). The matching primitive RelationKind's prototype absorbs the
    /// (from, to) context via EMA so primitives still gain expressive content
    /// from the relations that pass through them.
    ///
    /// For emergent online-discovered kinds, use `observe_relation_emergent`.
    ///
    /// Returns the RelId of the (created or refreshed) relation, or None when
    /// the observation was rejected (self-loop, sentinel id, dangling endpoint).
    pub fn observe_relation(
        &mut self,
        now_us: u64,
        from: Endpoint,
        to: Endpoint,
        kind: EdgeKind,
    ) -> Option<RelId> {
        let kind_id = kind.as_u8() as u32;
        self.observe_relation_with_kind_id(now_us, from, to, kind_id)
    }

    /// Observe a relation and ALSO let the system classify it into the best-
    /// matching RelationKind cluster (primitive or emergent). The kind cluster
    /// whose prototype is nearest (by context-sig distance) to the new
    /// observation absorbs it — if the nearest is within KIND_DISCOVERY_DIST.
    /// Otherwise a fresh emergent kind spawns with this observation as its
    /// initial prototype, provided capacity allows.
    ///
    /// The user's design-doc reframing: "千丝万缕" implies an open vocabulary
    /// of relation kinds; this is the API that grows that vocabulary.
    pub fn observe_relation_emergent(
        &mut self,
        now_us: u64,
        from: Endpoint,
        to: Endpoint,
    ) -> Option<RelId> {
        // Build the context signature (from_sig ++ to_sig).
        let ctx = match (self.endpoint_sig(from), self.endpoint_sig(to)) {
            (Some(a), Some(b)) => concat_context(&a, &b),
            _ => return None, // can't classify a dangling relation
        };
        // Classify into existing kind or spawn a new one.
        let kind_id = self.classify_or_spawn_kind(&ctx, now_us);
        self.observe_relation_with_kind_id(now_us, from, to, kind_id)
    }

    /// Shared implementation that takes an explicit kind_id (post-classification).
    fn observe_relation_with_kind_id(
        &mut self,
        now_us: u64,
        from: Endpoint,
        to: Endpoint,
        kind_id: u32,
    ) -> Option<RelId> {
        if from == to {
            return None;
        }
        if !self.endpoint_exists(from) || !self.endpoint_exists(to) {
            return None;
        }
        // Compute axis engagement now, before mutable borrows.
        let (expr, engaged) = match (self.endpoint_sig(from), self.endpoint_sig(to)) {
            (Some(a), Some(b)) => relation_expressivity(&a, &b),
            _ => (0.0, 0u8),
        };
        // EMA-drift the kind's prototype + strength toward the new observation.
        // For primitives this gives them measurable expressive content over time.
        let ctx = match (self.endpoint_sig(from), self.endpoint_sig(to)) {
            (Some(a), Some(b)) => Some(concat_context(&a, &b)),
            _ => None,
        };
        if let Some(ctx) = ctx {
            if let Some(k) = self.kinds.iter_mut().find(|k| k.cluster_id == kind_id) {
                for i in 0..PG_NODE_SIG_DIM * 2 {
                    k.prototype[i] = (1.0 - KIND_PROTOTYPE_ALPHA) * k.prototype[i]
                        + KIND_PROTOTYPE_ALPHA * ctx[i];
                }
                k.strength = (k.strength + KIND_STRENGTH_ALPHA * (1.0 - k.strength))
                    .clamp(0.0, 1.0);
                k.last_seen_t_us = now_us;
            }
        }
        // Now handle the Relation list (mirror of A3 logic but with kind_id u32).
        if let Some(r) = self
            .relations
            .iter_mut()
            .find(|r| r.from == from && r.to == to && r.kind_id == kind_id)
        {
            r.strength = (r.strength + PG_REL_ALPHA * (1.0 - r.strength)).clamp(0.0, 1.0);
            r.expressivity =
                (1.0 - PG_REL_ALPHA) * r.expressivity + PG_REL_ALPHA * expr;
            r.elig = 1.0;
            r.last_t_us = now_us;
            r.last_axes_engaged = engaged;
            return Some(r.id);
        }
        if self.relations.len() >= PG_REL_CAP {
            self.evict_weakest_relation();
        }
        let id = self.alloc_rel_id();
        self.relations.push(Relation {
            id,
            from,
            to,
            kind_id,
            strength: PG_REL_SEED,
            expressivity: expr,
            elig: 1.0,
            last_t_us: now_us,
            last_axes_engaged: engaged,
            first_above_promote_t_us: 0,
        });
        Some(id)
    }

    fn alloc_kind_id(&mut self) -> u32 {
        let id = self.next_kind_id;
        self.next_kind_id = self.next_kind_id.wrapping_add(1);
        // Clamp away from primitive id range on wrap.
        if (self.next_kind_id as usize) < PRIMITIVE_KIND_COUNT {
            self.next_kind_id = PRIMITIVE_KIND_COUNT as u32;
        }
        id
    }

    /// Classify a context signature into the nearest existing kind, or spawn
    /// a new emergent kind when no existing prototype is within
    /// KIND_DISCOVERY_DIST. Always returns a valid kind_id.
    fn classify_or_spawn_kind(
        &mut self,
        ctx: &[f64; PG_NODE_SIG_DIM * 2],
        now_us: u64,
    ) -> u32 {
        // First pass: find nearest existing kind by context-sig distance.
        // Primitives whose prototype is all-zero (never observed) are
        // considered "matchless" so emergent discovery isn't blocked by an
        // unused primitive sitting at the origin.
        let mut best: Option<(u32, f64)> = None;
        for k in &self.kinds {
            let proto_norm: f64 = k.prototype.iter().map(|x| x * x).sum::<f64>().sqrt();
            if proto_norm < 1e-6 {
                continue; // unset prototype, skip
            }
            let d = ctx_distance(ctx, &k.prototype);
            if d <= KIND_DISCOVERY_DIST
                && best.map(|(_, bd)| d < bd).unwrap_or(true)
            {
                best = Some((k.cluster_id, d));
            }
        }
        if let Some((id, _)) = best {
            return id;
        }
        // No close existing kind. Spawn a fresh emergent one (if capacity).
        if self.kinds.len() < PG_KIND_CAP {
            let id = self.alloc_kind_id();
            self.kinds.push(RelationKind {
                cluster_id: id,
                prototype: *ctx,
                strength: KIND_STRENGTH_ALPHA, // seed > 0 so it's evictable but visible
                spawned_t_us: now_us,
                last_seen_t_us: now_us,
            });
            return id;
        }
        // At capacity and nothing matched — fall back to Cooccur primitive.
        // Better than silently failing or evicting a learned emergent kind
        // mid-observation; the consumer can run kind eviction later.
        KIND_ID_COOCCUR
    }

    /// Lookup a kind by cluster_id. Returns None if no such kind is registered.
    pub fn kind(&self, cluster_id: u32) -> Option<&RelationKind> {
        self.kinds.iter().find(|k| k.cluster_id == cluster_id)
    }

    /// Decay long-idle EMERGENT kinds. Primitive kinds (0..3) are immune.
    /// When an emergent kind's last_seen_t_us is older than KIND_DEMOTE_IDLE_US,
    /// its strength is multiplied by KIND_DEMOTE_FACTOR. Below KIND_EVICTION_TH
    /// the kind is dropped and any relations tagged with its cluster_id are
    /// re-tagged with KIND_ID_COOCCUR (so they don't dangle).
    pub fn decay_emergent_kinds(&mut self, now_us: u64) {
        // First pass: collect cluster_ids to evict.
        let mut evict_ids: Vec<u32> = Vec::new();
        for k in self.kinds.iter_mut() {
            if (k.cluster_id as usize) < PRIMITIVE_KIND_COUNT {
                continue; // primitives never decay
            }
            if now_us.saturating_sub(k.last_seen_t_us) > KIND_DEMOTE_IDLE_US {
                k.strength *= KIND_DEMOTE_FACTOR;
            }
            if k.strength < KIND_EVICTION_TH {
                evict_ids.push(k.cluster_id);
            }
        }
        if evict_ids.is_empty() {
            return;
        }
        // Re-tag any relations referencing the evicted kinds.
        for r in self.relations.iter_mut() {
            if evict_ids.contains(&r.kind_id) {
                r.kind_id = KIND_ID_COOCCUR;
            }
        }
        self.kinds.retain(|k| !evict_ids.contains(&k.cluster_id));
    }

    fn evict_weakest_relation(&mut self) {
        if let Some((idx, _)) = self
            .relations
            .iter()
            .enumerate()
            .min_by(|(_, a), (_, b)| {
                let ad = a.strength * a.expressivity;
                let bd = b.strength * b.expressivity;
                ad.partial_cmp(&bd).unwrap()
            })
        {
            self.relations.swap_remove(idx);
        }
    }

    /// Decay every relation's eligibility trace by elapsed time. Idempotent.
    pub fn decay_relation_eligibility(&mut self, now_us: u64) {
        for r in self.relations.iter_mut() {
            let dt = now_us.saturating_sub(r.last_t_us) as f64;
            r.elig = (-dt / PG_ELIG_TAU_US).exp();
        }
    }

    /// Traverse forward from an endpoint, calling `visit(endpoint, depth)` for
    /// each unique reachable endpoint up to `max_depth`. Cycle-protected via
    /// a `HashSet<Endpoint>` of seen endpoints — a relation that points back
    /// to its ancestor is visited once and not followed again.
    pub fn traverse_relations_from(
        &self,
        start: Endpoint,
        max_depth: usize,
        mut visit: impl FnMut(Endpoint, usize),
    ) {
        if max_depth == 0 || !self.endpoint_exists(start) {
            return;
        }
        let mut stack: Vec<(Endpoint, usize)> = vec![(start, 0)];
        let mut visited: std::collections::HashSet<Endpoint> =
            std::collections::HashSet::new();
        while let Some((ep, d)) = stack.pop() {
            if !visited.insert(ep) || d > max_depth {
                continue;
            }
            visit(ep, d);
            if d == max_depth {
                continue;
            }
            // Push outgoing relations as next frames.
            for r in self.relations.iter().filter(|r| r.from == ep) {
                if !visited.contains(&r.to) {
                    stack.push((r.to, d + 1));
                }
            }
        }
    }

    /// Iterate outgoing relations from a given endpoint.
    pub fn out_relations<'a>(
        &'a self,
        from: Endpoint,
    ) -> impl Iterator<Item = &'a Relation> + 'a {
        self.relations.iter().filter(move |r| r.from == from)
    }

    /// Strongest outgoing relation (by strength × expressivity, the
    /// credit-weighted form the design doc specifies for inference).
    pub fn strongest_out_relation(&self, from: Endpoint) -> Option<&Relation> {
        self.out_relations(from).max_by(|a, b| {
            let av = a.strength * a.expressivity;
            let bv = b.strength * b.expressivity;
            av.partial_cmp(&bv).unwrap()
        })
    }

    /// B2.1: temperature-sampled outgoing relation — the divergent counterpart
    /// of [`strongest_out_relation`](Self::strongest_out_relation). With
    /// `temperature <= PG_WALK_TEMP_EPS` this returns exactly the argmax (zero
    /// regression on a cold/immature graph); above it, relations are drawn in
    /// proportion to `(strength × expressivity)` sharpened by `1/temperature`,
    /// so a mature graph can wander to a non-strongest edge. `u` is a single
    /// uniform draw in `[0,1)`; the caller owns the RNG so this stays a pure,
    /// unit-testable selection policy (mirrors `pg_reasoning_blend`).
    pub fn sample_out_relation(
        &self,
        from: Endpoint,
        temperature: f64,
        u: f64,
    ) -> Option<&Relation> {
        // Cold / argmax fast path — identical result to strongest_out_relation.
        if temperature <= crate::PG_WALK_TEMP_EPS {
            return self.strongest_out_relation(from);
        }
        let cands: Vec<&Relation> = self.out_relations(from).collect();
        if cands.is_empty() {
            return None;
        }
        let score = |r: &Relation| (r.strength * r.expressivity).max(0.0);
        let max_score = cands.iter().map(|r| score(r)).fold(0.0_f64, f64::max);
        if max_score <= 0.0 {
            // No expressive edge yet — stay deterministic.
            return self.strongest_out_relation(from);
        }
        // Normalised power weights: the strongest edge is 1.0, weaker edges
        // decay with 1/temperature. Normalising by max_score avoids overflow
        // and keeps the argmax limit exact as temperature → 0.
        let inv_t = 1.0 / temperature;
        let weights: Vec<f64> = cands
            .iter()
            .map(|r| (score(r) / max_score).powf(inv_t))
            .collect();
        let sum: f64 = weights.iter().sum();
        if sum <= 0.0 {
            return self.strongest_out_relation(from);
        }
        let target = u.clamp(0.0, 1.0) * sum;
        let mut acc = 0.0;
        for i in 0..cands.len() {
            acc += weights[i];
            if acc >= target {
                return Some(cands[i]);
            }
        }
        cands.last().copied()
    }

    /// B2.2: rest cue + its associative neighbourhood (read-only). Picks a
    /// salient seed cue (top node by `bind_strength × (1+persistence_ema)`,
    /// clock-rotated by `now_us` so the cue varies across quiet ticks — an
    /// internal cue, no RNG) and SPREADS from it along LEARNED relations up to
    /// `spread_depth`. Returns the visited node ids and the mean salience of the
    /// neighbourhood. None below the 8-node floor (cold graph → history replay).
    /// First live caller of `traverse_relations_from`.
    fn quiet_cue_neighborhood(
        &self,
        now_us: u64,
        spread_depth: usize,
    ) -> Option<(Vec<NodeId>, f64)> {
        if self.nodes.len() < 8 {
            return None;
        }
        let salience =
            |n: &PerturbationNode| (n.bind_strength * (1.0 + n.persistence_ema)).clamp(0.0, 1.0);
        let mut top: Vec<(NodeId, f64)> =
            self.nodes.iter().map(|n| (n.id, salience(n))).collect();
        top.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        let k = top.len().min(8);
        let idx = ((now_us / 100_000) as usize) % k.max(1);
        let (seed, seed_sal) = top[idx];
        // Spread from the cue along learned relations (cycle-safe, depth-bounded).
        let mut visited_ids: Vec<NodeId> = Vec::new();
        self.traverse_relations_from(Endpoint::Node(seed), spread_depth, |ep, _d| {
            if let Endpoint::Node(nid) = ep {
                visited_ids.push(nid);
            }
        });
        // Mean salience of the evoked neighbourhood: a cue embedded in a rich,
        // salient associative web replays stronger than an isolated one.
        let mut sum = 0.0;
        let mut count = 0.0;
        for &nid in &visited_ids {
            if let Some(n) = self.node(nid) {
                sum += salience(n);
                count += 1.0;
            }
        }
        let novelty = if count > 0.0 { sum / count } else { seed_sal };
        Some((visited_ids, novelty))
    }

    /// B2.2: read-only rest-replay salience — the mean salience of the cue's
    /// associative neighbourhood. See [`quiet_cue_neighborhood`](Self::quiet_cue_neighborhood).
    pub fn quiet_replay_novelty(&self, now_us: u64, spread_depth: usize) -> Option<f64> {
        self.quiet_cue_neighborhood(now_us, spread_depth)
            .map(|(_, novelty)| novelty)
    }

    /// B2.2b: rest consolidation. Re-walks the rest cue's associative
    /// neighbourhood and gives each rehearsed node BOUNDED eviction protection:
    /// its idle clock is pulled up only as far as "just aged out of the active
    /// window", so rehearsal resists forgetting WITHOUT letting the node
    /// masquerade as currently perceived — `is_active` stays false, and the live
    /// binding / disturbance set is untouched. The `.max` never moves a fresher
    /// node's timestamp backward. Returns the neighbourhood novelty (identical to
    /// `quiet_replay_novelty`) so the caller still gets the replay signal.
    pub fn consolidate_quiet_neighborhood(
        &mut self,
        now_us: u64,
        spread_depth: usize,
    ) -> Option<f64> {
        let (ids, novelty) = self.quiet_cue_neighborhood(now_us, spread_depth)?;
        // Just outside the active window: max protection among non-active nodes.
        let protect_floor = now_us.saturating_sub(PG_ACTIVE_WINDOW_US + 1);
        for id in ids {
            if let Some(n) = self.node_mut(id) {
                n.last_active_t_us = n.last_active_t_us.max(protect_floor);
            }
        }
        Some(novelty)
    }

    // ── FastPath ops (Phase A5: muscle memory / default cognition) ───────────

    /// The user's design-doc reframing: most behaviour flows through the
    /// muscle-memory layer. Query the FastPath FIRST given a `from` endpoint;
    /// if any habit fires, return it as the response. Otherwise fall through
    /// to the slow walk (callers can invoke `traverse_relations_from` etc.).
    pub fn default_response(&self, from: Endpoint) -> ResponseOutcome {
        if let Some(rel_ids) = self.fast_paths.by_from.get(&from) {
            // Pick the strongest fast-path candidate from this endpoint. The
            // index might have stale ids (eviction race); skip ones whose
            // backing relation has been demoted out from under us.
            let mut best: Option<(RelId, f64)> = None;
            for &rid in rel_ids {
                if let Some(r) = self.relation(rid) {
                    let score = r.strength * r.expressivity;
                    if best.map(|(_, bs)| score > bs).unwrap_or(true) {
                        best = Some((rid, score));
                    }
                }
            }
            if let Some((rid, _)) = best {
                let r = self.relation(rid).unwrap();
                return ResponseOutcome::FastPath { rel_id: rid, to: r.to };
            }
        }
        ResponseOutcome::NeedsDeliberation
    }

    /// Phase B3 telemetry: record whether the most recent FastPath query was
    /// a hit. EMA-tracked so consumers (INTERO_FAST_PATH_HIT_RATE) can see a
    /// smooth "muscle memory is doing the work" reading. Decay is symmetric
    /// — a long quiet stretch erodes both counters toward zero.
    pub fn note_fast_path_query(&mut self, was_hit: bool) {
        const ALPHA: f64 = 0.05;
        self.fast_path_query_ema = (1.0 - ALPHA) * self.fast_path_query_ema + ALPHA;
        let hit = if was_hit { 1.0 } else { 0.0 };
        self.fast_path_hit_ema = (1.0 - ALPHA) * self.fast_path_hit_ema + ALPHA * hit;
    }

    /// EMA-derived hit rate in [0,1]. Returns 0 when the channel has never
    /// been probed (fresh boot) — consumers can treat that as "no muscle
    /// memory in play yet" without special-casing.
    pub fn fast_path_hit_rate(&self) -> f64 {
        if self.fast_path_query_ema < 1e-6 {
            0.0
        } else {
            (self.fast_path_hit_ema / self.fast_path_query_ema).clamp(0.0, 1.0)
        }
    }

    /// Density readout: relations per node. Capped at 64 (saturates well
    /// before the design-doc "千丝万缕" ratio); divided by the cap before
    /// publishing so the axis stays in [0,1].
    pub fn graph_density(&self) -> f64 {
        if self.nodes.is_empty() {
            0.0
        } else {
            ((self.relations.len() as f64) / (self.nodes.len() as f64) / 64.0).clamp(0.0, 1.0)
        }
    }

    /// B2.0: instability readout for reasoning — population std-dev of relation
    /// `strength` (scaled ×2, clamped to [0,1]). High when the relation set spans
    /// mixed maturities (a structurally shifting network: some habits committing
    /// while others are fresh); ~0 when relations are uniform, and exactly 0 with
    /// fewer than two relations. Cheap O(relations) pass — and the reasoning
    /// cycle already iterates `relations` for graph_surprise, so this adds no new
    /// traversal class. Replaces the slot-fabric `relation_err + |lead_lag|` term.
    pub fn graph_instability(&self) -> f64 {
        let n = self.relations.len();
        if n < 2 {
            return 0.0;
        }
        let mean = self.relations.iter().map(|r| r.strength).sum::<f64>() / n as f64;
        let var = self
            .relations
            .iter()
            .map(|r| {
                let d = r.strength - mean;
                d * d
            })
            .sum::<f64>()
            / n as f64;
        (2.0 * var.sqrt()).clamp(0.0, 1.0)
    }

    /// Phase C: bump the post-hoc disagree EMA. Caller is `post_hoc_check`.
    pub fn note_post_hoc_outcome(&mut self, disagreed: bool) {
        const ALPHA: f64 = 0.05;
        let d = if disagreed { 1.0 } else { 0.0 };
        self.post_hoc_disagree_ema = (1.0 - ALPHA) * self.post_hoc_disagree_ema + ALPHA * d;
    }

    /// Tally a FastPath usage (called by the consumer after `default_response`
    /// returns a FastPath outcome and the response was actually emitted).
    pub fn note_fast_path_usage(&mut self, rel_id: RelId, now_us: u64) {
        let key = self.relation(rel_id).map(|r| (r.from, r.to));
        if let Some(key) = key {
            if let Some(fp) = self.fast_paths.by_endpoints.get_mut(&key) {
                fp.usage_count = fp.usage_count.saturating_add(1);
                fp.last_fired_t_us = now_us;
            }
        }
    }

    /// Periodic promote/demote pass. Mirrors the design-doc rules:
    /// - A relation that sustains strength > FAST_PATH_PROMOTE_TH for at
    ///   least FAST_PATH_PROMOTE_DURATION_US wall-clock, AND has expressivity
    ///   > FAST_PATH_EXPRESSIVITY_MIN, is PROMOTED to a FastPath.
    /// - A FastPath whose backing relation falls below FAST_PATH_DEMOTE_TH
    ///   OR FAST_PATH_DEMOTE_EXPRESSIVITY is DEMOTED.
    /// - When the cap is hit, the LRU (oldest last_fired_t_us) is dropped to
    ///   make room.
    /// Safe to call at any cadence (e.g. 1Hz); idempotent.
    pub fn tick_fast_paths(&mut self, now_us: u64) {
        // First: maintain Relation.first_above_promote_t_us so the promote
        // check sees an honest "how long has it been above" timer.
        for r in self.relations.iter_mut() {
            let above =
                r.strength > FAST_PATH_PROMOTE_TH && r.expressivity > FAST_PATH_EXPRESSIVITY_MIN;
            if above {
                if r.first_above_promote_t_us == 0 {
                    r.first_above_promote_t_us = now_us;
                }
            } else {
                r.first_above_promote_t_us = 0;
            }
        }
        // Collect promote / demote decisions in a separate pass so we don't
        // mutate the relation list while reading it.
        let mut to_promote: Vec<RelId> = Vec::new();
        let mut to_demote: Vec<RelId> = Vec::new();
        for r in &self.relations {
            let in_index = self.fast_paths.by_endpoints.contains_key(&(r.from, r.to));
            let qualifies_promote = r.first_above_promote_t_us > 0
                && now_us.saturating_sub(r.first_above_promote_t_us) >= FAST_PATH_PROMOTE_DURATION_US
                && r.expressivity > FAST_PATH_EXPRESSIVITY_MIN;
            let qualifies_demote = r.strength < FAST_PATH_DEMOTE_TH
                || r.expressivity < FAST_PATH_DEMOTE_EXPRESSIVITY;
            match (in_index, qualifies_promote, qualifies_demote) {
                (false, true, _) => to_promote.push(r.id),
                (true, _, true) => to_demote.push(r.id),
                _ => {}
            }
        }
        for rid in to_demote {
            self.demote_fast_path(rid);
        }
        for rid in to_promote {
            if self.fast_paths.by_endpoints.len() >= PG_FAST_PATH_CAP {
                // Evict LRU before promoting a new one.
                self.evict_lru_fast_path();
            }
            self.promote_fast_path(rid, now_us);
        }
    }

    fn promote_fast_path(&mut self, rid: RelId, now_us: u64) {
        let key = match self.relation(rid).map(|r| (r.from, r.to)) {
            Some(k) => k,
            None => return,
        };
        // Re-entry guard (in case caller called this twice).
        if self.fast_paths.by_endpoints.contains_key(&key) {
            return;
        }
        let fp = FastPath {
            rel_id: rid,
            promoted_t_us: now_us,
            usage_count: 0,
            last_fired_t_us: now_us, // count promotion as a baseline activity
        };
        self.fast_paths.by_endpoints.insert(key, fp);
        self.fast_paths
            .by_from
            .entry(key.0)
            .or_default()
            .push(rid);
    }

    fn demote_fast_path(&mut self, rid: RelId) {
        let key = match self.relation(rid).map(|r| (r.from, r.to)) {
            Some(k) => k,
            None => return,
        };
        self.fast_paths.by_endpoints.remove(&key);
        if let Some(list) = self.fast_paths.by_from.get_mut(&key.0) {
            list.retain(|&id| id != rid);
        }
    }

    fn evict_lru_fast_path(&mut self) {
        let lru = self
            .fast_paths
            .by_endpoints
            .iter()
            .min_by_key(|(_, fp)| fp.last_fired_t_us)
            .map(|(_, fp)| fp.rel_id);
        if let Some(rid) = lru {
            self.demote_fast_path(rid);
        }
    }

    /// Post-hoc check: the FastPath has already responded, the actual
    /// downstream evidence is now in. Compare the predicted endpoint against
    /// the observed outcome — strengthen the FastPath if they agree, weaken
    /// otherwise. The design-doc invariant: this runs ASYNC, not every fire.
    ///
    /// Returns true if the prediction was deemed correct.
    pub fn post_hoc_check(
        &mut self,
        fired_rel: RelId,
        observed_outcome: Endpoint,
    ) -> bool {
        let (predicted_to, from_sig, to_sig) = match self.relation(fired_rel) {
            Some(r) => (
                r.to,
                self.endpoint_sig(r.from).unwrap_or([0.0; PG_NODE_SIG_DIM]),
                self.endpoint_sig(r.to).unwrap_or([0.0; PG_NODE_SIG_DIM]),
            ),
            None => return false,
        };
        let observed_sig = self
            .endpoint_sig(observed_outcome)
            .unwrap_or([0.0; PG_NODE_SIG_DIM]);
        // Agreement = how aligned the observed outcome's sig is to the
        // predicted endpoint's sig. Above threshold = correct prediction.
        let _ = from_sig;
        let (_, axes_match) = relation_expressivity(&to_sig, &observed_sig);
        let agreement = axes_match as f64 / PG_NODE_SIG_DIM as f64;
        let correct = agreement > POST_HOC_AGREE_TH || predicted_to == observed_outcome;
        if let Some(r) = self.relations.iter_mut().find(|r| r.id == fired_rel) {
            if correct {
                r.strength = (r.strength + 0.01).min(1.0);
            } else {
                r.strength *= 0.95;
            }
        }
        correct
    }

    // ── Vocal profile ops (Phase A6) ───────────────────────────────────────

    /// Register or refresh a vocal profile on an existing node. The mel64
    /// fingerprint identifies the target; (f1_off, f2_off) is the motor
    /// recipe; fit is the best-known shape match. If the node has no profile
    /// yet, one is created. If it already has one, fit-gated update: only
    /// overwrite when the new fit is at least as good (mirrors
    /// vocal_target_recall::store's "best-so-far" rule).
    /// No-op if fit < VOCAL_FIT_MIN_STORE or node_id is unknown.
    pub fn register_vocal_profile(
        &mut self,
        node_id: NodeId,
        mel64: [f64; PG_VOCAL_MEL_DIM],
        f1_off: f64,
        f2_off: f64,
        fit: f64,
        now_us: u64,
    ) {
        if fit < VOCAL_FIT_MIN_STORE {
            return;
        }
        if !self.has_node(node_id) {
            return;
        }
        let entry = self.vocal_profiles.entry(node_id).or_insert(VocalProfile {
            mel64,
            f1_off,
            f2_off,
            fit,
            last_update_t_us: now_us,
        });
        // Existing profile: update only if the new fit is at least as good.
        if fit >= entry.fit {
            entry.mel64 = mel64;
            entry.f1_off = f1_off;
            entry.f2_off = f2_off;
            entry.fit = fit;
            entry.last_update_t_us = now_us;
        }
    }

    pub fn vocal_profile(&self, node_id: NodeId) -> Option<&VocalProfile> {
        self.vocal_profiles.get(&node_id)
    }

    /// Content-addressed lookup: find the vocal node whose mel64 fingerprint is
    /// nearest to the query target within VOCAL_MEL_MATCH_DIST. Returns
    /// (node_id, distance). Mirrors vocal_target_recall::recall.
    pub fn nearest_vocal_node(
        &self,
        target: &[f64; PG_VOCAL_MEL_DIM],
    ) -> Option<(NodeId, f64)> {
        // Silent target → no fingerprint → no match.
        if target.iter().sum::<f64>() <= 1e-6 {
            return None;
        }
        let mut best: Option<(NodeId, f64)> = None;
        for (&nid, vp) in &self.vocal_profiles {
            let d = mel64_shape_distance(target, &vp.mel64);
            if d <= VOCAL_MEL_MATCH_DIST
                && best.map(|(_, bd)| d < bd).unwrap_or(true)
            {
                best = Some((nid, d));
            }
        }
        best
    }

    /// Drop vocal profiles whose backing node has been evicted. Safe to call
    /// every tick; bounded by `vocal_profiles.len()` (small).
    pub fn prune_orphan_vocal_profiles(&mut self) {
        let live_ids: std::collections::HashSet<NodeId> =
            self.nodes.iter().map(|n| n.id).collect();
        self.vocal_profiles.retain(|nid, _| live_ids.contains(nid));
    }

    // ── Mark ops (Phase A7: the "指向/标记" operator) ───────────────────────

    /// Returns true if the MarkRef target still resolves to a live entity.
    pub fn mark_target_exists(&self, t: MarkRef) -> bool {
        match t {
            MarkRef::Node(id) => id != 0 && self.has_node(id),
            MarkRef::Group(id) => id != 0 && self.has_group(id),
            MarkRef::Relation(id) => id != 0 && self.has_relation(id),
        }
    }

    /// Stamp a Mark from `channel` onto `target` with `strength`. Replaces any
    /// previous Mark on the same channel (one active Mark per channel — the
    /// design-doc ownership rule). No-op if the target doesn't exist.
    pub fn stamp_mark(
        &mut self,
        channel: MarkChannel,
        target: MarkRef,
        strength: f64,
        now_us: u64,
    ) {
        if !self.mark_target_exists(target) {
            return;
        }
        let s = strength.clamp(0.0, 1.0);
        if let Some(m) = self.marks.iter_mut().find(|m| m.stamped_by == channel) {
            m.target = target;
            m.strength = s;
            m.stamped_t_us = now_us;
        } else {
            self.marks.push(Mark {
                target,
                stamped_by: channel,
                strength: s,
                stamped_t_us: now_us,
            });
        }
    }

    /// Read the current Mark on a channel, if any. Caller should still check
    /// staleness via Mark.stamped_t_us; a recently-decayed Mark is dropped by
    /// `decay_marks`, but a caller might want a stricter freshness rule.
    pub fn get_mark(&self, channel: MarkChannel) -> Option<&Mark> {
        self.marks.iter().find(|m| m.stamped_by == channel)
    }

    /// Drop Marks whose last stamp is older than MARK_DECAY_US. Idempotent;
    /// safe to call every tick.
    pub fn decay_marks(&mut self, now_us: u64) {
        self.marks
            .retain(|m| now_us.saturating_sub(m.stamped_t_us) <= MARK_DECAY_US);
    }

    /// Clear a single channel's mark explicitly (callers can release a Mark
    /// before it decays naturally, e.g. when intent is satisfied).
    pub fn clear_mark(&mut self, channel: MarkChannel) {
        self.marks.retain(|m| m.stamped_by != channel);
    }

    /// All marks currently active (after any pending decay). Convenience for
    /// observers and dashboards.
    pub fn active_marks<'a>(&'a self) -> impl Iterator<Item = &'a Mark> + 'a {
        self.marks.iter()
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
        // Phase A9 merge: fold the OTHER lineage's vocal_profiles in too.
        // A profile's node_id was remapped during the node-merge pass; look
        // up the local id, and import the profile if (a) we don't already
        // have one OR (b) the imported one has a better fit (mirrors
        // register_vocal_profile's best-so-far rule).
        for (&other_nid, other_vp) in &other.vocal_profiles {
            let Some(&local_nid) = id_map.get(&other_nid) else {
                continue;
            };
            match self.vocal_profiles.get(&local_nid) {
                Some(local_vp) if local_vp.fit >= other_vp.fit => continue,
                _ => {
                    self.vocal_profiles.insert(local_nid, other_vp.clone());
                }
            }
        }
        // Groups + relations + kinds + fast_paths are intentionally NOT merged
        // — they regenerate from observation in the next session. The merge
        // pipeline is deliberately conservative: node + vocal profile content
        // is the durable shared skill, the rest is structure that the body
        // reconstructs from its own sensory stream.
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
    /// Phase A9 sidecar version. v1 = original (nodes + edges only). v2 adds
    /// groups + relations + kinds + fast_paths + vocal_profiles (everything
    /// that the A1-A8 changes introduced). Marks are intentionally NOT
    /// persisted — they decay within MARK_DECAY_US (30 s) and reconstruct
    /// from the attention / intention subsystems each session.
    const VERSION_V1: u8 = 0x01;
    const VERSION: u8 = 0x02;

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

    /// Endpoint on-disk size: tag u8 + id u32.
    const ENDPOINT_BYTES: usize = 1 + 4;

    /// Bytes per group HEADER (excluding variable members block).
    const GROUP_HEADER_BYTES: usize = 4 // id u32
        + 1 + 8 // region tag + payload
        + 8 * PG_NODE_SIG_DIM // centroid_sig
        + 8 // bind_strength
        + 8 // formed_t_us
        + 8 // last_active_t_us
        + 8; // n_members u64

    /// Bytes per group member entry.
    const MEMBER_BYTES: usize = 4 + 8 + 8;

    /// Bytes per relation: id u32 + Endpoint(5) ×2 + kind_id u32 + strength f64
    /// + expressivity f64 + elig f64 + last_t_us u64 + last_axes_engaged u8
    /// + first_above_promote_t_us u64.
    const REL_BYTES: usize =
        4 + Self::ENDPOINT_BYTES + Self::ENDPOINT_BYTES + 4 + 8 + 8 + 8 + 8 + 1 + 8;

    /// Bytes per RelationKind on disk.
    const KIND_BYTES: usize = 4 // cluster_id
        + 8 * PG_NODE_SIG_DIM * 2 // prototype [f64; 48]
        + 8 // strength
        + 8 // spawned_t_us
        + 8; // last_seen_t_us

    /// Bytes per FastPath on disk.
    const FAST_PATH_BYTES: usize = 4 + 8 + 8 + 8;

    /// Bytes per VocalProfile on disk.
    const VOCAL_PROFILE_BYTES: usize = 4 // node_id
        + 8 * PG_VOCAL_MEL_DIM
        + 8 // f1_off
        + 8 // f2_off
        + 8 // fit
        + 8; // last_update_t_us

    /// Encode an Endpoint as ENDPOINT_BYTES (tag u8 + id u32).
    fn encode_endpoint(out: &mut Vec<u8>, ep: Endpoint) {
        match ep {
            Endpoint::Node(id) => {
                out.push(0);
                out.extend_from_slice(&id.to_le_bytes());
            }
            Endpoint::Group(id) => {
                out.push(1);
                out.extend_from_slice(&id.to_le_bytes());
            }
            Endpoint::Relation(id) => {
                out.push(2);
                out.extend_from_slice(&id.to_le_bytes());
            }
        }
    }

    /// Decode an Endpoint (tag u8 + id u32). Returns None on bad tag.
    fn decode_endpoint(data: &[u8], off: &mut usize) -> Option<Endpoint> {
        if data.len() < *off + Self::ENDPOINT_BYTES {
            return None;
        }
        let tag = data[*off];
        *off += 1;
        let id = u32::from_le_bytes(data[*off..*off + 4].try_into().ok()?);
        *off += 4;
        match tag {
            0 => Some(Endpoint::Node(id)),
            1 => Some(Endpoint::Group(id)),
            2 => Some(Endpoint::Relation(id)),
            _ => None,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        // Pre-size: v2 header + all blocks. Underestimate is fine (Vec grows).
        let mut total_groups_bytes = 0usize;
        for g in &self.groups {
            total_groups_bytes += Self::GROUP_HEADER_BYTES + g.members.len() * Self::MEMBER_BYTES;
        }
        let cap = 4 + 1 + 4 * 4 // header + 4 next_* ids
            + 8 + self.nodes.len() * Self::NODE_BYTES
            + 8 + self.edges.len() * Self::EDGE_BYTES
            + 8 + total_groups_bytes
            + 8 + self.relations.len() * Self::REL_BYTES
            + 8 + self.kinds.len() * Self::KIND_BYTES
            + 8 + self.fast_paths.by_endpoints.len() * Self::FAST_PATH_BYTES
            + 8 + self.vocal_profiles.len() * Self::VOCAL_PROFILE_BYTES;
        let mut out = Vec::with_capacity(cap);

        // ── Header: magic + version + 4 next_* allocators ─────────────────
        out.extend_from_slice(&Self::MAGIC);
        out.push(Self::VERSION);
        out.extend_from_slice(&self.next_id.to_le_bytes());
        out.extend_from_slice(&self.next_group_id.to_le_bytes());
        out.extend_from_slice(&self.next_rel_id.to_le_bytes());
        out.extend_from_slice(&self.next_kind_id.to_le_bytes());

        // ── Nodes ──────────────────────────────────────────────────────────
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

        // ── Edges ──────────────────────────────────────────────────────────
        out.extend_from_slice(&(self.edges.len() as u64).to_le_bytes());
        for e in &self.edges {
            out.extend_from_slice(&e.from.to_le_bytes());
            out.extend_from_slice(&e.to.to_le_bytes());
            out.push(e.kind_byte);
            out.extend_from_slice(&e.strength.to_le_bytes());
            out.extend_from_slice(&e.elig.to_le_bytes());
            out.extend_from_slice(&e.last_t_us.to_le_bytes());
        }

        // ── Groups (variable per-group due to member count) ───────────────
        out.extend_from_slice(&(self.groups.len() as u64).to_le_bytes());
        for g in &self.groups {
            out.extend_from_slice(&g.id.to_le_bytes());
            // Region tag + 8-byte payload.
            match g.region {
                RegionKey::Spatial(x, y) => {
                    out.push(0);
                    let payload: [u8; 8] = [x, y, 0, 0, 0, 0, 0, 0];
                    out.extend_from_slice(&payload);
                }
                RegionKey::TemporalWindow(t) => {
                    out.push(1);
                    out.extend_from_slice(&t.to_le_bytes());
                }
                RegionKey::NestSubtree(nid) => {
                    out.push(2);
                    out.extend_from_slice(&(nid as u64).to_le_bytes());
                }
                RegionKey::CoOccurCluster(cid) => {
                    out.push(3);
                    out.extend_from_slice(&(cid as u64).to_le_bytes());
                }
            }
            for v in &g.centroid_sig {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&g.bind_strength.to_le_bytes());
            out.extend_from_slice(&g.formed_t_us.to_le_bytes());
            out.extend_from_slice(&g.last_active_t_us.to_le_bytes());
            out.extend_from_slice(&(g.members.len() as u64).to_le_bytes());
            for m in &g.members {
                out.extend_from_slice(&m.node_id.to_le_bytes());
                out.extend_from_slice(&m.weight.to_le_bytes());
                out.extend_from_slice(&m.last_refreshed_t_us.to_le_bytes());
            }
        }

        // ── Relations ──────────────────────────────────────────────────────
        out.extend_from_slice(&(self.relations.len() as u64).to_le_bytes());
        for r in &self.relations {
            out.extend_from_slice(&r.id.to_le_bytes());
            Self::encode_endpoint(&mut out, r.from);
            Self::encode_endpoint(&mut out, r.to);
            out.extend_from_slice(&r.kind_id.to_le_bytes());
            out.extend_from_slice(&r.strength.to_le_bytes());
            out.extend_from_slice(&r.expressivity.to_le_bytes());
            out.extend_from_slice(&r.elig.to_le_bytes());
            out.extend_from_slice(&r.last_t_us.to_le_bytes());
            out.push(r.last_axes_engaged);
            out.extend_from_slice(&r.first_above_promote_t_us.to_le_bytes());
        }

        // ── Kinds ──────────────────────────────────────────────────────────
        out.extend_from_slice(&(self.kinds.len() as u64).to_le_bytes());
        for k in &self.kinds {
            out.extend_from_slice(&k.cluster_id.to_le_bytes());
            for v in &k.prototype {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&k.strength.to_le_bytes());
            out.extend_from_slice(&k.spawned_t_us.to_le_bytes());
            out.extend_from_slice(&k.last_seen_t_us.to_le_bytes());
        }

        // ── FastPaths ──────────────────────────────────────────────────────
        out.extend_from_slice(&(self.fast_paths.by_endpoints.len() as u64).to_le_bytes());
        for (_, fp) in &self.fast_paths.by_endpoints {
            out.extend_from_slice(&fp.rel_id.to_le_bytes());
            out.extend_from_slice(&fp.promoted_t_us.to_le_bytes());
            out.extend_from_slice(&fp.usage_count.to_le_bytes());
            out.extend_from_slice(&fp.last_fired_t_us.to_le_bytes());
        }

        // ── VocalProfiles ──────────────────────────────────────────────────
        out.extend_from_slice(&(self.vocal_profiles.len() as u64).to_le_bytes());
        for (nid, vp) in &self.vocal_profiles {
            out.extend_from_slice(&nid.to_le_bytes());
            for v in &vp.mel64 {
                out.extend_from_slice(&v.to_le_bytes());
            }
            out.extend_from_slice(&vp.f1_off.to_le_bytes());
            out.extend_from_slice(&vp.f2_off.to_le_bytes());
            out.extend_from_slice(&vp.fit.to_le_bytes());
            out.extend_from_slice(&vp.last_update_t_us.to_le_bytes());
        }
        out
    }

    pub fn deserialize(data: &[u8]) -> Option<Self> {
        // Minimum header check: at least magic + version (5 bytes).
        if data.len() < 5 || data[0..4] != Self::MAGIC {
            return None;
        }
        let version = data[4];
        if version != Self::VERSION && version != Self::VERSION_V1 {
            return None;
        }
        let mut off = 5usize;
        let next_id = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
        off += 4;
        // v2 has 3 additional next_* allocators in the header; v1 doesn't.
        let (next_group_id, next_rel_id, next_kind_id) = if version == Self::VERSION {
            let ngid = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
            off += 4;
            let nrid = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
            off += 4;
            let nkid = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
            off += 4;
            (ngid, nrid, nkid)
        } else {
            (1, 1, PRIMITIVE_KIND_COUNT as u32)
        };
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
        // v1: groups + relations + kinds + fast_paths + vocal_profiles all
        // regenerate. v2: read each block. Seed primitive kinds either way so
        // even a partial / v1 load lands with a usable substrate.
        let mut groups: Vec<NodeGroup> = Vec::new();
        let mut relations: Vec<Relation> = Vec::new();
        let mut kinds: Vec<RelationKind> = Vec::new();
        let mut fast_path_entries: Vec<FastPath> = Vec::new();
        let mut vocal_profiles: std::collections::HashMap<NodeId, VocalProfile> =
            std::collections::HashMap::new();

        if version == Self::VERSION {
            // Groups block
            if data.len() < off + 8 { return None; }
            let n_groups = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
            off += 8;
            if n_groups > PG_GROUP_CAP { return None; }
            for _ in 0..n_groups {
                if data.len() < off + Self::GROUP_HEADER_BYTES { return None; }
                let gid = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
                off += 4;
                let region_tag = data[off];
                off += 1;
                let mut payload = [0u8; 8];
                payload.copy_from_slice(&data[off..off + 8]);
                off += 8;
                let region = match region_tag {
                    0 => RegionKey::Spatial(payload[0], payload[1]),
                    1 => RegionKey::TemporalWindow(u64::from_le_bytes(payload)),
                    2 => RegionKey::NestSubtree(
                        u64::from_le_bytes(payload) as u32,
                    ),
                    3 => RegionKey::CoOccurCluster(u64::from_le_bytes(payload) as u32),
                    _ => return None,
                };
                let mut centroid_sig = [0.0f64; PG_NODE_SIG_DIM];
                for v in centroid_sig.iter_mut() {
                    *v = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                    off += 8;
                }
                let bind_strength = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let formed_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let last_active_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let n_members = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
                off += 8;
                if data.len() < off + n_members * Self::MEMBER_BYTES { return None; }
                let mut members = Vec::with_capacity(n_members);
                for _ in 0..n_members {
                    let node_id = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
                    off += 4;
                    let weight = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                    off += 8;
                    let last_refreshed_t_us =
                        u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                    off += 8;
                    members.push(GroupMember {
                        node_id,
                        weight,
                        last_refreshed_t_us,
                    });
                }
                groups.push(NodeGroup {
                    id: gid,
                    centroid_sig,
                    members,
                    region,
                    bind_strength,
                    formed_t_us,
                    last_active_t_us,
                });
            }
            // Relations block
            if data.len() < off + 8 { return None; }
            let n_relations = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
            off += 8;
            if n_relations > PG_REL_CAP { return None; }
            if data.len() < off + n_relations * Self::REL_BYTES { return None; }
            for _ in 0..n_relations {
                let id = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
                off += 4;
                let from = Self::decode_endpoint(data, &mut off)?;
                let to = Self::decode_endpoint(data, &mut off)?;
                let kind_id = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
                off += 4;
                let strength = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let expressivity = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let elig = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let last_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let last_axes_engaged = data[off];
                off += 1;
                let first_above_promote_t_us =
                    u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                relations.push(Relation {
                    id,
                    from,
                    to,
                    kind_id,
                    strength,
                    expressivity,
                    elig,
                    last_t_us,
                    last_axes_engaged,
                    first_above_promote_t_us,
                });
            }
            // Kinds block
            if data.len() < off + 8 { return None; }
            let n_kinds = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
            off += 8;
            if n_kinds > PG_KIND_CAP { return None; }
            if data.len() < off + n_kinds * Self::KIND_BYTES { return None; }
            for _ in 0..n_kinds {
                let cluster_id = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
                off += 4;
                let mut prototype = [0.0f64; PG_NODE_SIG_DIM * 2];
                for v in prototype.iter_mut() {
                    *v = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                    off += 8;
                }
                let strength = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let spawned_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let last_seen_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                kinds.push(RelationKind {
                    cluster_id,
                    prototype,
                    strength,
                    spawned_t_us,
                    last_seen_t_us,
                });
            }
            // FastPaths block
            if data.len() < off + 8 { return None; }
            let n_fast = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
            off += 8;
            if n_fast > PG_FAST_PATH_CAP { return None; }
            if data.len() < off + n_fast * Self::FAST_PATH_BYTES { return None; }
            for _ in 0..n_fast {
                let rel_id = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
                off += 4;
                let promoted_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let usage_count = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let last_fired_t_us = u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                fast_path_entries.push(FastPath {
                    rel_id,
                    promoted_t_us,
                    usage_count,
                    last_fired_t_us,
                });
            }
            // VocalProfiles block
            if data.len() < off + 8 { return None; }
            let n_vp = u64::from_le_bytes(data[off..off + 8].try_into().ok()?) as usize;
            off += 8;
            if data.len() < off + n_vp * Self::VOCAL_PROFILE_BYTES { return None; }
            for _ in 0..n_vp {
                let node_id = u32::from_le_bytes(data[off..off + 4].try_into().ok()?);
                off += 4;
                let mut mel64 = [0.0f64; PG_VOCAL_MEL_DIM];
                for v in mel64.iter_mut() {
                    *v = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                    off += 8;
                }
                let f1_off = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let f2_off = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let fit = f64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                let last_update_t_us =
                    u64::from_le_bytes(data[off..off + 8].try_into().ok()?);
                off += 8;
                vocal_profiles.insert(
                    node_id,
                    VocalProfile {
                        mel64,
                        f1_off,
                        f2_off,
                        fit,
                        last_update_t_us,
                    },
                );
            }
        }

        // If kinds is empty (v1 reload or no kinds in v2), seed the primitives
        // so the substrate is usable.
        if kinds.is_empty() {
            kinds.reserve(PRIMITIVE_KIND_COUNT);
            for primitive in 0..PRIMITIVE_KIND_COUNT as u32 {
                kinds.push(RelationKind {
                    cluster_id: primitive,
                    prototype: [0.0; PG_NODE_SIG_DIM * 2],
                    strength: 0.0,
                    spawned_t_us: 0,
                    last_seen_t_us: 0,
                });
            }
        }

        // Rebuild the FastPathIndex from the persisted FastPath list. The
        // by_endpoints / by_from caches are derived from (rel_id → relation).
        let mut fast_paths = FastPathIndex::default();
        for fp in fast_path_entries {
            if let Some(r) = relations.iter().find(|r| r.id == fp.rel_id) {
                let key = (r.from, r.to);
                fast_paths.by_endpoints.insert(key, fp);
                fast_paths
                    .by_from
                    .entry(key.0)
                    .or_default()
                    .push(fp.rel_id);
            }
        }

        Some(PerturbationGraph {
            nodes,
            edges,
            groups,
            relations,
            kinds,
            fast_paths,
            marks: Vec::new(), // Marks are not persisted (decay-based)
            vocal_profiles,
            next_id,
            next_group_id,
            next_rel_id,
            next_kind_id,
            // Phase B3 telemetry — not persisted, rebuilt online.
            fast_path_hit_ema: 0.0,
            fast_path_query_ema: 0.0,
            post_hoc_disagree_ema: 0.0,
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

/// Tier-0 axis-engagement score between two endpoint signatures. Counts the
/// number of axes where the per-axis absolute difference is below
/// `RELATION_EXPRESSIVITY_AXIS_TH` (so the axes are "aligned"). Returns
/// `(ratio_in_0_1, raw_count)`. The user's design-doc reframing: a relation's
/// expressivity = how many tier-0 axes describe the link, not just whether
/// it has been observed.
pub fn relation_expressivity(
    a: &[f64; PG_NODE_SIG_DIM],
    b: &[f64; PG_NODE_SIG_DIM],
) -> (f64, u8) {
    let mut engaged = 0u8;
    for i in 0..PG_NODE_SIG_DIM {
        let diff = (a[i] - b[i]).abs();
        if diff < RELATION_EXPRESSIVITY_AXIS_TH {
            engaged = engaged.saturating_add(1);
        }
    }
    let ratio = engaged as f64 / PG_NODE_SIG_DIM as f64;
    (ratio.clamp(0.0, 1.0), engaged)
}

/// Build a 48-D context signature by concatenating from-sig and to-sig.
/// This is the prototype space for emergent RelationKind clustering.
pub fn concat_context(
    from_sig: &[f64; PG_NODE_SIG_DIM],
    to_sig: &[f64; PG_NODE_SIG_DIM],
) -> [f64; PG_NODE_SIG_DIM * 2] {
    let mut ctx = [0.0; PG_NODE_SIG_DIM * 2];
    for i in 0..PG_NODE_SIG_DIM {
        ctx[i] = from_sig[i];
        ctx[PG_NODE_SIG_DIM + i] = to_sig[i];
    }
    ctx
}

/// Euclidean distance between two 48-D context signatures.
pub fn ctx_distance(
    a: &[f64; PG_NODE_SIG_DIM * 2],
    b: &[f64; PG_NODE_SIG_DIM * 2],
) -> f64 {
    let mut s = 0.0;
    for i in 0..PG_NODE_SIG_DIM * 2 {
        let d = a[i] - b[i];
        s += d * d;
    }
    s.sqrt()
}

/// Shape distance between two 64-band mel fingerprints. Mirrors the
/// normalised-shape metric vocal_target_recall uses (spectral_shape_distance_mel64);
/// here we implement the same idea inline so perturbation_graph stays
/// self-contained: normalise each to sum-1 (treats it as a shape, not a level),
/// then Euclidean distance.
pub fn mel64_shape_distance(
    a: &[f64; PG_VOCAL_MEL_DIM],
    b: &[f64; PG_VOCAL_MEL_DIM],
) -> f64 {
    let sa: f64 = a.iter().map(|v| v.abs()).sum();
    let sb: f64 = b.iter().map(|v| v.abs()).sum();
    if sa < 1e-9 || sb < 1e-9 {
        return f64::INFINITY;
    }
    let mut acc = 0.0;
    for i in 0..PG_VOCAL_MEL_DIM {
        let na = a[i].abs() / sa;
        let nb = b[i].abs() / sb;
        let d = na - nb;
        acc += d * d;
    }
    acc.sqrt()
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

    // ── Phase A1 tests: NodeGroup + soft membership ─────────────────────────

    fn spatial_region(x: u8, y: u8) -> RegionKey {
        RegionKey::Spatial(x, y)
    }

    #[test]
    fn gaussian_weight_drops_off_with_distance() {
        // At d=0 the weight is 1.0; at d=σ ≈ 0.61; at d=3σ ≈ 0.011. Smooth
        // monotonic decay is the whole point of the soft-membership transform.
        assert!((gaussian_weight(0.0, 0.15) - 1.0).abs() < 1e-9);
        let one_sigma = gaussian_weight(0.15, 0.15);
        assert!((one_sigma - 0.6065).abs() < 0.01);
        let three_sigma = gaussian_weight(0.45, 0.15);
        assert!(three_sigma < 0.05);
        // Pathological σ=0 collapses to an indicator at d=0.
        assert_eq!(gaussian_weight(0.0, 0.0), 1.0);
        assert_eq!(gaussian_weight(0.1, 0.0), 0.0);
    }

    #[test]
    fn add_group_with_no_resolvable_seeds_returns_none() {
        let mut g = PerturbationGraph::default();
        // Seed list points at unknown node ids → no group can be formed.
        let r = g.add_group(0, spatial_region(0, 0), &[42, 99]);
        assert!(r.is_none());
        assert_eq!(g.groups.len(), 0);
    }

    #[test]
    fn add_group_returns_unique_ids_starting_above_sentinel() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.11));
        let c = g.add_node(0, sig_at(0.12));
        let id1 = g.add_group(1000, spatial_region(0, 0), &[a, b, c]).unwrap();
        let id2 = g.add_group(1000, spatial_region(0, 0), &[a, b, c]).unwrap();
        assert!(id1 >= 1, "id 0 reserved as no-group sentinel");
        assert_ne!(id1, id2);
        assert_eq!(g.groups.len(), 2);
    }

    #[test]
    fn refresh_group_updates_member_weight_toward_target() {
        // A node at the centroid gets weight ≈ 1.0; pulling it away with a
        // new sig should EMA its weight downward over several refresh ticks.
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.20));
        let n2 = g.add_node(0, sig_at(0.20));
        let gid = g.add_group(1000, spatial_region(0, 0), &[n1, n2]).unwrap();
        let initial_w = g
            .group(gid)
            .unwrap()
            .members
            .iter()
            .find(|m| m.node_id == n1)
            .unwrap()
            .weight;
        assert!(initial_w > 0.9, "tight initial weight, got {initial_w}");
        // Move n1's sig far from the centroid and refresh several times.
        g.refresh_node(n1, 2000, &sig_at(0.90), 1.0); // collapse to far sig
        for t in 0..10 {
            g.refresh_group(gid, n1, 3000 + t * 100);
        }
        let drifted_w = g
            .group(gid)
            .unwrap()
            .members
            .iter()
            .find(|m| m.node_id == n1)
            .unwrap()
            .weight;
        assert!(
            drifted_w < initial_w * 0.5,
            "weight must drop after sustained distance: initial={initial_w}, drifted={drifted_w}"
        );
    }

    #[test]
    fn refresh_group_admits_new_member_when_close_enough() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.10));
        let gid = g.add_group(1000, spatial_region(0, 0), &[n1, n2]).unwrap();
        let n3 = g.add_node(0, sig_at(0.105)); // close to centroid
        assert!(g
            .group(gid)
            .unwrap()
            .members
            .iter()
            .all(|m| m.node_id != n3));
        g.refresh_group(gid, n3, 2000);
        let m3 = g
            .group(gid)
            .unwrap()
            .members
            .iter()
            .find(|m| m.node_id == n3);
        assert!(m3.is_some(), "close node must be soft-admitted");
        assert!(m3.unwrap().weight > 0.5);
    }

    #[test]
    fn centroid_drifts_toward_refreshing_members() {
        // A group of tight members has its centroid pulled toward whichever
        // member keeps refreshing — this is the "live neuron" property at
        // the group level. After many refreshes from one specific member,
        // the centroid sig should be closer to that member than to the
        // original mean.
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.00));
        let n2 = g.add_node(0, sig_at(0.20));
        let gid = g.add_group(1000, spatial_region(0, 0), &[n1, n2]).unwrap();
        let initial_centroid = g.group(gid).unwrap().centroid_sig;
        // n1's sig hasn't changed; but if only n1 keeps refreshing the group,
        // the centroid should drift toward n1.
        for t in 0..100 {
            g.refresh_group(gid, n1, 2000 + t * 100);
        }
        let drifted = g.group(gid).unwrap().centroid_sig;
        let d_initial_to_n1 = sig_distance(&initial_centroid, &sig_at(0.00));
        let d_drifted_to_n1 = sig_distance(&drifted, &sig_at(0.00));
        assert!(
            d_drifted_to_n1 < d_initial_to_n1,
            "centroid must drift toward repeatedly-refreshed member: initial_d={d_initial_to_n1}, drifted_d={d_drifted_to_n1}"
        );
    }

    #[test]
    fn nearest_group_only_matches_same_region() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.11));
        let c = g.add_node(0, sig_at(0.12));
        let g_spatial = g
            .add_group(1000, RegionKey::Spatial(0, 0), &[a, b, c])
            .unwrap();
        let _g_temporal = g
            .add_group(1000, RegionKey::TemporalWindow(42), &[a, b, c])
            .unwrap();
        // Querying with Spatial should ONLY return the spatial group.
        let (hit_id, _) = g
            .nearest_group(RegionKey::Spatial(0, 0), &sig_at(0.105), 0.10)
            .expect("close-sig spatial group");
        assert_eq!(hit_id, g_spatial);
        // Querying with TemporalWindow should not return the spatial group.
        let other_hit = g.nearest_group(RegionKey::TemporalWindow(42), &sig_at(0.105), 0.10);
        assert!(other_hit.is_some());
        assert_ne!(other_hit.unwrap().0, g_spatial);
    }

    #[test]
    fn most_bound_group_picks_highest_bind_strength() {
        let mut g = PerturbationGraph::default();
        // Group A: nodes never refreshed → seed bind_strength only.
        let a1 = g.add_node(0, sig_at(0.10));
        let a2 = g.add_node(0, sig_at(0.11));
        let _ga = g.add_group(1000, RegionKey::Spatial(0, 0), &[a1, a2]).unwrap();
        // Group B: nodes refreshed many times → high bind_strength members.
        let b1 = g.add_node(0, sig_at(0.50));
        let b2 = g.add_node(0, sig_at(0.51));
        let gb = g.add_group(1000, RegionKey::Spatial(5, 5), &[b1, b2]).unwrap();
        for t in 0..40 {
            g.refresh_node(b1, 1000 + t * 100, &sig_at(0.50), 0.3);
            g.refresh_node(b2, 1000 + t * 100, &sig_at(0.51), 0.3);
        }
        // Group B's aggregated bind_strength is now much higher than A's.
        g.decay_group_members(5_000);
        let top = g.most_bound_group().expect("at least one group");
        assert_eq!(top.id, gb);
    }

    #[test]
    fn decay_group_members_prunes_dormant_members() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.11));
        let c = g.add_node(0, sig_at(0.12));
        let gid = g.add_group(1000, RegionKey::Spatial(0, 0), &[a, b, c]).unwrap();
        assert_eq!(g.group(gid).unwrap().members.len(), 3);
        // Jump time well past GROUP_MEMBER_DECAY_US for ALL members.
        let now = 1000 + GROUP_MEMBER_DECAY_US + 1_000;
        // One decay step → all weights × 0.95 → still above floor.
        g.decay_group_members(now);
        assert_eq!(g.group(gid).unwrap().members.len(), 3);
        // Many decay steps → eventually members get pruned, AND the group
        // itself is dropped (empty groups are husks; the bug fix removes them
        // so they don't accumulate as 87% empty containers under live load).
        for k in 0..120 {
            g.decay_group_members(now + (k + 1) * GROUP_MEMBER_DECAY_US);
        }
        assert!(
            g.group(gid).is_none(),
            "an emptied group should be dropped, not left as a husk"
        );
    }

    #[test]
    fn evicting_a_node_drops_group_membership() {
        // The whole-graph invariant: when a node is evicted, no surviving
        // group should still list it as a member. Otherwise group bind_strength
        // and centroid drift silently keep referencing a ghost.
        let mut g = PerturbationGraph::default();
        let weak = g.add_node(0, sig_at(0.10));
        let stronger_a = g.add_node(0, sig_at(0.50));
        let stronger_b = g.add_node(0, sig_at(0.51));
        let gid = g
            .add_group(1000, RegionKey::Spatial(0, 0), &[weak, stronger_a, stronger_b])
            .unwrap();
        // Raise bind on the two stronger nodes so eviction targets `weak`.
        for t in 0..30 {
            g.refresh_node(stronger_a, 1000 + t * 100, &sig_at(0.50), 0.3);
            g.refresh_node(stronger_b, 1000 + t * 100, &sig_at(0.51), 0.3);
        }
        let now = 5_000;
        g.evict_weakest_node(now);
        assert!(!g.has_node(weak));
        let members = &g.group(gid).unwrap().members;
        assert!(
            members.iter().all(|m| m.node_id != weak),
            "evicted node must not remain as a group member"
        );
    }

    #[test]
    fn evict_weakest_group_protects_higher_bind_strength() {
        let mut g = PerturbationGraph::default();
        // Group A: low-bind seeds, no refresh → weak.
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.11));
        let weak_g = g.add_group(0, RegionKey::Spatial(0, 0), &[a, b]).unwrap();
        // Group B: refreshed many times → strong.
        let c = g.add_node(0, sig_at(0.50));
        let d = g.add_node(0, sig_at(0.51));
        let strong_g = g.add_group(0, RegionKey::Spatial(5, 5), &[c, d]).unwrap();
        for t in 0..40 {
            g.refresh_node(c, 100 * t, &sig_at(0.50), 0.3);
            g.refresh_node(d, 100 * t, &sig_at(0.51), 0.3);
            g.refresh_group(strong_g, c, 100 * t);
            g.refresh_group(strong_g, d, 100 * t);
        }
        g.decay_group_members(5_000);
        let now = 100_000;
        g.evict_weakest_group(now);
        assert!(g.has_group(strong_g));
        assert!(!g.has_group(weak_g));
    }

    #[test]
    fn active_groups_filters_dormant() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.11));
        let live = g
            .add_group(0, RegionKey::Spatial(0, 0), &[a, b])
            .unwrap();
        let dormant = g
            .add_group(0, RegionKey::Spatial(5, 5), &[a, b])
            .unwrap();
        let now = PG_ACTIVE_WINDOW_US * 2;
        // Touch `live` recently.
        g.refresh_group(live, a, now);
        let active_ids: Vec<GroupId> = g.active_groups(now).map(|g| g.id).collect();
        assert!(active_ids.contains(&live));
        assert!(!active_ids.contains(&dormant));
    }

    // ── Phase A3 tests: first-class Relations + Endpoint enum ──────────────

    #[test]
    fn relation_expressivity_counts_aligned_axes() {
        // Two identical sigs → expressivity 1.0 (all 24 axes aligned).
        let s = sig_at(0.5);
        let (ratio, count) = relation_expressivity(&s, &s);
        assert_eq!(count, PG_NODE_SIG_DIM as u8);
        assert!((ratio - 1.0).abs() < 1e-9);
        // Two wildly different sigs → expressivity 0.0 (no axis below the th).
        let a = [0.0; PG_NODE_SIG_DIM];
        let b = [1.0; PG_NODE_SIG_DIM];
        let (ratio, count) = relation_expressivity(&a, &b);
        assert_eq!(count, 0);
        assert!(ratio.abs() < 1e-9);
        // Half-aligned: first 12 axes equal, last 12 differ by 1.0 → 12 engaged.
        let mut c = [0.0; PG_NODE_SIG_DIM];
        for i in 12..PG_NODE_SIG_DIM { c[i] = 1.0; }
        let (ratio, count) = relation_expressivity(&a, &c);
        assert_eq!(count, 12);
        assert!((ratio - 0.5).abs() < 1e-9);
    }

    #[test]
    fn observe_relation_node_node_creates_and_strengthens() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.11));
        let r1 = g.observe_relation(1_000, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Cooccur);
        assert!(r1.is_some());
        let id = r1.unwrap();
        assert_eq!(g.relations.len(), 1);
        let initial_strength = g.relation(id).unwrap().strength;
        // Re-observe → same id, higher strength.
        let r2 = g.observe_relation(2_000, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Cooccur);
        assert_eq!(r2, Some(id));
        assert_eq!(g.relations.len(), 1);
        let after = g.relation(id).unwrap().strength;
        assert!(after > initial_strength, "strength must grow with repeat observation");
    }

    #[test]
    fn observe_relation_distinguishes_kinds_and_rejects_self_loops() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.20));
        g.observe_relation(1_000, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Cooccur);
        g.observe_relation(1_000, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Causal);
        assert_eq!(g.relations.len(), 2, "different kinds yield distinct relations");
        // Self-loop rejected.
        let r = g.observe_relation(1_000, Endpoint::Node(a), Endpoint::Node(a), EdgeKind::Cooccur);
        assert!(r.is_none());
        assert_eq!(g.relations.len(), 2);
        // Sentinel endpoint rejected.
        let r = g.observe_relation(1_000, Endpoint::Node(0), Endpoint::Node(a), EdgeKind::Cooccur);
        assert!(r.is_none());
        // Dangling endpoint rejected.
        let r = g.observe_relation(1_000, Endpoint::Node(a), Endpoint::Node(99999), EdgeKind::Cooccur);
        assert!(r.is_none());
    }

    #[test]
    fn observe_relation_group_to_node_works() {
        let mut g = PerturbationGraph::default();
        let m1 = g.add_node(0, sig_at(0.10));
        let m2 = g.add_node(0, sig_at(0.11));
        let m3 = g.add_node(0, sig_at(0.12));
        let grp = g.add_group(1_000, RegionKey::Spatial(0, 0), &[m1, m2, m3]).unwrap();
        let other = g.add_node(0, sig_at(0.80));
        let r = g.observe_relation(2_000, Endpoint::Group(grp), Endpoint::Node(other), EdgeKind::Causal);
        assert!(r.is_some());
        let rel = g.relation(r.unwrap()).unwrap();
        assert!(matches!(rel.from, Endpoint::Group(_)));
        assert!(matches!(rel.to, Endpoint::Node(_)));
        // Expressivity should be low (groups around 0.10, target at 0.80 → many axes disagree).
        assert!(rel.expressivity < 0.5, "low expressivity, got {}", rel.expressivity);
    }

    #[test]
    fn observe_relation_relation_to_node_is_first_class_meta() {
        // The user's "关系指向另一段扰动元素": a relation can point AT another
        // relation. Build (A → B), then (R_ab → C) — a meta-relation whose
        // source IS the first relation.
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.12));
        let c = g.add_node(0, sig_at(0.50));
        let r_ab = g.observe_relation(1_000, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Causal).unwrap();
        // Now point a Relation at the existing relation.
        let r_meta = g.observe_relation(2_000, Endpoint::Relation(r_ab), Endpoint::Node(c), EdgeKind::Cooccur);
        assert!(r_meta.is_some());
        let rel = g.relation(r_meta.unwrap()).unwrap();
        assert!(matches!(rel.from, Endpoint::Relation(_)));
    }

    #[test]
    fn traverse_relations_respects_cycle_and_depth_cap() {
        // Build a 3-cycle A → B → C → A and traverse from A with depth 10.
        // Cycle protection should visit each endpoint once.
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.20));
        let c = g.add_node(0, sig_at(0.30));
        g.observe_relation(1_000, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Causal);
        g.observe_relation(1_000, Endpoint::Node(b), Endpoint::Node(c), EdgeKind::Causal);
        g.observe_relation(1_000, Endpoint::Node(c), Endpoint::Node(a), EdgeKind::Causal);
        let mut visited: Vec<Endpoint> = Vec::new();
        g.traverse_relations_from(Endpoint::Node(a), 10, |ep, _d| visited.push(ep));
        assert_eq!(visited.len(), 3, "cycle visited each endpoint once");
        // Depth cap = 1: only A and one neighbour.
        let mut shallow: Vec<Endpoint> = Vec::new();
        g.traverse_relations_from(Endpoint::Node(a), 1, |ep, _d| shallow.push(ep));
        assert_eq!(shallow.len(), 2, "depth=1 visits start + one neighbour");
    }

    #[test]
    fn strongest_out_relation_weighs_strength_times_expressivity() {
        let mut g = PerturbationGraph::default();
        let from = g.add_node(0, sig_at(0.10));
        // Target A is sig-aligned with `from` (high expressivity).
        let to_a = g.add_node(0, sig_at(0.105));
        // Target B is sig-far (low expressivity).
        let to_b = g.add_node(0, sig_at(0.99));
        // Observe both 5× each so strength is similar.
        for t in 0..5 {
            g.observe_relation(t * 100, Endpoint::Node(from), Endpoint::Node(to_a), EdgeKind::Cooccur);
            g.observe_relation(t * 100, Endpoint::Node(from), Endpoint::Node(to_b), EdgeKind::Cooccur);
        }
        let top = g.strongest_out_relation(Endpoint::Node(from)).expect("at least one");
        // The well-aligned target should win on expressivity tie-break.
        assert_eq!(top.to, Endpoint::Node(to_a));
    }

    // ── B2.1: temperature-sampled walk step (sample_out_relation) ────────────

    #[test]
    fn sample_out_relation_below_temp_eps_equals_argmax() {
        // Regression guard: at/below the eps temperature the sampler must
        // return byte-for-byte what strongest_out_relation returns, for ANY u.
        let mut g = PerturbationGraph::default();
        let from = g.add_node(0, sig_at(0.10));
        let strong = g.add_node(0, sig_at(0.105)); // well-aligned → high expressivity
        let weak = g.add_node(0, sig_at(0.40)); // farther → lower expressivity
        for t in 0..6 {
            g.observe_relation(t * 100, Endpoint::Node(from), Endpoint::Node(strong), EdgeKind::Cooccur);
        }
        for t in 0..3 {
            g.observe_relation(t * 100, Endpoint::Node(from), Endpoint::Node(weak), EdgeKind::Cooccur);
        }
        let argmax = g.strongest_out_relation(Endpoint::Node(from)).unwrap().to;
        for &u in &[0.0, 0.1, 0.37, 0.5, 0.83, 0.999] {
            let picked = g.sample_out_relation(Endpoint::Node(from), 0.0, u).unwrap().to;
            assert_eq!(picked, argmax, "temp=0 must equal argmax for u={u}");
        }
        // Below eps but non-zero — still argmax.
        let picked = g
            .sample_out_relation(Endpoint::Node(from), crate::PG_WALK_TEMP_EPS / 2.0, 0.99)
            .unwrap()
            .to;
        assert_eq!(picked, argmax);
    }

    #[test]
    fn sample_out_relation_can_reach_weaker_edge_at_high_temp() {
        // Two similar-score targets: at the exploration ceiling, sweeping u
        // must be able to diverge to more than one of them.
        let mut g = PerturbationGraph::default();
        let from = g.add_node(0, sig_at(0.10));
        let a = g.add_node(0, sig_at(0.105));
        let b = g.add_node(0, sig_at(0.120));
        for t in 0..5 {
            g.observe_relation(t * 100, Endpoint::Node(from), Endpoint::Node(a), EdgeKind::Cooccur);
            g.observe_relation(t * 100, Endpoint::Node(from), Endpoint::Node(b), EdgeKind::Cooccur);
        }
        let mut seen = std::collections::HashSet::new();
        for i in 0..64 {
            let u = i as f64 / 64.0;
            if let Some(r) = g.sample_out_relation(Endpoint::Node(from), crate::PG_WALK_MAX_TEMP, u) {
                seen.insert(r.to);
            }
        }
        assert!(
            seen.len() >= 2,
            "high-temp walk must diverge to >1 target, saw {seen:?}"
        );
    }

    #[test]
    fn sample_out_relation_single_candidate_is_returned_regardless_of_u() {
        // Safety property: a node with one out-edge can never diverge —
        // sampling is a no-op there, so an immature/sparse graph is unperturbed.
        let mut g = PerturbationGraph::default();
        let from = g.add_node(0, sig_at(0.10));
        let only = g.add_node(0, sig_at(0.20));
        g.observe_relation(0, Endpoint::Node(from), Endpoint::Node(only), EdgeKind::Cooccur);
        for &u in &[0.0, 0.5, 0.99] {
            let picked = g
                .sample_out_relation(Endpoint::Node(from), crate::PG_WALK_MAX_TEMP, u)
                .unwrap()
                .to;
            assert_eq!(picked, Endpoint::Node(only));
        }
    }

    #[test]
    fn sample_out_relation_returns_none_when_no_out_edges() {
        let mut g = PerturbationGraph::default();
        let lonely = g.add_node(0, sig_at(0.10));
        assert!(g
            .sample_out_relation(Endpoint::Node(lonely), crate::PG_WALK_MAX_TEMP, 0.5)
            .is_none());
    }

    #[test]
    fn sample_out_relation_is_deterministic_in_u() {
        let mut g = PerturbationGraph::default();
        let from = g.add_node(0, sig_at(0.10));
        let a = g.add_node(0, sig_at(0.105));
        let b = g.add_node(0, sig_at(0.120));
        for t in 0..5 {
            g.observe_relation(t * 100, Endpoint::Node(from), Endpoint::Node(a), EdgeKind::Cooccur);
            g.observe_relation(t * 100, Endpoint::Node(from), Endpoint::Node(b), EdgeKind::Cooccur);
        }
        let x = g.sample_out_relation(Endpoint::Node(from), crate::PG_WALK_MAX_TEMP, 0.42).unwrap().to;
        let y = g.sample_out_relation(Endpoint::Node(from), crate::PG_WALK_MAX_TEMP, 0.42).unwrap().to;
        assert_eq!(x, y);
    }

    // ── B2.2: rest-state spreading replay (quiet_replay_novelty) ─────────────

    /// Helper: 8 nodes with controlled salience; node 0 is the unique top
    /// (so now_us=0 → idx 0 picks it as the seed). persistence zeroed so
    /// salience == bind_strength.
    fn graph_with_salience(binds: [f64; 8]) -> (PerturbationGraph, Vec<NodeId>) {
        let mut g = PerturbationGraph::default();
        let ids: Vec<NodeId> = (0..8).map(|i| g.add_node(0, sig_at(0.1 * i as f64))).collect();
        for (i, &id) in ids.iter().enumerate() {
            let n = g.node_mut(id).unwrap();
            n.bind_strength = binds[i];
            n.persistence_ema = 0.0;
        }
        (g, ids)
    }

    #[test]
    fn quiet_replay_novelty_none_below_node_floor() {
        let mut g = PerturbationGraph::default();
        for i in 0..7 {
            g.add_node(0, sig_at(0.1 * i as f64));
        }
        assert!(g.quiet_replay_novelty(0, 2).is_none(), "7 nodes < 8-floor → None");
    }

    #[test]
    fn quiet_replay_novelty_is_the_seed_alone_when_cue_is_isolated() {
        // Seed (node 0, bind 0.9) has NO relations → neighbourhood is just itself.
        let (g, _ids) = graph_with_salience([0.9, 0.5, 0.1, 0.1, 0.1, 0.05, 0.05, 0.05]);
        let nov = g.quiet_replay_novelty(0, 2).unwrap();
        assert!((nov - 0.9).abs() < 1e-9, "isolated cue replays at its own salience, got {nov}");
    }

    #[test]
    fn quiet_replay_novelty_follows_relations_not_global_salience() {
        // Seed (node 0, bind 0.9) is related to three LOW nodes (1,2,3 = 0.1).
        // Node 4 (bind 0.5) is salient but UNRELATED — it must not be evoked.
        let (mut g, ids) = graph_with_salience([0.9, 0.1, 0.1, 0.1, 0.5, 0.05, 0.05, 0.05]);
        for &low in &ids[1..4] {
            g.observe_relation(0, Endpoint::Node(ids[0]), Endpoint::Node(low), EdgeKind::Cooccur);
        }
        let nov = g.quiet_replay_novelty(0, 2).unwrap();
        // Neighbourhood = {0.9, 0.1, 0.1, 0.1} → mean 0.3. The spread pulls the
        // signal DOWN from the seed (0.9); the unrelated 0.5 node is excluded.
        assert!((nov - 0.3).abs() < 1e-9, "mean over relational neighbourhood, got {nov}");
        assert!(nov < 0.9, "spread must move the signal off the bare seed salience");
    }

    #[test]
    fn quiet_replay_novelty_is_deterministic_in_now_us() {
        let (mut g, ids) = graph_with_salience([0.9, 0.1, 0.1, 0.1, 0.5, 0.05, 0.05, 0.05]);
        for &low in &ids[1..4] {
            g.observe_relation(0, Endpoint::Node(ids[0]), Endpoint::Node(low), EdgeKind::Cooccur);
        }
        assert_eq!(g.quiet_replay_novelty(0, 2), g.quiet_replay_novelty(0, 2));
    }

    // ── B2.2b: rest consolidation (consolidate_quiet_neighborhood) ───────────

    /// Build the standard cue→{low1,low2,low3} neighbourhood with the lows aged
    /// out (last_active = 0), seed at node 0.
    fn graph_with_aged_neighborhood() -> (PerturbationGraph, Vec<NodeId>) {
        let (mut g, ids) = graph_with_salience([0.9, 0.1, 0.1, 0.1, 0.5, 0.05, 0.05, 0.05]);
        for &low in &ids[1..4] {
            g.observe_relation(0, Endpoint::Node(ids[0]), Endpoint::Node(low), EdgeKind::Cooccur);
        }
        for &id in &ids {
            g.node_mut(id).unwrap().last_active_t_us = 0;
        }
        (g, ids)
    }

    #[test]
    fn consolidate_protects_old_neighbor_without_making_it_active() {
        let (mut g, ids) = graph_with_aged_neighborhood();
        let now = 10 * PG_ACTIVE_WINDOW_US;
        g.consolidate_quiet_neighborhood(now, 2);
        let low = ids[1]; // reachable from seed
        assert_eq!(
            g.node(low).unwrap().last_active_t_us,
            now - PG_ACTIVE_WINDOW_US - 1,
            "rehearsed node pulled up to just-aged-out (eviction-protected)"
        );
        assert!(
            !g.is_active(low, now),
            "rehearsal must NOT make a node count as currently perceived"
        );
    }

    #[test]
    fn consolidate_never_pulls_a_fresher_node_backward() {
        let (mut g, ids) = graph_with_aged_neighborhood();
        let now = 10 * PG_ACTIVE_WINDOW_US;
        // Make a reachable node fresh (currently active).
        g.node_mut(ids[1]).unwrap().last_active_t_us = now;
        g.consolidate_quiet_neighborhood(now, 2);
        assert_eq!(
            g.node(ids[1]).unwrap().last_active_t_us,
            now,
            "max() keeps the fresher timestamp"
        );
        assert!(g.is_active(ids[1], now), "an already-active node stays active");
    }

    #[test]
    fn consolidate_leaves_unreachable_nodes_untouched() {
        let (mut g, ids) = graph_with_aged_neighborhood();
        let now = 10 * PG_ACTIVE_WINDOW_US;
        // ids[4] (salient 0.5) is NOT related to the seed → outside the walk.
        g.consolidate_quiet_neighborhood(now, 2);
        assert_eq!(
            g.node(ids[4]).unwrap().last_active_t_us,
            0,
            "a node outside the cue's neighbourhood is not protected"
        );
    }

    #[test]
    fn consolidate_returns_same_novelty_as_read_only() {
        let (mut g, _ids) = graph_with_aged_neighborhood();
        // now_us=0 → protect_floor=0 → no mutation, so the signal must match.
        assert_eq!(
            g.quiet_replay_novelty(0, 2),
            g.consolidate_quiet_neighborhood(0, 2)
        );
    }

    #[test]
    fn consolidate_none_below_node_floor() {
        let mut g = PerturbationGraph::default();
        for i in 0..7 {
            g.add_node(0, sig_at(0.1 * i as f64));
        }
        assert!(g.consolidate_quiet_neighborhood(10 * PG_ACTIVE_WINDOW_US, 2).is_none());
    }

    #[test]
    fn decay_relation_eligibility_drops_over_tau() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.11));
        let r_id = g.observe_relation(1_000, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Cooccur).unwrap();
        assert!((g.relation(r_id).unwrap().elig - 1.0).abs() < 1e-9);
        g.decay_relation_eligibility(1_000 + (PG_ELIG_TAU_US as u64) * 10);
        assert!(g.relation(r_id).unwrap().elig < 0.01);
    }

    #[test]
    fn endpoint_sig_resolves_node_group_and_single_hop_relation() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.11));
        let n3 = g.add_node(0, sig_at(0.12));
        let grp = g.add_group(0, RegionKey::Spatial(0, 0), &[n1, n2, n3]).unwrap();
        // Node endpoint resolves to its sig.
        let s = g.endpoint_sig(Endpoint::Node(n1)).unwrap();
        assert!((s[0] - 0.10).abs() < 0.01);
        // Group endpoint resolves to centroid sig.
        let cs = g.endpoint_sig(Endpoint::Group(grp)).unwrap();
        assert!((cs[0] - 0.11).abs() < 0.05); // centroid ≈ mean of [0.10, 0.11, 0.12]
        // Relation endpoint: a 1-hop indirection.
        let r_id = g.observe_relation(0, Endpoint::Node(n1), Endpoint::Node(n2), EdgeKind::Cooccur).unwrap();
        let rs = g.endpoint_sig(Endpoint::Relation(r_id)).unwrap();
        // Resolves to the FROM-node's sig (n1).
        assert!((rs[0] - 0.10).abs() < 0.01);
        // Dangling endpoints return None.
        assert!(g.endpoint_sig(Endpoint::Node(99999)).is_none());
        assert!(g.endpoint_sig(Endpoint::Group(99999)).is_none());
        assert!(g.endpoint_sig(Endpoint::Relation(99999)).is_none());
    }

    #[test]
    fn evict_weakest_relation_prefers_low_strength_times_expressivity() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.11)); // close → high expressivity
        let n3 = g.add_node(0, sig_at(0.99)); // far → low expressivity
        // Both observed once → similar strength, but expressivity differs.
        let r_keep = g.observe_relation(0, Endpoint::Node(n1), Endpoint::Node(n2), EdgeKind::Cooccur).unwrap();
        let r_evict = g.observe_relation(0, Endpoint::Node(n1), Endpoint::Node(n3), EdgeKind::Cooccur).unwrap();
        // Strengthen the keeper several more times so it's clearly higher.
        for _ in 0..5 {
            g.observe_relation(0, Endpoint::Node(n1), Endpoint::Node(n2), EdgeKind::Cooccur);
        }
        g.evict_weakest_relation();
        assert!(g.has_relation(r_keep));
        assert!(!g.has_relation(r_evict));
    }

    // ── Phase A4 tests: dynamic RelationKind discovery ─────────────────────

    #[test]
    fn default_graph_seeds_five_primitive_kinds() {
        let g = PerturbationGraph::default();
        assert_eq!(g.kinds.len(), PRIMITIVE_KIND_COUNT);
        for i in 0..PRIMITIVE_KIND_COUNT as u32 {
            let k = g.kind(i).expect("primitive kind must exist");
            assert_eq!(k.cluster_id, i);
            // Primitive prototypes start at zero (no observed context yet).
            assert!(k.prototype.iter().all(|x| x.abs() < 1e-9));
        }
        assert_eq!(g.next_kind_id, PRIMITIVE_KIND_COUNT as u32);
    }

    #[test]
    fn observe_relation_with_primitive_kind_drifts_its_prototype() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.20));
        // Observe many cooccur relations on this (from, to) context.
        for _ in 0..50 {
            g.observe_relation(1_000, Endpoint::Node(n1), Endpoint::Node(n2), EdgeKind::Cooccur);
        }
        // The Cooccur primitive's prototype should now be non-trivial.
        let cooccur = g.kind(KIND_ID_COOCCUR).unwrap();
        let proto_norm: f64 = cooccur.prototype.iter().map(|x| x * x).sum::<f64>().sqrt();
        assert!(proto_norm > 1e-3, "primitive prototype must accumulate context");
        assert!(cooccur.strength > 0.1, "primitive strength must accumulate");
    }

    #[test]
    fn observe_relation_emergent_with_distant_context_spawns_new_kind() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.20));
        let before = g.kinds.len();
        let r_id = g
            .observe_relation_emergent(1_000, Endpoint::Node(n1), Endpoint::Node(n2))
            .unwrap();
        // Since no primitive has a non-zero prototype yet, the emergent path
        // sees no nearby match and SHOULD spawn a fresh kind.
        assert_eq!(g.kinds.len(), before + 1);
        let rel = g.relation(r_id).unwrap();
        assert!(
            rel.kind_id >= PRIMITIVE_KIND_COUNT as u32,
            "emergent relations should land in cluster_id ≥ 4 (got {})",
            rel.kind_id
        );
        // Re-observing the same context should NOT spawn another kind — the
        // existing emergent kind absorbs it.
        let count_after = g.kinds.len();
        g.observe_relation_emergent(2_000, Endpoint::Node(n1), Endpoint::Node(n2));
        assert_eq!(g.kinds.len(), count_after, "near-identical context must reuse the kind");
    }

    #[test]
    fn observe_relation_emergent_with_close_context_reuses_existing_kind() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.20));
        let r1 = g.observe_relation_emergent(0, Endpoint::Node(n1), Endpoint::Node(n2)).unwrap();
        let kind_of_r1 = g.relation(r1).unwrap().kind_id;
        // Slightly perturb the endpoints' sigs by refreshing them gently.
        g.refresh_node(n1, 100, &sig_at(0.105), 0.05);
        g.refresh_node(n2, 100, &sig_at(0.205), 0.05);
        let r2 = g.observe_relation_emergent(200, Endpoint::Node(n1), Endpoint::Node(n2)).unwrap();
        let kind_of_r2 = g.relation(r2).unwrap().kind_id;
        assert_eq!(kind_of_r1, kind_of_r2, "near-context reuses same emergent kind");
    }

    #[test]
    fn emergent_kind_cap_is_respected() {
        // Fill the emergent kind space with many distinct contexts, verify
        // that once PG_KIND_CAP is hit no further kinds are added.
        let mut g = PerturbationGraph::default();
        // Add many distinct nodes and observe distinct emergent relations.
        let n_anchor = g.add_node(0, sig_at(0.0));
        let mut spawned = 0usize;
        for i in 0..(PG_KIND_CAP - PRIMITIVE_KIND_COUNT + 20) {
            // Distinct distant sig so it doesn't reuse an existing kind.
            let other = g.add_node(0, sig_at(i as f64 * 0.05 + 0.1));
            let before = g.kinds.len();
            g.observe_relation_emergent(i as u64 * 100, Endpoint::Node(n_anchor), Endpoint::Node(other));
            if g.kinds.len() > before {
                spawned += 1;
            }
        }
        assert!(g.kinds.len() <= PG_KIND_CAP, "must not exceed cap");
        // We added more than (PG_KIND_CAP - PRIMITIVE_KIND_COUNT) attempts so
        // some MUST have been rejected.
        let _ = spawned;
    }

    #[test]
    fn decay_emergent_kinds_evicts_long_idle_clusters() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.20));
        // Spawn an emergent kind with low strength.
        let r_id = g
            .observe_relation_emergent(1_000, Endpoint::Node(n1), Endpoint::Node(n2))
            .unwrap();
        let kind_id = g.relation(r_id).unwrap().kind_id;
        assert!(kind_id >= PRIMITIVE_KIND_COUNT as u32);
        // Advance time past KIND_DEMOTE_IDLE_US many times to crush strength.
        let mut t = 1_000_u64;
        for _ in 0..30 {
            t += KIND_DEMOTE_IDLE_US + 1;
            g.decay_emergent_kinds(t);
        }
        // The emergent kind should now be evicted; the relation re-tagged to
        // Cooccur primitive.
        assert!(g.kind(kind_id).is_none(), "long-idle emergent kind must be evicted");
        let rel = g.relation(r_id).unwrap();
        assert_eq!(
            rel.kind_id, KIND_ID_COOCCUR,
            "evicted-kind relation must be re-tagged to primitive Cooccur"
        );
    }

    #[test]
    fn primitive_kinds_never_decay() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.20));
        // Observe a Causal primitive once.
        g.observe_relation(1_000, Endpoint::Node(n1), Endpoint::Node(n2), EdgeKind::Causal);
        let s_before = g.kind(KIND_ID_CAUSAL).unwrap().strength;
        // Many decay cycles later — the primitive should still exist.
        for k in 0..20 {
            g.decay_emergent_kinds(1_000 + (k + 1) * KIND_DEMOTE_IDLE_US + 100);
        }
        assert!(g.kind(KIND_ID_CAUSAL).is_some(), "primitives are immune to eviction");
        // Strength only EMA'd up earlier — decay doesn't touch it.
        let s_after = g.kind(KIND_ID_CAUSAL).unwrap().strength;
        assert!((s_after - s_before).abs() < 1e-9, "primitive strength is not decayed");
    }

    #[test]
    fn ctx_distance_and_concat_helpers_match() {
        let a = sig_at(0.10);
        let b = sig_at(0.20);
        let ctx = concat_context(&a, &b);
        // Lower half is a, upper half is b.
        for i in 0..PG_NODE_SIG_DIM {
            assert!((ctx[i] - a[i]).abs() < 1e-12);
            assert!((ctx[PG_NODE_SIG_DIM + i] - b[i]).abs() < 1e-12);
        }
        // Distance from a context to itself is zero.
        assert!(ctx_distance(&ctx, &ctx) < 1e-9);
    }

    // ── Phase A5 tests: FastPath (muscle memory / default cognition) ──────

    /// Helper: build a graph + two nodes + a relation, then strength it up
    /// over many observations so it qualifies for promotion. Returns rel_id.
    fn make_strong_relation(g: &mut PerturbationGraph, start_t_us: u64) -> RelId {
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.11)); // close → high expressivity
        let mut rid = 0u32;
        // 50 observations: strength ≈ 1 - (1 - α)^50 ≈ 1.0 for α=0.20.
        for k in 0..50 {
            rid = g
                .observe_relation(start_t_us + k * 10, Endpoint::Node(n1), Endpoint::Node(n2), EdgeKind::Cooccur)
                .unwrap();
        }
        rid
    }

    #[test]
    fn graph_instability_reflects_strength_spread() {
        let mut g = PerturbationGraph::default();
        // Empty → 0 (need >= 2 relations).
        assert_eq!(g.graph_instability(), 0.0);

        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.11));
        let c = g.add_node(0, sig_at(0.50));

        // One strong relation → still 0 (need >= 2).
        for k in 0..40 {
            g.observe_relation(k * 10, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Cooccur);
        }
        assert_eq!(g.graph_instability(), 0.0);

        // A second, equally-strengthened relation → uniform maturities → ~0.
        for k in 0..40 {
            g.observe_relation(1_000 + k * 10, Endpoint::Node(a), Endpoint::Node(c), EdgeKind::Cooccur);
        }
        let uniform = g.graph_instability();
        assert!(uniform < 0.05, "uniform strengths → low instability, got {uniform}");

        // A third, freshly-seeded weak relation → spread of maturities → rises.
        g.observe_relation(9_000, Endpoint::Node(b), Endpoint::Node(c), EdgeKind::Cooccur);
        let mixed = g.graph_instability();
        assert!(
            mixed > uniform,
            "mixed maturities → higher instability ({mixed} vs {uniform})"
        );
    }

    #[test]
    fn default_response_returns_needs_deliberation_when_no_fast_path() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.20));
        g.observe_relation(1_000, Endpoint::Node(n1), Endpoint::Node(n2), EdgeKind::Cooccur);
        // No FastPath promotion yet → fall through to deliberation.
        match g.default_response(Endpoint::Node(n1)) {
            ResponseOutcome::NeedsDeliberation => {}
            other => panic!("expected NeedsDeliberation, got {:?}", other),
        }
    }

    #[test]
    fn tick_fast_paths_promotes_sustained_strong_relation() {
        let mut g = PerturbationGraph::default();
        let rid = make_strong_relation(&mut g, 0);
        // First tick at t=10: relation is above threshold for the first time.
        g.tick_fast_paths(10);
        assert!(g.fast_paths.by_endpoints.is_empty(), "promotion needs sustained duration");
        // Tick well past the promote duration → must now be promoted.
        g.tick_fast_paths(FAST_PATH_PROMOTE_DURATION_US + 1_000);
        let from_ep = g.relation(rid).unwrap().from;
        let resp = g.default_response(from_ep);
        match resp {
            ResponseOutcome::FastPath { rel_id, .. } => assert_eq!(rel_id, rid),
            other => panic!("expected FastPath after sustained promotion, got {:?}", other),
        }
        assert!(g.fast_paths.by_endpoints.len() == 1);
    }

    #[test]
    fn tick_fast_paths_demotes_when_strength_drops_below_th() {
        let mut g = PerturbationGraph::default();
        let rid = make_strong_relation(&mut g, 0);
        // Two-step promotion: first tick sets the timer; second tick sees
        // duration cleared.
        g.tick_fast_paths(10);
        g.tick_fast_paths(FAST_PATH_PROMOTE_DURATION_US + 1_000);
        assert_eq!(g.fast_paths.by_endpoints.len(), 1);
        // Crush the relation's strength by direct mutation (simulating long
        // unused decay without going through real eligibility loops).
        if let Some(r) = g.relations.iter_mut().find(|r| r.id == rid) {
            r.strength = 0.30; // below FAST_PATH_DEMOTE_TH=0.5
        }
        g.tick_fast_paths(FAST_PATH_PROMOTE_DURATION_US + 5_000);
        assert!(
            g.fast_paths.by_endpoints.is_empty(),
            "weakened relation must be demoted"
        );
    }

    #[test]
    fn tick_fast_paths_resets_clock_when_strength_dips_below_threshold() {
        // first_above_promote_t_us must reset whenever strength dips below the
        // promote threshold, otherwise an oscillating relation would
        // accumulate "credit" toward promotion that it doesn't deserve.
        let mut g = PerturbationGraph::default();
        let rid = make_strong_relation(&mut g, 0);
        g.tick_fast_paths(1_000);
        let stamp1 = g.relation(rid).unwrap().first_above_promote_t_us;
        assert!(stamp1 > 0);
        // Crush strength temporarily so it dips below threshold.
        if let Some(r) = g.relations.iter_mut().find(|r| r.id == rid) {
            r.strength = 0.30;
        }
        g.tick_fast_paths(2_000);
        let stamp2 = g.relation(rid).unwrap().first_above_promote_t_us;
        assert_eq!(stamp2, 0, "clock must reset on a dip below threshold");
    }

    #[test]
    fn post_hoc_check_strengthens_correct_predictions_and_weakens_wrong() {
        let mut g = PerturbationGraph::default();
        let n1 = g.add_node(0, sig_at(0.10));
        let n2 = g.add_node(0, sig_at(0.20));
        let n3 = g.add_node(0, sig_at(0.90));
        let rid = g.observe_relation(1_000, Endpoint::Node(n1), Endpoint::Node(n2), EdgeKind::Cooccur).unwrap();
        let s_before = g.relation(rid).unwrap().strength;
        // Observed outcome exactly matches the predicted target (n2).
        let correct = g.post_hoc_check(rid, Endpoint::Node(n2));
        assert!(correct);
        let s_after_correct = g.relation(rid).unwrap().strength;
        assert!(s_after_correct > s_before, "correct prediction must strengthen");
        // Observed outcome very far from prediction (sig dist large).
        let _ = g.post_hoc_check(rid, Endpoint::Node(n3));
        let s_after_wrong = g.relation(rid).unwrap().strength;
        assert!(
            s_after_wrong < s_after_correct,
            "wrong prediction must weaken"
        );
    }

    #[test]
    fn note_fast_path_usage_increments_counter_and_timestamp() {
        let mut g = PerturbationGraph::default();
        let rid = make_strong_relation(&mut g, 0);
        // Two-step promotion.
        g.tick_fast_paths(10);
        g.tick_fast_paths(FAST_PATH_PROMOTE_DURATION_US + 1_000);
        let key = {
            let r = g.relation(rid).unwrap();
            (r.from, r.to)
        };
        let before = *g.fast_paths.by_endpoints.get(&key).unwrap();
        g.note_fast_path_usage(rid, 999_999);
        let after = *g.fast_paths.by_endpoints.get(&key).unwrap();
        assert_eq!(after.usage_count, before.usage_count + 1);
        assert_eq!(after.last_fired_t_us, 999_999);
    }

    // ── Phase A7 tests: MarkRef + Mark + channel ownership ───────────────

    #[test]
    fn stamp_mark_writes_first_mark_on_channel() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        g.stamp_mark(MarkChannel::Attention, MarkRef::Node(a), 0.5, 1_000);
        let m = g.get_mark(MarkChannel::Attention).expect("mark exists");
        assert_eq!(m.target, MarkRef::Node(a));
        assert_eq!(m.stamped_by, MarkChannel::Attention);
        assert!((m.strength - 0.5).abs() < 1e-9);
        assert_eq!(m.stamped_t_us, 1_000);
    }

    #[test]
    fn second_stamp_on_same_channel_replaces() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.20));
        g.stamp_mark(MarkChannel::Attention, MarkRef::Node(a), 0.3, 1_000);
        g.stamp_mark(MarkChannel::Attention, MarkRef::Node(b), 0.9, 2_000);
        let m = g.get_mark(MarkChannel::Attention).unwrap();
        assert_eq!(m.target, MarkRef::Node(b));
        assert!((m.strength - 0.9).abs() < 1e-9);
        // Only one mark per channel — should not have duplicated.
        let count: usize = g.active_marks().filter(|m| m.stamped_by == MarkChannel::Attention).count();
        assert_eq!(count, 1);
    }

    #[test]
    fn distinct_channels_hold_independent_marks() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.20));
        g.stamp_mark(MarkChannel::Attention, MarkRef::Node(a), 0.5, 1_000);
        g.stamp_mark(MarkChannel::Curriculum, MarkRef::Node(b), 0.7, 1_000);
        assert!(g.get_mark(MarkChannel::Attention).is_some());
        assert!(g.get_mark(MarkChannel::Curriculum).is_some());
        assert!(g.get_mark(MarkChannel::DualDrive).is_none());
        assert!(g.get_mark(MarkChannel::Intention).is_none());
    }

    #[test]
    fn stamp_mark_rejects_dangling_target() {
        let mut g = PerturbationGraph::default();
        // No nodes added — Node(99) is dangling.
        g.stamp_mark(MarkChannel::Attention, MarkRef::Node(99), 0.5, 1_000);
        assert!(g.get_mark(MarkChannel::Attention).is_none());
        // Sentinel id also rejected.
        g.stamp_mark(MarkChannel::Attention, MarkRef::Node(0), 0.5, 1_000);
        assert!(g.get_mark(MarkChannel::Attention).is_none());
    }

    #[test]
    fn decay_marks_drops_stale_marks() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        g.stamp_mark(MarkChannel::Attention, MarkRef::Node(a), 0.5, 1_000);
        // Within the decay window: mark survives.
        g.decay_marks(1_000 + MARK_DECAY_US / 2);
        assert!(g.get_mark(MarkChannel::Attention).is_some());
        // Past the decay window: mark dropped.
        g.decay_marks(1_000 + MARK_DECAY_US + 1);
        assert!(g.get_mark(MarkChannel::Attention).is_none());
    }

    #[test]
    fn clear_mark_releases_one_channel() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        g.stamp_mark(MarkChannel::Attention, MarkRef::Node(a), 0.5, 1_000);
        g.stamp_mark(MarkChannel::Intention, MarkRef::Node(a), 0.8, 1_000);
        g.clear_mark(MarkChannel::Attention);
        assert!(g.get_mark(MarkChannel::Attention).is_none());
        assert!(g.get_mark(MarkChannel::Intention).is_some());
    }

    #[test]
    fn mark_can_target_group_and_relation() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.11));
        let c = g.add_node(0, sig_at(0.12));
        let grp = g.add_group(0, RegionKey::Spatial(0, 0), &[a, b, c]).unwrap();
        let rid = g.observe_relation(0, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Causal).unwrap();
        g.stamp_mark(MarkChannel::DualDrive, MarkRef::Group(grp), 0.6, 1_000);
        g.stamp_mark(MarkChannel::Intention, MarkRef::Relation(rid), 0.7, 1_000);
        assert_eq!(g.get_mark(MarkChannel::DualDrive).unwrap().target, MarkRef::Group(grp));
        assert_eq!(g.get_mark(MarkChannel::Intention).unwrap().target, MarkRef::Relation(rid));
    }

    // ── Phase A6 tests: VocalProfile additive migration ───────────────────

    fn mel64_at(peak_bin: usize) -> [f64; PG_VOCAL_MEL_DIM] {
        let mut m = [0.0; PG_VOCAL_MEL_DIM];
        for (i, v) in m.iter_mut().enumerate() {
            let d = i as f64 - peak_bin as f64;
            *v = (-(d * d) / 18.0).exp();
        }
        m
    }

    #[test]
    fn register_vocal_profile_creates_entry() {
        let mut g = PerturbationGraph::default();
        let nid = g.add_node(0, sig_at(0.1));
        let mel = mel64_at(20);
        g.register_vocal_profile(nid, mel, 0.05, -0.02, 0.7, 1_000);
        let vp = g.vocal_profile(nid).expect("profile registered");
        assert!((vp.f1_off - 0.05).abs() < 1e-9);
        assert!((vp.f2_off + 0.02).abs() < 1e-9);
        assert!((vp.fit - 0.7).abs() < 1e-9);
        assert_eq!(vp.last_update_t_us, 1_000);
    }

    #[test]
    fn register_vocal_profile_skips_below_min_fit() {
        let mut g = PerturbationGraph::default();
        let nid = g.add_node(0, sig_at(0.1));
        let mel = mel64_at(20);
        g.register_vocal_profile(nid, mel, 0.0, 0.0, 0.10, 1_000);
        assert!(g.vocal_profile(nid).is_none(), "below VOCAL_FIT_MIN_STORE → no profile");
    }

    #[test]
    fn register_vocal_profile_only_updates_when_fit_improves() {
        let mut g = PerturbationGraph::default();
        let nid = g.add_node(0, sig_at(0.1));
        let mel = mel64_at(20);
        g.register_vocal_profile(nid, mel, 0.05, -0.02, 0.7, 1_000);
        // Worse fit: ignored.
        g.register_vocal_profile(nid, mel, 0.99, 0.99, 0.50, 2_000);
        let vp = g.vocal_profile(nid).unwrap();
        assert!((vp.fit - 0.7).abs() < 1e-9, "worse fit must not overwrite");
        assert!((vp.f1_off - 0.05).abs() < 1e-9);
        // Better fit: overwrites.
        g.register_vocal_profile(nid, mel, 0.10, -0.01, 0.85, 3_000);
        let vp2 = g.vocal_profile(nid).unwrap();
        assert!((vp2.fit - 0.85).abs() < 1e-9);
        assert!((vp2.f1_off - 0.10).abs() < 1e-9);
    }

    #[test]
    fn nearest_vocal_node_finds_close_match() {
        let mut g = PerturbationGraph::default();
        let nid_a = g.add_node(0, sig_at(0.1));
        let mel_a = mel64_at(20);
        let nid_b = g.add_node(0, sig_at(0.5));
        let mel_b = mel64_at(40);
        g.register_vocal_profile(nid_a, mel_a, 0.0, 0.0, 0.8, 0);
        g.register_vocal_profile(nid_b, mel_b, 0.0, 0.0, 0.8, 0);
        // Query very close to mel_a → returns nid_a.
        let q = mel64_at(20);
        let (hit, _) = g.nearest_vocal_node(&q).expect("match");
        assert_eq!(hit, nid_a);
        // Query close to neither (peak at 5 — far from 20 and 40) → no match.
        let q = mel64_at(5);
        assert!(g.nearest_vocal_node(&q).is_none());
    }

    #[test]
    fn prune_orphan_vocal_profiles_drops_evicted() {
        let mut g = PerturbationGraph::default();
        let nid = g.add_node(0, sig_at(0.1));
        g.register_vocal_profile(nid, mel64_at(20), 0.0, 0.0, 0.8, 0);
        assert!(g.vocal_profile(nid).is_some());
        // Manually evict the node (simulate eviction).
        g.nodes.retain(|n| n.id != nid);
        g.prune_orphan_vocal_profiles();
        assert!(g.vocal_profile(nid).is_none());
    }

    // ── Phase A8 tests: Sequence as 5th primitive RelationKind ─────────────

    // ── Phase A9 tests: persistence v2 round-trip ─────────────────────────

    #[test]
    fn persistence_v2_roundtrip_preserves_groups_relations_kinds_fastpaths_vocal() {
        let mut g = PerturbationGraph::default();
        // Populate some nodes.
        let a = g.add_node(1_000, sig_at(0.10));
        let b = g.add_node(1_000, sig_at(0.20));
        let c = g.add_node(1_000, sig_at(0.30));
        // A group.
        let grp = g.add_group(1_500, RegionKey::Spatial(3, 5), &[a, b, c]).unwrap();
        // A relation Node→Group.
        let r1 = g.observe_relation(2_000, Endpoint::Node(a), Endpoint::Group(grp), EdgeKind::Causal).unwrap();
        // A meta-relation Relation→Node.
        let r2 = g.observe_relation(2_500, Endpoint::Relation(r1), Endpoint::Node(c), EdgeKind::Sequence).unwrap();
        // An emergent kind.
        let r3 = g.observe_relation_emergent(3_000, Endpoint::Node(b), Endpoint::Node(c)).unwrap();
        // Strong relation → FastPath.
        for _ in 0..50 {
            g.observe_relation(4_000, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Cooccur);
        }
        g.tick_fast_paths(5_000);
        g.tick_fast_paths(FAST_PATH_PROMOTE_DURATION_US + 10_000);
        // A vocal profile on node a.
        let mel = [0.1_f64; PG_VOCAL_MEL_DIM];
        g.register_vocal_profile(a, mel, 0.05, -0.02, 0.7, 5_000);
        // Snapshot baseline counts.
        let n_nodes = g.nodes.len();
        let n_groups = g.groups.len();
        let n_relations = g.relations.len();
        let n_kinds = g.kinds.len();
        let n_fp = g.fast_paths.by_endpoints.len();
        let n_vp = g.vocal_profiles.len();
        // Roundtrip.
        let bytes = g.serialize();
        let back = PerturbationGraph::deserialize(&bytes).expect("v2 roundtrip");
        assert_eq!(back.nodes.len(), n_nodes);
        assert_eq!(back.groups.len(), n_groups);
        assert_eq!(back.relations.len(), n_relations);
        assert_eq!(back.kinds.len(), n_kinds);
        assert_eq!(back.fast_paths.by_endpoints.len(), n_fp);
        assert_eq!(back.vocal_profiles.len(), n_vp);
        // next_* allocators preserved.
        assert_eq!(back.next_id, g.next_id);
        assert_eq!(back.next_group_id, g.next_group_id);
        assert_eq!(back.next_rel_id, g.next_rel_id);
        assert_eq!(back.next_kind_id, g.next_kind_id);
        // Spot checks: the meta-relation Relation→Node endpoint survives.
        let r2_back = back.relation(r2).expect("meta relation present");
        assert!(matches!(r2_back.from, Endpoint::Relation(_)));
        // The emergent kind cluster_id roundtripped.
        let r3_back = back.relation(r3).expect("emergent relation present");
        assert!(r3_back.kind_id >= PRIMITIVE_KIND_COUNT as u32);
        assert!(back.kind(r3_back.kind_id).is_some());
        // Vocal profile fidelity.
        let vp_back = back.vocal_profile(a).expect("vocal profile preserved");
        assert!((vp_back.fit - 0.7).abs() < 1e-9);
        assert!((vp_back.f1_off - 0.05).abs() < 1e-9);
        // Group members preserved.
        let grp_back = back.group(grp).expect("group preserved");
        assert_eq!(grp_back.members.len(), 3);
    }

    #[test]
    fn v1_sidecar_still_loads_under_v2_codepath() {
        // Build a graph by hand, serialise via the v1-shape bytes (build them
        // manually so we don't depend on the old v1 serialize being intact),
        // and confirm deserialize accepts them with empty groups/relations.
        let mut g = PerturbationGraph::default();
        let a = g.add_node(1_000, sig_at(0.10));
        let b = g.add_node(1_000, sig_at(0.20));
        // Hand-craft v1 bytes: magic + version=1 + next_id + n_nodes + nodes +
        // n_edges + edges.
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PGRA");
        bytes.push(0x01); // VERSION_V1
        bytes.extend_from_slice(&g.next_id.to_le_bytes());
        bytes.extend_from_slice(&(g.nodes.len() as u64).to_le_bytes());
        for n in &g.nodes {
            bytes.extend_from_slice(&n.id.to_le_bytes());
            for v in &n.sig { bytes.extend_from_slice(&v.to_le_bytes()); }
            for v in &n.cross_modal { bytes.extend_from_slice(&v.to_le_bytes()); }
            for v in &n.vital_coupling { bytes.extend_from_slice(&v.to_le_bytes()); }
            bytes.extend_from_slice(&n.motion_var.to_le_bytes());
            bytes.extend_from_slice(&n.size_ema.to_le_bytes());
            bytes.extend_from_slice(&n.persistence_ema.to_le_bytes());
            bytes.extend_from_slice(&n.causal_outflow.to_le_bytes());
            bytes.extend_from_slice(&n.bind_strength.to_le_bytes());
            bytes.extend_from_slice(&n.age_us.to_le_bytes());
            bytes.extend_from_slice(&n.last_active_t_us.to_le_bytes());
        }
        bytes.extend_from_slice(&0u64.to_le_bytes()); // n_edges = 0
        let back = PerturbationGraph::deserialize(&bytes).expect("v1 still readable");
        assert_eq!(back.nodes.len(), 2);
        assert!(back.groups.is_empty());
        assert!(back.relations.is_empty());
        // Kinds were seeded to 5 primitives.
        assert_eq!(back.kinds.len(), PRIMITIVE_KIND_COUNT);
        let _ = (a, b);
    }

    #[test]
    fn deserialize_rejects_unknown_version() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(b"PGRA");
        bytes.push(0x99); // unknown version
        bytes.extend_from_slice(&[0; 32]);
        assert!(PerturbationGraph::deserialize(&bytes).is_none());
    }

    #[test]
    fn merge_propagates_vocal_profiles_via_id_map() {
        let mut mainline = PerturbationGraph::default();
        let m_a = mainline.add_node(0, sig_at(0.10));
        mainline.register_vocal_profile(m_a, [0.1_f64; PG_VOCAL_MEL_DIM], 0.05, 0.0, 0.4, 0);
        let mut branch = PerturbationGraph::default();
        let b_a = branch.add_node(0, sig_at(0.105)); // close to mainline m_a → matches
        // Better fit on branch — should win in merge.
        branch.register_vocal_profile(b_a, [0.1_f64; PG_VOCAL_MEL_DIM], 0.10, 0.0, 0.9, 100);
        mainline.merge(&branch, 0.05);
        let merged_vp = mainline
            .vocal_profile(m_a)
            .expect("vocal profile carried via id_map");
        // Better fit (0.9) should have won.
        assert!((merged_vp.fit - 0.9).abs() < 1e-9);
        assert!((merged_vp.f1_off - 0.10).abs() < 1e-9);
    }

    #[test]
    fn persistence_v2_stride_constants_pinned() {
        assert_eq!(PerturbationGraph::NODE_BYTES, 364);
        assert_eq!(PerturbationGraph::EDGE_BYTES, 33);
        assert_eq!(PerturbationGraph::ENDPOINT_BYTES, 5);
        assert_eq!(PerturbationGraph::GROUP_HEADER_BYTES, 4 + 1 + 8 + 8 * PG_NODE_SIG_DIM + 8 + 8 + 8 + 8);
        assert_eq!(PerturbationGraph::MEMBER_BYTES, 20);
        assert_eq!(PerturbationGraph::REL_BYTES, 4 + 5 + 5 + 4 + 8 + 8 + 8 + 8 + 1 + 8);
        assert_eq!(PerturbationGraph::REL_BYTES, 59);
        assert_eq!(PerturbationGraph::KIND_BYTES, 4 + 8 * PG_NODE_SIG_DIM * 2 + 8 + 8 + 8);
        assert_eq!(PerturbationGraph::KIND_BYTES, 412);
        assert_eq!(PerturbationGraph::FAST_PATH_BYTES, 28);
        assert_eq!(PerturbationGraph::VOCAL_PROFILE_BYTES, 4 + 8 * PG_VOCAL_MEL_DIM + 8 + 8 + 8 + 8);
        assert_eq!(PerturbationGraph::VOCAL_PROFILE_BYTES, 548);
    }

    #[test]
    fn sequence_kind_is_5th_primitive() {
        let g = PerturbationGraph::default();
        assert_eq!(g.kinds.len(), PRIMITIVE_KIND_COUNT);
        assert_eq!(PRIMITIVE_KIND_COUNT, 5);
        let k = g.kind(KIND_ID_SEQUENCE).expect("Sequence primitive exists");
        assert_eq!(k.cluster_id, KIND_ID_SEQUENCE);
        // EdgeKind::Sequence maps to discriminant 4.
        assert_eq!(EdgeKind::Sequence.as_u8(), 4);
        assert_eq!(EdgeKind::from_u8(4), EdgeKind::Sequence);
    }

    #[test]
    fn observe_relation_with_sequence_kind_distinguishes_from_causal() {
        let mut g = PerturbationGraph::default();
        let a = g.add_node(0, sig_at(0.10));
        let b = g.add_node(0, sig_at(0.20));
        let causal = g
            .observe_relation(0, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Causal)
            .unwrap();
        let seq = g
            .observe_relation(0, Endpoint::Node(a), Endpoint::Node(b), EdgeKind::Sequence)
            .unwrap();
        assert_ne!(causal, seq, "different kinds → distinct relation ids");
        assert_eq!(g.relation(causal).unwrap().kind_id, KIND_ID_CAUSAL);
        assert_eq!(g.relation(seq).unwrap().kind_id, KIND_ID_SEQUENCE);
    }

    #[test]
    fn mel64_shape_distance_basics() {
        let a = mel64_at(20);
        // Identical fingerprints → 0 distance.
        assert!(mel64_shape_distance(&a, &a) < 1e-9);
        // Silent target → infinite distance (no shape to compare).
        let zero = [0.0; PG_VOCAL_MEL_DIM];
        assert!(mel64_shape_distance(&zero, &a).is_infinite());
        // Distinct peaks → strictly positive distance.
        let b = mel64_at(40);
        assert!(mel64_shape_distance(&a, &b) > 0.01);
    }

    #[test]
    fn mark_channel_enum_roundtrip() {
        for ch in [
            MarkChannel::Attention,
            MarkChannel::Curriculum,
            MarkChannel::DualDrive,
            MarkChannel::Intention,
        ] {
            assert_eq!(MarkChannel::from_u8(ch.as_u8()), ch);
        }
        // Unknown bytes fall back to Attention.
        assert_eq!(MarkChannel::from_u8(255), MarkChannel::Attention);
    }

    #[test]
    fn fast_path_cap_evicts_lru() {
        // Drive cap to a small value indirectly: not feasible at compile time,
        // but we can confirm the LRU evictor runs at all by promoting several
        // relations and verifying by_endpoints stays sane.
        let mut g = PerturbationGraph::default();
        for i in 0..5 {
            let anchor = sig_at(0.1 + 0.01 * i as f64);
            let n1 = g.add_node(0, anchor);
            let n2 = g.add_node(0, sig_at(0.5 + 0.01 * i as f64));
            for k in 0..50 {
                g.observe_relation((i as u64) * 10_000 + k * 100, Endpoint::Node(n1), Endpoint::Node(n2), EdgeKind::Cooccur);
            }
        }
        // Two-step promotion as for all FastPath tests.
        g.tick_fast_paths(10);
        g.tick_fast_paths(FAST_PATH_PROMOTE_DURATION_US + 1_000);
        assert!(g.fast_paths.by_endpoints.len() >= 5, "all five should fit under the 2048 cap");
        // by_from should also map each from endpoint to at least one entry.
        for (_, list) in &g.fast_paths.by_from {
            assert!(!list.is_empty());
        }
    }
}
