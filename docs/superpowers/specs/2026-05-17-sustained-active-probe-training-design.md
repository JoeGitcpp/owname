# Sustained Active Probe Training Design

## Goal

Enter the next training stage after foundation readiness by proving that active
probe closure can repeat across several disturbance/correction cycles in one
persistent body-side substrate.

## Boundary

This is a curriculum/observer-side training harness. It does not add a
subconscious/conscious module, a request/response surface, a semantic readiness
gate, a concept allocator, or any human-readable output inside `body`.

The high-risk terms in this document are external descriptions only:

- `active probe`: external description for existing framed-byte write-out plus
  ingress-return pairing.
- `text`: external description for byte-mark disturbance streams already carried
  by `mark_disturbance`; not an internal token or word ontology.
- `correction`: external description for reversible binding feedback events; not
  a truth label.

## Design

Add a new curriculum function:

`run_sustained_active_probe_training_curriculum(config)`

It runs one persistent viewport through:

1. shared mark/target training,
2. several conflict cycles,
3. two framed-byte probe write-outs and returns per cycle,
4. local binding correction after each return,
5. final reentry compared with a cold viewport.

The evidence is numeric and observer-side:

- number of cycles,
- conflict events,
- probe frames,
- return windows,
- paired consequences,
- average pair similarity,
- minimum ambiguity reduction ratio across cycles,
- minimum correction recovery ratio across cycles,
- maximum wrong binding strength across cycles,
- persistent reentry strength,
- cold reentry strength.

The pass condition is not just that one probe closes. It requires every cycle to
produce return pairing and correction evidence, while final persistent reentry
remains stronger than the cold baseline.

## Testing

Use TDD:

1. Add a failing test for sustained active probe evidence.
2. Add a failing report test for `--sustained-active-probe-training`.
3. Implement the minimal curriculum and report branch.
4. Verify focused tests, report output, formatting, diff hygiene, and full
   `bionic_v0` tests.

