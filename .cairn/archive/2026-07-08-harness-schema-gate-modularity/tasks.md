## Tasks

- [x] [serial] r[molten.testing.modularity.harness_layers] Inventoried harness schema/gate ownership in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.testing.modularity.pure_gate_decisions] Extracted `plan_harness_gate` as a pure in-memory suite/report gate decision.
- [x] [serial] r[molten.testing.modularity.runtime_boundary] Documented that harness orchestration and fixture IO remain shell-owned while runtime cores avoid harness orchestration dependencies.
- [x] [parallel] r[molten.testing.modularity.fixtures] Added positive/negative core tests for supported reports and missing, malformed, stale, or unsupported harness inputs.
- [x] [serial] r[molten.testing.modularity.fixtures] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
