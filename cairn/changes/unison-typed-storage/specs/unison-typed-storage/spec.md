# unison typed storage Delta Spec

## ADDED Requirements

### Requirement: Define typed durable reference DTOs with namespace, key, schema ref, value ref/hash, producer artifact ref, policy refs, and evidence refs
r[molten.storage.typed_ref_model] Define typed durable reference DTOs with namespace, key, schema ref, value ref/hash, producer artifact ref, policy refs, and evidence refs.

### Requirement: Persist small values as canonical Preserves bytes and large values as content refs with verified hashes
r[molten.storage.canonical_values] Persist small values as canonical Preserves bytes and large values as content refs with verified hashes.

### Requirement: Bind stored values to Preserves schema/type artifact ids and reject writes that do not conform
r[molten.storage.schema_binding] Bind stored values to Preserves schema/type artifact ids and reject writes that do not conform.

### Requirement: Document that storage never persists raw Rust memory layouts, pointers, closures, or debug formatting
r[molten.storage.no_raw_memory] Document that storage never persists raw Rust memory layouts, pointers, closures, or debug formatting.

### Requirement: Require storage read/write effects in executable artifact effect manifests
r[molten.storage.effect_manifest] Require storage read/write effects in executable artifact effect manifests.

### Requirement: Gate storage writes through capability, namespace, schema, and policy admission before Redb mutation
r[molten.storage.write_admission] Gate storage writes through capability, namespace, schema, and policy admission before Redb mutation.

### Requirement: Gate storage loads through capability, schema/type compatibility, content-integrity, and receipt validation
r[molten.storage.load_admission] Gate storage loads through capability, schema/type compatibility, content-integrity, and receipt validation.

### Requirement: Add a Redb-backed local typed-storage adapter for namespace/key/value metadata
r[molten.storage.redb_adapter] Add a Redb-backed local typed-storage adapter for namespace/key/value metadata.

### Requirement: Store large immutable value payloads through content refs suitable for Iroh blobs
r[molten.storage.blob_payloads] Store large immutable value payloads through content refs suitable for Iroh blobs.

### Requirement: Emit Cairn receipts for put, get, deny, migrate, and verify operations
r[molten.storage.cairn_receipts] Emit Cairn receipts for put, get, deny, migrate, and verify operations.

### Requirement: Define migration recipe artifacts with source schema, target schema, transformer artifact, policies, tests, and receipts
r[molten.storage.migration_model] Define migration recipe artifacts with source schema, target schema, transformer artifact, policies, tests, and receipts.

### Requirement: Require policy admission and handler binding before running migration artifacts
r[molten.storage.migration_admission] Require policy admission and handler binding before running migration artifacts.

### Requirement: Support explicit or lazy-on-read migration planning without hiding schema changes from callers
r[molten.storage.lazy_migration] Support explicit or lazy-on-read migration planning without hiding schema changes from callers.

### Requirement: Record original value hash, migration artifact id, result value hash, and receipt refs for every migration
r[molten.storage.migration_trace] Record original value hash, migration artifact id, result value hash, and receipt refs for every migration.

### Requirement: Add tests for writing and loading schema-tagged Preserves values through the local adapter
r[molten.storage.roundtrip_tests] Add tests for writing and loading schema-tagged Preserves values through the local adapter.

### Requirement: Add tests that reject schema mismatches unless an admitted migration exists
r[molten.storage.schema_mismatch_tests] Add tests that reject schema mismatches unless an admitted migration exists.

### Requirement: Add tests ensuring snapshots cannot mint capabilities not present in stored authority metadata
r[molten.storage.snapshot_authority_tests] Add tests ensuring snapshots cannot mint capabilities not present in stored authority metadata.

### Requirement: Add Hegel property tests for canonical value hashes, typed ref stability, and migration trace invariants
r[molten.storage.property_tests] Add Hegel property tests for canonical value hashes, typed ref stability, and migration trace invariants.

