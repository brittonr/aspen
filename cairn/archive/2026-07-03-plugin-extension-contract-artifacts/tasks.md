# Tasks: plugin-extension-contract-artifacts

## Phase 1: Contract artifact model

- [x] [serial] r[molten.plugin_extension_contracts.contract_artifact] Define `plugin-extension-contract-v1` canonical Preserves constructors and parsers for extension id/version, ABI compatibility, lifecycle changes, hostcall descriptors, schemas, authority/resource/effect requirements, replay/idempotency, errors, conformance refs, policy refs, and supply-chain refs.
- [x] [serial] r[molten.plugin_extension_contracts.contract_artifact] Add manifest `extension-contract-refs` binding and parsing while preserving existing plugin manifest fields.
- [x] [parallel] r[molten.plugin_extension_contracts.contract_artifact] Add catalog/summary classification for extension contract artifacts if they enter the ledger.

## Phase 2: Contract-aware hostcall core

- [x] [serial] r[molten.plugin_extension_contracts.per_hostcall_requirements] Implement pure hostcall descriptor matching from manifest-bound extension contracts.
- [x] [serial] r[molten.plugin_extension_contracts.per_hostcall_requirements] Require descriptor-specific input/output schema, authority, resource, effect, and executor evidence before hostcall receipts can pass.
- [x] [parallel] r[molten.plugin_extension_contracts.per_hostcall_requirements] Preserve deny-by-default behavior for undeclared or ambient hostcalls.

## Phase 3: Nickel authoring rail

- [x] [serial] r[molten.plugin_extension_contracts.nickel_authoring] Add typed Nickel contracts for authored plugin extension definitions.
- [x] [parallel] r[molten.plugin_extension_contracts.nickel_authoring] Add valid Nickel fixture plus negative fixtures for missing schema, missing authority, invalid version, duplicate hostcall descriptor, and ambient hostcall defaults.
- [x] [serial] r[molten.plugin_extension_contracts.nickel_authoring] Add an export/check path that emits checked-in canonical evidence consumed by Rust validation without runtime Nickel execution.

## Phase 4: Positive and negative tests

- [x] [parallel] r[molten.plugin_extension_contracts.contract_artifact] Add a valid extension contract fixture bound by a plugin manifest.
- [x] [parallel] r[molten.plugin_extension_contracts.per_hostcall_requirements] Add negative tests showing unrelated non-empty authority/resource refs do not satisfy descriptor-specific requirements.
- [x] [parallel] r[molten.plugin_extension_contracts.nickel_authoring] Add Nickel validation tests for valid and invalid contract fixtures.

## Phase 5: Evidence and validation

- [x] [serial] r[molten.plugin_extension_contracts.contract_artifact] r[molten.plugin_extension_contracts.per_hostcall_requirements] r[molten.plugin_extension_contracts.nickel_authoring] Run focused plugin tests, Nickel export checks, and Cairn validation.
