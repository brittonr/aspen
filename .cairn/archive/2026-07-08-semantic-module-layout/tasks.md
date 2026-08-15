## Tasks

- [x] [serial] r[molten.modularity.semantic_modules.named_boundaries] Inventoried ordinal `include!` shard entry points and selected chunk store as the highest-pressure first semantic split.
- [x] [serial] r[molten.modularity.semantic_modules.named_boundaries] Refactored `src/chunk/store.rs` into the named `semantic_store` submodule while preserving existing exported paths.
- [x] [serial] r[molten.modularity.semantic_modules.functional_core] Moved boundary decisions behind in-memory functional-core APIs in `molten-core` while keeping IO in shells.
- [x] [parallel] r[molten.modularity.semantic_modules.exemptions] Recorded explicit staged-compatibility exemptions for remaining ordinal shards in `docs/modularity-boundaries.md`.
- [x] [parallel] r[molten.modularity.semantic_modules.functional_core] Added/preserved positive and negative tests for core planners, codec façade, Preserves profile, dependency gates, and stack envelopes.
- [x] [serial] r[molten.modularity.semantic_modules.named_boundaries] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, Nickel fixture checks, pre-commit, and Cairn validation.
