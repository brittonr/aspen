## Tasks

- [x] [serial] r[molten.operator_workflow.modularity.integration_boundary] Inventoried dogfood, prod-soak, and NixOS VM dependencies and assigned them to integration-shell ownership in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.operator_workflow.modularity.dependency_direction] Added/documented dependency-boundary policy so runtime/node cores do not import operator dogfood, prod, or VM modules.
- [x] [serial] r[molten.operator_workflow.modularity.evidence_only] Preserved evidence-only semantics for dogfood, soak, and VM receipts in the boundary inventory and non-claims.
- [x] [parallel] r[molten.operator_workflow.modularity.tests] Preserved existing dogfood/prod evidence tests and added pure boundary tests proving evidence/adapter availability alone is not trust.
- [x] [serial] r[molten.operator_workflow.modularity.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
