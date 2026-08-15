## Tasks

- [x] [serial] r[molten.project.policy_boundary.layered_policy] Documented policy authoring, generated export, runtime consumption, and freshness responsibilities in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.project.policy_boundary.runtime_no_live_tooling] Checked and documented that runtime admission consumes checked exports, refs, or receipts rather than invoking Nickel or Cairn policy tooling as live authority.
- [x] [serial] r[molten.project.policy_boundary.fresh_generated_policy] Refreshed `cairn-policy/default.ncl`, `contracts.ncl`, and generated JSON with current Cairn fields: `policy_schema_compatibility`, `traceability_policy`, `stack_provenance_gate`, and `runtime_evidence_policy`.
- [x] [parallel] r[molten.project.policy_boundary.tests] Added pure freshness tests for missing schema fields and stale generated refs; Nickel fixture checks validate fresh exports and negative contract behavior.
- [x] [serial] r[molten.project.policy_boundary.tests] Ran policy export drift check, Nickel fixture checks, `cargo test -p molten-core`, `cargo test --lib`, pre-commit, and Cairn validation without the canonical-policy workaround.
