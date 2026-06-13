# Unison Typed Storage Specification

## Purpose

Defines the `unison-typed-storage` capability.

## Requirements

### Requirement: System MUST Define typed durable reference DTOs with namespace, key, schema ref, value ref/hash, producer artifact ref, policy refs, and evidence refs
r[molten.storage.typed_ref_model] The system MUST Define typed durable reference DTOs with namespace, key, schema ref, value ref/hash, producer artifact ref, policy refs, and evidence refs.

### Requirement: System MUST Persist small values as canonical Preserves bytes and large values as content refs with verified hashes
r[molten.storage.canonical_values] The system MUST Persist small values as canonical Preserves bytes and large values as content refs with verified hashes.

### Requirement: System MUST Bind stored values to Preserves schema/type artifact ids and reject writes that do not conform
r[molten.storage.schema_binding] The system MUST Bind stored values to Preserves schema/type artifact ids and reject writes that do not conform.

### Requirement: System MUST Document that storage never persists raw Rust memory layouts, pointers, closures, or debug formatting
r[molten.storage.no_raw_memory] The system MUST Document that storage never persists raw Rust memory layouts, pointers, closures, or debug formatting.

### Requirement: System MUST Require storage read/write effects in executable artifact effect manifests
r[molten.storage.effect_manifest] The system MUST Require storage read/write effects in executable artifact effect manifests.

### Requirement: System MUST Gate storage writes through capability, namespace, schema, and policy admission before Redb mutation
r[molten.storage.write_admission] The system MUST Gate storage writes through capability, namespace, schema, and policy admission before Redb mutation.

### Requirement: System MUST Gate storage loads through capability, schema/type compatibility, content-integrity, and receipt validation
r[molten.storage.load_admission] The system MUST Gate storage loads through capability, schema/type compatibility, content-integrity, and receipt validation.

### Requirement: System MUST Add a Redb-backed local typed-storage adapter for namespace/key/value metadata
r[molten.storage.redb_adapter] The system MUST Add a Redb-backed local typed-storage adapter for namespace/key/value metadata.

### Requirement: System MUST Store large immutable value payloads through content refs suitable for Iroh blobs
r[molten.storage.blob_payloads] The system MUST Store large immutable value payloads through content refs suitable for Iroh blobs.

### Requirement: System MUST Emit Cairn receipts for put, get, deny, migrate, and verify operations
r[molten.storage.cairn_receipts] The system MUST Emit Cairn receipts for put, get, deny, migrate, and verify operations.

### Requirement: System MUST Define migration recipe artifacts with source schema, target schema, transformer artifact, policies, tests, and receipts
r[molten.storage.migration_model] The system MUST Define migration recipe artifacts with source schema, target schema, transformer artifact, policies, tests, and receipts.

### Requirement: System MUST Require policy admission and handler binding before running migration artifacts
r[molten.storage.migration_admission] The system MUST Require policy admission and handler binding before running migration artifacts.

### Requirement: System MUST Support explicit or lazy-on-read migration planning without hiding schema changes from callers
r[molten.storage.lazy_migration] The system MUST Support explicit or lazy-on-read migration planning without hiding schema changes from callers.

### Requirement: System MUST Record original value hash, migration artifact id, result value hash, and receipt refs for every migration
r[molten.storage.migration_trace] The system MUST Record original value hash, migration artifact id, result value hash, and receipt refs for every migration.

### Requirement: System MUST Add tests for writing and loading schema-tagged Preserves values through the local adapter
r[molten.storage.roundtrip_tests] The system MUST Add tests for writing and loading schema-tagged Preserves values through the local adapter.

### Requirement: System MUST Add tests that reject schema mismatches unless an admitted migration exists
r[molten.storage.schema_mismatch_tests] The system MUST Add tests that reject schema mismatches unless an admitted migration exists.

### Requirement: System MUST Add tests ensuring snapshots cannot mint capabilities not present in stored authority metadata
r[molten.storage.snapshot_authority_tests] The system MUST Add tests ensuring snapshots cannot mint capabilities not present in stored authority metadata.

### Requirement: System MUST Add Hegel property tests for canonical value hashes, typed ref stability, and migration trace invariants
r[molten.storage.property_tests] The system MUST Add Hegel property tests for canonical value hashes, typed ref stability, and migration trace invariants.
