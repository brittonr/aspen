## Tasks

- [x] [serial] r[molten.retention.modularity.boundaries] Inventoried retention ownership for admission, plan, apply, audit, store, bundle, live, and receipts in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.retention.modularity.destructive_plan] Extracted `plan_retention_gc` as a pure destructive admission/GC planning boundary returning explicit effects before side effects.
- [x] [serial] r[molten.retention.modularity.store_shell] Kept selected retention filesystem/store operations behind shell/store-port ownership and documented the split.
- [x] [parallel] r[molten.retention.modularity.tests] Added positive retention-plan tests and negative tests for missing authority, stale plan, incomplete index, and missing remote clearance.
- [x] [serial] r[molten.retention.modularity.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
