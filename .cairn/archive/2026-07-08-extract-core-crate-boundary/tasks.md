## Tasks

- [x] [serial] r[molten.modularity.core_crate.pure_foundation] Added the workspace `crates/molten-core` pure crate with no adapter, CLI, filesystem, process, network, clock, environment, Redb, Iroh, Wasmtime, Steel, or live Nickel dependencies.
- [x] [serial] r[molten.modularity.core_crate.pure_foundation] Moved low-risk foundational planning, policy freshness, codec identity, stack envelope, Preserves profile, and dependency-boundary cores into `molten-core` with root compatibility re-exports.
- [x] [serial] r[molten.modularity.core_crate.dependency_direction] Added a pure dependency-boundary validator and reviewed Nickel boundary policy to fail adapter/CLI leakage into core.
- [x] [parallel] r[molten.modularity.core_crate.validation] Added positive and negative core tests for valid inputs and malformed refs, missing fields, invalid bounds, unsupported states, and denied plans.
- [x] [serial] r[molten.modularity.core_crate.validation] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
