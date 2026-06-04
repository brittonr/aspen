## Phase 1: Durable value model

- [x] [serial] r[molten.storage.typed_ref_model] Define typed durable reference DTOs with namespace, key, schema ref, value ref/hash, producer artifact ref, policy refs, and evidence refs.
- [x] [serial] r[molten.storage.canonical_values] Persist small values as canonical Preserves bytes and large values as content refs with verified hashes.
- [x] [serial] r[molten.storage.schema_binding] Bind stored values to Preserves schema/type artifact ids and reject writes that do not conform.
- [x] [parallel] r[molten.storage.no_raw_memory] Document that storage never persists raw Rust memory layouts, pointers, closures, or debug formatting.

## Phase 2: Storage handler and receipts

- [x] [serial] r[molten.storage.effect_manifest] Require storage read/write effects in executable artifact effect manifests.
- [x] [serial] r[molten.storage.write_admission] Gate storage writes through capability, namespace, schema, and policy admission before Redb mutation.
- [x] [serial] r[molten.storage.load_admission] Gate storage loads through capability, schema/type compatibility, content-integrity, and receipt validation.
- [x] [parallel] r[molten.storage.redb_adapter] Add a Redb-backed local typed-storage adapter for namespace/key/value metadata.
- [x] [parallel] r[molten.storage.blob_payloads] Store large immutable value payloads through content refs suitable for Iroh blobs.
- [x] [parallel] r[molten.storage.cairn_receipts] Emit Cairn receipts for put, get, deny, migrate, and verify operations.

## Phase 3: Migration artifacts

- [x] [serial] r[molten.storage.migration_model] Define migration recipe artifacts with source schema, target schema, transformer artifact, policies, tests, and receipts.
- [x] [serial] r[molten.storage.migration_admission] Require policy admission and handler binding before running migration artifacts.
- [x] [parallel] r[molten.storage.lazy_migration] Support explicit or lazy-on-read migration planning without hiding schema changes from callers.
- [x] [parallel] r[molten.storage.migration_trace] Record original value hash, migration artifact id, result value hash, and receipt refs for every migration.

## Phase 4: Tests

- [x] [serial] r[molten.storage.roundtrip_tests] Add tests for writing and loading schema-tagged Preserves values through the local adapter.
- [x] [serial] r[molten.storage.schema_mismatch_tests] Add tests that reject schema mismatches unless an admitted migration exists.
- [x] [parallel] r[molten.storage.snapshot_authority_tests] Add tests ensuring snapshots cannot mint capabilities not present in stored authority metadata.
- [x] [parallel] r[molten.storage.property_tests] Add Hegel property tests for canonical value hashes, typed ref stability, and migration trace invariants.
