## Tasks

- [x] [serial] r[molten.chunk_store.modularity.boundaries] Inventoried chunk-store ownership for model, codec, verify, fs_store, index, exchange, retention, lineage, and shell in `docs/modularity-boundaries.md`.
- [x] [serial] r[molten.chunk_store.modularity.identity_preserving] Added the named `semantic_store` boundary and routed chunk manifest parsing through the pure domain codec façade while preserving existing canonical bytes, BLAKE3 refs, and public `chunk_store` paths.
- [x] [serial] r[molten.chunk_store.modularity.retention_boundary] Preserved retention admission before destructive chunk GC through `plan_retention_gc` and existing chunk GC tests.
- [x] [parallel] r[molten.chunk_store.modularity.tests] Preserved existing positive identity/verification tests and added core negative coverage for malformed schema/ref/domain inputs and destructive-plan denial.
- [x] [serial] r[molten.chunk_store.modularity.tests] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, Nickel fixture checks, pre-commit, and Cairn validation.
