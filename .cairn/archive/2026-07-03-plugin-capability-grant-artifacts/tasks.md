# Tasks: plugin-capability-grant-artifacts

## Phase 1: Grant artifact model

- [x] [serial] r[molten.plugin_capability_grants.grant_artifact] Define pure `plugin-capability-grant-v1` canonical Preserves constructors and parsers for subject plugin, manifest, extension contract, hostcall descriptor, operation, schemas, resources, effects, policies, issuer/proof refs, attenuation, revocation evidence, and replay class.
- [x] [serial] r[molten.plugin_capability_grants.grant_artifact] Add typed ref wrappers or classifier helpers so `CapabilityGrantRef` cannot be confused with artifact, schema, policy, resource, effect, or receipt refs inside plugin admission code.
- [x] [parallel] r[molten.plugin_capability_grants.grant_artifact] Add catalog/show summaries for capability grant artifacts when they are stored or inspected.

## Phase 2: Hostcall admission integration

- [x] [serial] r[molten.plugin_capability_grants.hostcall_admission] Extend plugin hostcall receipt input and canonical receipt shape to bind `capability-grant-refs` while preserving existing authority refs as proof/compatibility evidence.
- [x] [serial] r[molten.plugin_capability_grants.hostcall_admission] Implement pure grant matching for manifest ref, plugin ref/id, extension contract ref, hostcall descriptor ref, operation, input/output schema refs, resource scope, effect manifests/receipts, policy refs, and proof refs.
- [x] [parallel] r[molten.plugin_capability_grants.hostcall_admission] Fail closed when a descriptor requires typed capability grants but the request supplies only generic authority refs or unrelated BLAKE3 refs.
- [x] [parallel] r[molten.plugin_capability_grants.hostcall_admission] Preserve deny-by-default Steel and Wasm behavior so only declared hostcall imports/functions can reach the host boundary.

## Phase 3: Revocation and attenuation

- [x] [serial] r[molten.plugin_capability_grants.revocation_attenuation] Add deterministic attenuation checks for operation narrowing, resource sub-scope, schema/profile constraints, delegation depth, budget refs, turn/tick validity evidence, and replay/idempotency class.
- [x] [serial] r[molten.plugin_capability_grants.revocation_attenuation] Add revocation evidence parsing and matching without core filesystem, clock, network, or registry reads.
- [x] [parallel] r[molten.plugin_capability_grants.revocation_attenuation] Add receipt diagnostics that distinguish expired, revoked, over-delegated, wrong-resource, wrong-operation, and wrong-manifest grants.

## Phase 4: Nickel authoring rail

- [x] [serial] r[molten.plugin_capability_grants.nickel_authoring] Add typed Nickel contracts for plugin capability grants and grant templates.
- [x] [parallel] r[molten.plugin_capability_grants.nickel_authoring] Add a valid storage-read grant fixture plus negative fixtures for raw artifact ref as grant, wrong operation, wrong manifest, wrong resource, missing proof, expired grant, revoked grant, and over-delegation.
- [x] [serial] r[molten.plugin_capability_grants.nickel_authoring] Document or script the export/check path that emits checked-in canonical Preserves evidence consumed by Rust validation without runtime Nickel execution.

## Phase 5: Positive and negative tests

- [x] [parallel] r[molten.plugin_capability_grants.grant_artifact] Add positive constructor/parser determinism tests for canonical grant refs.
- [x] [parallel] r[molten.plugin_capability_grants.hostcall_admission] Add a positive hostcall test where a matching capability grant admits `storage.read`.
- [x] [parallel] r[molten.plugin_capability_grants.hostcall_admission] Add negative tests for BLAKE3 artifact refs supplied where capability grant refs are required, generic-only authority, wrong operation, wrong descriptor, wrong manifest, wrong schema, and wrong resource.
- [x] [parallel] r[molten.plugin_capability_grants.revocation_attenuation] Add negative tests for expired, revoked, over-delegated, and out-of-budget grants.
- [x] [parallel] r[molten.plugin_capability_grants.nickel_authoring] Add Nickel validation tests for valid and invalid grant fixtures.

## Phase 6: Evidence and validation

- [x] [serial] r[molten.plugin_capability_grants.grant_artifact] r[molten.plugin_capability_grants.hostcall_admission] r[molten.plugin_capability_grants.revocation_attenuation] r[molten.plugin_capability_grants.nickel_authoring] Run focused plugin tests, Nickel export checks, and Cairn validation.
