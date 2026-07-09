## Tasks

- [x] [serial] r[molten.artifact_registry.modularity.layer_ownership] Inventoried evidence, ledger, registry, and catalog ownership in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.artifact_registry.modularity.evidence_without_storage] Added in-memory evidence/discovery planning through `plan_registry_discovery` without requiring ledger persistence.
- [x] [serial] r[molten.artifact_registry.modularity.discovery_non_authority] Preserved registry/catalog discovery as read-only evidence that cannot grant authority or trust by itself.
- [x] [parallel] r[molten.artifact_registry.modularity.tests] Added positive/negative `molten-core` tests for discovered evidence with explicit authority/provenance and registry-only denial.
- [x] [serial] r[molten.artifact_registry.modularity.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
