# Quarantine

This directory holds artifacts that defended a **rejected architecture**.
They are kept on disk for archaeological reference only.

Per cleanup spec
[`docs/superpowers/specs/2026-04-20-bionic-v0-architecture-audit-cleanup-design.md`](../../docs/superpowers/specs/2026-04-20-bionic-v0-architecture-audit-cleanup-design.md)
and language-system axioms
[`docs/axioms/2026-04-20-new-axioms-v1.md`](../../docs/axioms/2026-04-20-new-axioms-v1.md):

- `semantic_readiness.rs.quarantine` — Phase 5H readiness gate. Rejected:
  it made naming/labeling a resource gate over substrate.
- `symbolizer.rs.quarantine` — Phase 6A concept-id allocator. Rejected:
  it stamped objects/edges with discrete internal "concepts," violating
  axioms 1, 3, 5, 8.

**Do not** restore these files into the active `tests/` directory or wire
them into Cargo. They reference removed APIs and represent forbidden
architecture. New language-related work must start from the axioms doc, not
from these contracts.
