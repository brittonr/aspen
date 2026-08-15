## Tasks

- [x] [serial] r[molten.project.modularity.dependency_classes] Inventoried Cargo dependencies and classified each as core, codec, policy-evidence, runtime, adapter, CLI, test, or integration in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.project.modularity.minimal_core_build] Defined and implemented `crates/molten-core` as a minimal core/codec-adjacent build surface excluding Iroh, Redb, Wasmtime, Steel execution, Nickel tooling, and stack integration crates.
- [x] [serial] r[molten.project.modularity.default_compatibility] Preserved default developer and CLI build behavior by keeping the root crate and adding compatibility re-exports.
- [x] [parallel] r[molten.project.modularity.dependency_tests] Added positive minimal-build tests and negative dependency-boundary diagnostics for adapter leakage.
- [x] [serial] r[molten.project.modularity.dependency_tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, Nickel fixture checks, pre-commit, and Cairn validation.
