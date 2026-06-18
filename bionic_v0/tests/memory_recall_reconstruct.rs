//! R1 (within-lifetime): recall re-descend can RECONSTRUCT a memory's detail
//! from the learned cross-layer RELATION RULE, not only fetch a live slot's raw
//! value.
//!
//! Phase 3 cutover: SlotCrossLayer / object_high_layer were deleted. The
//! reconstruction path depended on them (`tick_slot_cross_layer`,
//! `read_slot_cross_layer`, `refresh_slot_high_layer`). The relation-rule
//! re-descend semantics need to be re-derived against perturbation_graph
//! before this test can be brought back.
//!
//! TODO: rewrite against perturbation_graph

#![cfg(any())]
