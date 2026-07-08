# Tasks: plugin-grant-contract-binding

## Phase 1: Bound grant contract

- [x] [serial] r[molten.plugin_grant_contract_binding.descriptor_binding] Add a Nickel helper that validates a plugin capability grant against a supplied plugin extension contract descriptor.
- [x] [parallel] r[molten.plugin_grant_contract_binding.resource_scope] Require grant resource scope and effect refs to be subsets or exact matches of the referenced descriptor's reviewed authority/resource/effect domains.
- [x] [parallel] r[molten.plugin_grant_contract_binding.revocation_attenuation] Tighten local grant invariants for revocation evidence, validity windows, delegation depth, and replay class.

## Phase 2: Fixtures and drift

- [x] [serial] r[molten.plugin_grant_contract_binding.fixture_migration] Migrate storage grant fixtures and envelopes to the bound grant contract where generated drift is reviewed.
- [x] [parallel] r[molten.plugin_grant_contract_binding.negative_grant_bindings] Add negative fixtures for wrong contract ref, wrong descriptor, schema mismatch, operation mismatch, resource over-scope, replay mismatch, missing revocation evidence, and inverted validity.

## Phase 3: Runtime confirmation

- [x] [parallel] r[molten.plugin_grant_contract_binding.runtime_boundary] Add or confirm Rust parser/admission tests showing runtime gates still deny mismatched grants even if authored fixtures are absent.
- [x] [serial] r[molten.plugin_grant_contract_binding.descriptor_binding] Run plugin contract drift gate, focused plugin host tests, and `nix run path:$PWD#cairn -- validate --root .`.
