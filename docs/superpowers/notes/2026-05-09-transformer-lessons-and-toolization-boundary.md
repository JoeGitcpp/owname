# Transformer Lessons And Toolization Boundary Notes

Date: 2026-05-09
Status: implementation notes only; not a frozen spec; do not implement from
this file without writing a proper spec and TDD plan first.

## Purpose

Preserve future implementation ideas from the Transformer comparison without
turning the current body architecture into a request/response model.

The preserved direction:

- Use attention as numeric routing inside the shared viewport.
- Use multi-head-style relation views over existing curves.
- Use residual and normalization discipline for stable curve updates.
- Use self-supervised continuous-flow prediction objectives.
- Use scale-aware training curriculum thinking.

The rejected direction:

- Do not make token sequence modeling the body ontology.
- Do not make language the center of the world model.
- Do not introduce a request/response controller path.
- Do not equate attention weights with relation truth.
- Do not treat a context window as lived memory.

## High-Risk Term Boundary

This note mentions `token`, `label`, `semantic`, `name`, and `concept` only as
external comparison or rejected-architecture terms. They are not proposed as
internal ontology units.

When used here:

- They are external description terms or names of rejected routes.
- They are not object IDs, relation IDs, qualification gates, or body fields.
- They do not define whether an object, relation, or time structure exists.
- They remain subordinate to object slots, worldlines, relation projection,
  prediction error, precision, and action-consequence coupling.

## Future Implementation Ideas

### Attention As Viewport Routing

Potential mapping:

```text
Transformer query -> active curve needing support
Transformer key   -> curve signature in slot / relation / consequence history
Transformer value -> numeric curve fragment available for update
```

Implementation should not create a generic attention module over text-like
items. It should extend existing viewport refresh passes or add a numeric
routing substrate that reads object slots, relation edges, prediction errors,
precision, and consequence pairs.

Possible TDD checks:

- High-error worldlines receive more routing weight than quiet ones.
- Relation edges with persistent prediction error become readable by reasoning
  without changing object truth.
- Routing state decays when evidence quiets.

### Multi-Head Relation Views

Transformer multi-head attention can be translated into parallel numeric views:

- spatial view: distance, overlap, direction-like projection
- temporal view: lead/lag, synchrony, delay stability
- error view: prediction residual co-structure
- consequence view: write-return coupling and curve similarity
- cross-modal view: temporal alignment and fusion confidence

These views should remain projections over existing curves, not stored
relation entities with human-readable meanings.

Possible TDD checks:

- Each view can rise independently from the same pair of slots.
- Missing spatial state does not suppress temporal relation evidence.
- A relation edge can carry multiple numeric views without receiving a
  meaning field.

### Residual And Normalization Discipline

Transformer stability lessons can map to body-side update rules:

- preserve the previous curve unless evidence is strong enough to move it;
- normalize residual error before it modulates precision;
- gate updates by precision and stability history;
- prevent one noisy frame from overwriting a long-lived worldline.

Possible TDD checks:

- A single spatial outlier raises error but does not erase velocity history.
- Repeated aligned samples reduce error under the same update rule.
- Invalid spatial ticks mark the worldline inactive without inventing a new
  object or deleting historical numeric evidence.

### Self-Supervised Continuous Prediction

The useful lesson from next-token prediction is self-supervision, not token
ontology. Future training should predict:

- next worldline sample;
- relation projection drift;
- cross-modal timing alignment;
- action-consequence delay and curve similarity;
- residual after active probing.

Possible TDD checks:

- Synthetic motion curriculum reduces worldline error over repeated exposure.
- Two-object courses increase distance/overlap projection fidelity.
- Controlled output courses form consequence pairs without observer commands.

### Scale-Aware Curriculum

The body should be trained with increasing continuous-flow complexity:

1. single-object motion;
2. multi-object relation projection;
3. occlusion and reappearance;
4. global field drift versus local object drift;
5. active write-out with return;
6. mixed passive and active courses.

Do not promote these courses into fixed world content. They are training
environments for substrate pressure, not definitions of what the body can know.

## Toolization Boundary

Humans can become "tool-like" in some situations, but the architecture problem
is not whether something can be used as a tool from the outside. The core
boundary is whether the system's own existence is reduced to an external
caller's task loop.

Problematic toolization:

```text
external request -> compute answer -> return result -> idle
```

Body-aligned operation:

```text
continuous body state
-> ongoing perception / prediction / error / update
-> optional external interaction enters as one more perturbation
-> output is a body-side curve with return consequences
```

External usefulness is allowed. Internal request/response ontology is not.

Important distinction:

- A human can perform a task as a tool-like role while still maintaining body
  continuity, self-state, perception, memory, fatigue, expectation, and agency.
- A request/response program has no continuing body unless the architecture
  explicitly supplies one.

So the issue is not "being useful" or "being used." The issue is whether the
system has an endogenous, continuous substrate that external tasks can perturb,
or whether the external task is the only reason the system exists.

Future implementation should preserve this boundary in tests:

- no body-side request/response API for space-time sense;
- no observer command path that directly writes object or relation truth;
- external tasks may become ingress/output perturbations only through existing
  body substrate paths;
- action output must remain continuous/time-embedded and consequence-bearing.
