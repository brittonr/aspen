# Tasks: plugin-extension-domain-contract-hardening

- [x] [serial] r[molten.plugin_extension_contracts.domain_hardening.authoring_contracts] Add pure Nickel predicates for plugin extension refs, ids, versions, profiles, replay classes, non-empty evidence arrays, conformance refs, and descriptor uniqueness.
- [x] [serial] r[molten.plugin_extension_contracts.domain_hardening.grant_invariants] Add pure Nickel predicates for grant proof refs, evidence refs, attenuation depth/window coherence, and revoked-grant revocation evidence.
- [x] [parallel] r[molten.plugin_extension_contracts.domain_hardening.authoring_contracts] Add positive plugin extension contract fixture coverage and negative fixtures for malformed refs, invalid profile/id/version, empty evidence arrays, and duplicate descriptors.
- [x] [parallel] r[molten.plugin_extension_contracts.domain_hardening.grant_invariants] Add positive grant fixture coverage and negative fixtures for missing proofs, malformed refs, over-delegation, inverted validity window, and missing revocation evidence.
- [x] [serial] r[molten.plugin_extension_contracts.domain_hardening.authoring_contracts] Run focused Nickel fixture validation, Rust plugin Preserves admission tests, and `cairn validate --root .`, or record the blocker and next best check.
