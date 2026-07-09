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

### Requirement: Typed storage keeps rkyv materializations non-authoritative
r[molten.storage.derived_archive_sidecars] Typed storage MAY keep rkyv-backed zero-copy materializations only as tagged, rebuildable sidecars for local read acceleration; durable stored values, value refs, schema bindings, receipts, and migration traces MUST remain canonical Preserves values or content refs.

#### Scenario: Stored value identity remains canonical
- GIVEN a typed storage value has a canonical Preserves value ref and a local rkyv sidecar exists for fast reads
- WHEN a caller verifies storage identity or schema conformance
- THEN verification uses the canonical Preserves value ref, schema ref, policy refs, and receipts rather than the rkyv archive bytes

#### Scenario: Sidecar loss does not lose durable value
- GIVEN a rkyv sidecar is deleted or invalidated
- WHEN the typed storage adapter loads the stored value
- THEN the durable value remains available from canonical Preserves bytes or content refs, and the sidecar may be rebuilt or skipped

### Requirement: rkyv sidecars cannot weaken raw-memory storage prohibition
r[molten.storage.derived_archive_no_raw_memory_claims] rkyv sidecars MUST NOT be used to persist raw Rust memory layouts, pointers, closures, debug formatting, or unchecked process-local state as durable typed storage values.

#### Scenario: Raw memory sidecar is rejected as storage value
- GIVEN a sidecar manifest does not bind canonical Preserves source refs and schema refs
- WHEN code attempts to promote it to a durable typed storage value
- THEN typed storage admission rejects the write before mutating storage metadata

#### Scenario: Migration uses canonical source values
- GIVEN a stored value has both a canonical Preserves representation and a derived rkyv materialization
- WHEN a schema migration is planned
- THEN migration planning reads and records the canonical source value identity, not the derived rkyv layout
