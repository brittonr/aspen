# Unison Typed Storage Delta: Derived Archive Sidecars

## ADDED Requirements

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
