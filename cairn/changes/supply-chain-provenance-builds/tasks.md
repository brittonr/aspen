## Phase 1: Provenance model

- [ ] [serial] r[molten.provenance.record_model] Define canonical provenance records with artifact id, source refs, dependency closure, toolchain refs, build params, builder identity, signatures, review refs, tests, and policy receipts.
- [ ] [serial] r[molten.provenance.trust_states] Define contextual trust states for unknown, source-known, builder-attested, reviewed, reproducible-verified, sandbox-only, policy-trusted, and denied artifacts.
- [ ] [parallel] r[molten.provenance.hash_not_trust] Document that content addressing proves identity but not trust.
- [ ] [parallel] r[molten.provenance.catalog_view] Expose provenance summaries through catalog/MCP with visibility filtering.

## Phase 2: Installation gates

- [ ] [serial] r[molten.provenance.install_policy] Gate artifact installation and execution by provenance policy per artifact kind and environment.
- [ ] [serial] r[molten.provenance.policy_artifacts] Require stronger provenance for policy predicates, migration recipes, and production executable artifacts.
- [ ] [parallel] r[molten.provenance.receipts] Emit Cairn receipts for provenance evaluation, approval, denial, and trust-state changes.
- [ ] [parallel] r[molten.provenance.remote_sync] Validate provenance requirements during remote artifact sync before execution.

## Phase 3: Reproducible builds

- [ ] [serial] r[molten.provenance.build_record] Define reproducible build records with source, dependency closure, toolchain, build params, and expected artifact id.
- [ ] [parallel] r[molten.provenance.nix_refs] Represent Nix derivation/toolchain refs where available for Rust/Wasm artifacts.
- [ ] [parallel] r[molten.provenance.verify_build] Add verification receipts for matching or mismatching reproducible builds.
- [ ] [parallel] r[molten.provenance.mismatch_diagnostics] Report expected/actual artifact ids and differing provenance inputs on mismatch.

## Phase 4: Tests

- [ ] [serial] r[molten.provenance.install_tests] Add tests that artifacts missing required provenance are denied in production policy.
- [ ] [serial] r[molten.provenance.sandbox_tests] Add tests that low-trust artifacts may run only under restricted sandbox/test profiles when policy admits.
- [ ] [parallel] r[molten.provenance.repro_tests] Add tests for reproducible build record verification and mismatch diagnostics.
- [ ] [parallel] r[molten.provenance.property_tests] Add Hegel property tests for provenance-context monotonicity and no-trust-from-hash invariants.
