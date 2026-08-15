## Tasks

- [x] [serial] r[molten.modularity.codec_facades.domain_owned] Selected the chunk store as the high-fan-in Preserves domain and defined `molten_core::codec::validate_domain_artifact` as the domain-owned identity façade.
- [x] [serial] r[molten.modularity.codec_facades.identity_preserving] Migrated chunk manifest parsing to call the façade after canonical ref computation while preserving existing canonical Preserves bytes, BLAKE3 refs, and parser decisions.
- [x] [parallel] r[molten.modularity.codec_facades.parser_symmetry] Added positive core tests for valid artifact identity and negative tests for unsupported labels, schema drift, malformed refs, and missing domain identity.
- [x] [serial] r[molten.modularity.codec_facades.dependency_direction] Documented that shared broad codec helpers must not import high-level runtime, node, retention, job, plugin, CLI, or adapter domains.
- [x] [serial] r[molten.modularity.codec_facades.identity_preserving] Ran `cargo test -p molten-core`, `cargo test --lib`, `cargo fmt --check`, pre-commit, and Cairn validation.
