## Phase 1: Dogfood retention workflow

- [x] [serial] r[molten.operator_dogfood_retention_gc.workflow] Add deterministic retention fixture evidence for requester, object, policy, authority, supporting evidence, reference-index, remote-GC, and remote clearance.
- [x] [serial] r[molten.operator_dogfood_retention_gc.workflow] Run retention GC plan, apply, execute, audit, explain, bundle export/profile/verify, and catalog/MCP discovery inside `molten dogfood local-node`.
- [x] [parallel] r[molten.operator_dogfood_retention_gc.evidence_only] Keep dogfood retention receipts evidence-only and require normal retention gates before any destructive subsystem mutation.

## Phase 2: Release evidence and docs

- [x] [serial] r[molten.operator_dogfood_retention_gc.release_gate] Bind retention GC refs into operator checkpoints, dogfood reports, release evidence, and ledger/catalog imports.
- [x] [parallel] r[molten.operator_dogfood_retention_gc.release_gate] Document retention dogfood workflow usage and evidence-only boundaries.

## Phase 3: Verification

- [x] [serial] r[molten.operator_dogfood_retention_gc.tests] Add tests that local dogfood emits retention GC steps, bundle verify evidence, catalog discovery, and a pass report.
- [x] [serial] r[molten.operator_dogfood_retention_gc.tests] Validate Cairn gates, Rust checks, Octet, and Nix nextest before archiving.
