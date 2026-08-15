## Phase 1: Schema identity model

- [x] [serial] r[molten.schema_identity.model] Define schema artifact identity modes for structural, unique, and optional branded structural schemas.
- [x] [serial] r[molten.schema_identity.structural_fingerprint] Compute domain-separated structural fingerprints over normalized schema shapes.
- [x] [serial] r[molten.schema_identity.unique_ids] Treat unique schema identity as schema artifact id plus admitted alias metadata, not mutable names.
- [x] [parallel] r[molten.schema_identity.no_unison_typechecker] Document that Unison unique/structural types are prior art only and Molten does not adopt Unison's typechecker or hash format.

## Phase 2: Compatibility decisions

- [x] [serial] r[molten.schema_identity.compatibility_result] Define structured compatibility results for exact match, structural match, brand match, alias, migration available, mismatch, and policy denial.
- [x] [serial] r[molten.schema_identity.policy_gate] Gate schema alias and compatibility override decisions through Nickel/Basalt/Trellis policy.
- [x] [parallel] r[molten.schema_identity.receipts] Emit Cairn receipts for schema compatibility decisions at trust boundaries.
- [x] [parallel] r[molten.schema_identity.semantic_search] Add registry queries for structurally equivalent schemas and nominal dependents.

## Phase 3: Integration

- [x] [serial] r[molten.schema_identity.storage_integration] Use schema identity decisions in typed-storage writes, loads, and migrations.
- [x] [serial] r[molten.schema_identity.choreography_payloads] Use schema identity decisions in choreography payload registries and protocol upgrade checks.
- [x] [parallel] r[molten.schema_identity.effect_schemas] Use schema identity decisions for effect-request and effect-response schemas.
- [x] [parallel] r[molten.schema_identity.policy_contract_schemas] Use schema identity decisions for Nickel and Steel contract input/output schemas.

## Phase 4: Tests

- [x] [serial] r[molten.schema_identity.structural_tests] Add tests showing structural schemas with equal normalized shapes are compatible.
- [x] [serial] r[molten.schema_identity.unique_tests] Add tests showing unique schemas with equal shapes are incompatible without explicit alias or migration.
- [x] [parallel] r[molten.schema_identity.migration_tests] Add tests showing mismatches can be admitted only through migration recipe artifacts.
- [x] [parallel] r[molten.schema_identity.property_tests] Add Hegel property tests for fingerprint determinism, alias safety, and compatibility-result invariants.
