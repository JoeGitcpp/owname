# Long-Horizon Pressure Chain v0 Design

## Boundary

This design extends the existing active surface chain memory with long-horizon
numeric evidence. It does not introduce `Thought`, `Goal`, `Plan`, `Question`,
`Answer`, `Desire`, `Mind`, `Subconscious`, or `Conscious` modules. Those words
remain external metaphors only.

The body may store only substrate-level numeric/time evidence:

- chain span
- chain step depth
- long-chain score
- interruption count
- recovery count
- retained branch pressure
- delayed reentry pressure

No body-side human-readable output is introduced. Observer reporting remains
outside the body.

## Problem

The current reasoning substrate proves a short active probe loop:

- pressure selects an active surface focus
- probe writes out
- return evidence revises local pressure
- chain memory sediments the loop
- soft arbitration can hold, switch, and reenter between competing chains

The missing piece is duration. A useful early reasoning substrate must show
that a chain can remain coherent across many recurrence steps, quiet windows,
interruptions, delayed returns, and later reentry.

This is not a planner and not a symbolic reasoning engine. It is a pressure
chain that can persist long enough for later active closure.

## Design

Extend `ChainMemoryTrace` with long-horizon pressure fields:

- `span_us`: elapsed time between first and latest sediment update
- `depth_score`: recurrence-depth pressure
- `long_chain_score`: combined pressure from sediment, continuity, span, and
  depth
- `interruption_count`: non-dominant survival under competing branch pressure
- `recovery_count`: return to dominance after interruption

The update law stays local:

- Every new sediment update refreshes span and depth for that trace.
- When a trace is not dominant but retains branch pressure, interruption count
  can rise.
- When a previously interrupted trace becomes dominant again, recovery count
  can rise.
- Long-chain score is bounded and revisable; it is not a fixed identity, label,
  concept, or task.

Soft arbitration may use the new long-chain score as a small contributor to
dominance. This lets long-lived chains resist deletion without making them
unstoppable.

## Observer Evidence

`curriculum_report --long-horizon-pressure-chain` must emit compact numeric
evidence:

- `long_chain_steps`
- `long_chain_span_us`
- `long_chain_score`
- `interruption_count`
- `recovery_count`
- `interruption_branch_pressure`
- `delayed_reentry_pressure`
- `single_aperture_preserved`
- `branch_trace_retained`
- `wrong_chain_binding`
- `body_source_order_guard`
- `principle_guard_pass`

## Acceptance

- `long_chain_steps >= 12`
- `long_chain_span_us > 1_000_000`
- `long_chain_score > 0.50`
- `interruption_count >= 1`
- `recovery_count >= 1`
- `interruption_branch_pressure > 0.0`
- `delayed_reentry_pressure > 0.0`
- `single_aperture_preserved = 1`
- `branch_trace_retained = 1`
- `wrong_chain_binding <= 0.01`
- `body_source_order_guard = 1`
- `principle_guard_pass = 1`

## Guardrails

- No labels, concepts, tokens, tasks, planner, or question-answer layer.
- Coordinates are substrate indices only.
- Long-chain metrics are pressure shadows of learned continuity, not ontology.
- Branches remain bounded traces and must not be deleted merely because they
  are not current.
