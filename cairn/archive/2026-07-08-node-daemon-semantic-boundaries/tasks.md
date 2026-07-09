## Tasks

- [x] [serial] r[molten.node_runtime.modularity.daemon_modules] Inventoried node daemon responsibilities and selected duplicate enqueue planning as the first extraction slice in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.node_runtime.modularity.pure_daemon_core] Extracted `plan_node_enqueue` as a pure node daemon decision boundary for duplicate enqueue/replay behavior.
- [x] [serial] r[molten.node_runtime.modularity.shell_boundary] Kept state-root IO, live transport, and service-loop orchestration documented as shell responsibilities.
- [x] [parallel] r[molten.node_runtime.modularity.tests] Added positive and negative core tests for admitted enqueue, duplicate receipt-only replay, and denied inputs.
- [x] [serial] r[molten.node_runtime.modularity.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
