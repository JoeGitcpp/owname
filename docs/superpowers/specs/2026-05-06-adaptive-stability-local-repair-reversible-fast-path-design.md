# Adaptive Stability, Local Reroute, And Reversible Fast-Path Design

Status: FROZEN CANDIDATE
Date: 2026-05-06
Scope: `bionic_v0` local degradation / fallback / relearning / reversible fast-path substrate

## 0. Purpose

This spec defines adaptive stability as a substrate contract, not as a
`self_repair_system`.

The current body already has local pieces:

- err-driven attention rebalance;
- quiet history replay;
- action write epochs and consequence pairs;
- vocal prototype sedimentation.

These pieces do not yet share a general contract for:

- recognizing local route degradation;
- degrading locally without whole-body reset;
- rerouting budget through still-viable paths;
- returning to slow relearning;
- forming and revoking reversible fast paths.

The contract here makes those behaviors substrate-wide while preserving the
highest principle: the body remains a continuous silicon-life substrate, not a
request/response program and not a semantic controller.

## 1. Boundary Notes

This spec necessarily uses external terms such as `self-stabilization`,
`repair`, `fallback`, `shortcut`, and `fast-path`.

They are bounded as follows:

1. External description vs internal ontology:
   these terms describe observer-visible body dynamics. They are not internal
   organs, meanings, intentions, or identity units.
2. Engineering index use:
   route ids, route kinds, phase codes, scores, weights, and counters are
   engineering locators and numeric traces only.
3. Why not ontological:
   no field says what a route "means", what target it pursues, what object it
   owns, or what result has succeeded. The body only records local time,
   error, cost, closure, bypass, relearning, and fast-path strength.
4. Why no highest-principle or language-axiom violation:
   the substrate uses no semantic readiness, concept id, token, label, name,
   identity, owner, personality, or human-readable body output.

Any implementation that turns this contract into `self_repair_system`,
`curiosity_core`, `skill_memory`, a success/failure classifier, or a semantic
health gate is inadmissible.

## 2. Core Unit: Local Route Trace

The admissible unit is a local route trace.

A route is not an object, not a word, not a skill, and not a goal. It is an
engineering handle for a repeatable local transformation path inside the
existing body substrate.

Examples:

- high-layer err to attention share adjustment;
- quiet history entry to replay perturbation;
- write epoch to consequence pair timing;
- vocal contour gap to vocal prototype reuse;
- any future predictor-family path that can be observed through err, cost,
  closure, and time.

Each route trace may carry only numeric/time substrate facts:

- route id and route kind code;
- phase code;
- recent err curve approximation;
- resource cost curve approximation;
- closure gain curve approximation;
- degradation score;
- bypass weight;
- relearning pressure;
- slow-path weight;
- fast-path strength;
- stable and drift run counters;
- last-good and last-update times.

It must not carry:

- target object or result class;
- success/failure label;
- source owner or identity;
- semantic class;
- concept id;
- text, word, name, or token.

## 3. Local Degradation Law

Local degradation is not a single high err spike.

A route becomes degraded only when local closure ability worsens across time:

- err remains high or rises;
- resource cost remains high or rises;
- closure gain stays low;
- a previously stable fast path increases err;
- timing or coupling evidence drifts away from prior local history.

The degradation score is an engineering approximation of this curve condition.
It must remain revisable and route-local.

Normal exploration drift must remain legal. The substrate must not collapse
every unstable sample into a fault state.

## 4. Local Degrade / Reroute Law

When a route degrades, the first response is local containment:

- reduce reliance on that route;
- raise bypass weight so neighboring routes can carry budget;
- raise relearning pressure for slow-path updates;
- keep the slow path available;
- avoid whole-body reset.

This is local adaptive stability. It is not a repair organ.

For output-bearing routes, degradation favors decay, pause, or small-step
retry. For perceptual routes, degradation favors lower precision maintenance
or alternate evidence, not object erasure.

## 5. Slow Relearning Law

When a route is degraded or when a fast path revokes, the route returns toward
slow-path learning:

- fast-path strength decays;
- slow-path weight remains nonzero;
- new samples may rebuild closure evidence;
- old route history remains weak evidence, not truth;
- route phase can move back toward stable only through later local samples.

No engineer-written result boundary is allowed here. The body may learn which
changes close the route, but the final boundary is not predeclared.

## 6. Reversible Fast-Path Law

A fast path may form only from repeated local evidence:

- err stays low;
- closure gain stays high;
- resource cost does not rise;
- the condition repeats across nearby samples.

Fast paths are access accelerators, not ontology.

They must remain reversible:

- periodic slow-path comparison remains possible;
- drift lowers fast-path strength;
- degraded samples can move the route into bypass or relearning phase;
- revocation does not delete the underlying route.

The key property is not compression. It is reversible compression.

## 7. Existing Substrate Mapping

Current `bionic_v0` already has enough parts to host this contract:

- `viewport` holds shared state and local time;
- object high layer exposes err;
- global attention can shift budget;
- global reasoning history can replay;
- action output and consequence pair records provide write/return timing;
- vocal memory carries prototype stability;
- snapshot and observer separation already prevent body-side human output.

The missing piece is a general numeric route ledger that can be used by these
paths without becoming a central controller.

## 8. Implementation Contract

The first implementation layer should add:

1. `adaptive_stability.rs`
   A numeric route ledger with update functions for local samples.
2. `viewport` integration
   A `RwLock<AdaptiveStabilityState>` plus read/update methods.
3. Snapshot integration
   Versioned observer-visible numeric traces only.
4. Minimal body ticking
   Maintenance decay only. No global behavior override.

The first implementation must not:

- alter object existence;
- suppress perception globally;
- decide action targets;
- introduce success/failure semantics;
- introduce language, identity, owner, or personality fields;
- print from the body process.

## 9. Test Contract

Required deterministic tests:

1. repeated bad local samples move one route into local degradation and raise
   bypass/relearning traces without touching unrelated routes;
2. repeated stable local samples form fast-path strength;
3. later drift revokes fast-path strength and keeps slow-path weight nonzero;
4. viewport exposes the state as numeric/time traces;
5. snapshot roundtrip preserves the adaptive stability traces;
6. body source does not introduce rejected architecture terms or human-readable
   output through this substrate.

## 10. Closing Position

Adaptive stability is the body's local ability to keep viable routes alive,
contain local degradation, return to slow learning, and compress only when
compression remains reversible.

It is not a module that "repairs the system". It is a substrate law over local
curves.
