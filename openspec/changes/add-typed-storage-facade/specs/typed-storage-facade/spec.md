## ADDED Requirements

### Requirement: Typed Storage Resource Handles [r[typed-storage-facade.resource-handles]]

Aspen MUST define typed storage resource handles that bind a logical resource name to codec version, schema/type hash, storage backend, and access policy.

#### Scenario: Cell handle carries schema identity [r[typed-storage-facade.resource-handles.cell-schema]]

- GIVEN a `Cell<T>` resource is created or opened
- WHEN Aspen records or validates the handle
- THEN the handle MUST include resource identity, codec version, schema/type hash for `T`, backend location, and access policy summary

#### Scenario: OrderedTable handle carries key and value schema identity [r[typed-storage-facade.resource-handles.ordered-table-schema]]

- GIVEN an `OrderedTable<K,V>` resource is created or opened
- WHEN Aspen records or validates the handle
- THEN the handle MUST include key schema hash, value schema hash, ordering policy, range-limit policy, and backend location

### Requirement: Typed Read and Write Validation [r[typed-storage-facade.read-write-validation]]

Aspen MUST validate codec and schema identity when reading or writing typed durable values.

#### Scenario: Matching schema reads successfully [r[typed-storage-facade.read-write-validation.matching-read]]

- GIVEN a stored value was written with the same codec version and schema hash expected by the resource handle
- WHEN typed read runs
- THEN Aspen MUST decode the value and return it through the typed facade

#### Scenario: Mismatched schema fails explicitly [r[typed-storage-facade.read-write-validation.schema-mismatch]]

- GIVEN a stored value or resource metadata has a codec version or schema hash different from the caller's expected handle
- WHEN typed read or write validation runs
- THEN Aspen MUST return a typed schema-mismatch error rather than silently decoding with the wrong shape

### Requirement: OrderedTable Range Bounds [r[typed-storage-facade.ordered-table-bounds]]

Aspen MUST require bounded range operations for typed ordered tables.

#### Scenario: Bounded range query [r[typed-storage-facade.ordered-table-bounds.bounded-range]]

- GIVEN a caller performs an ordered-table range query
- WHEN the query is admitted
- THEN it MUST include explicit start/end or prefix bounds and a result limit within policy

#### Scenario: Unbounded range rejected [r[typed-storage-facade.ordered-table-bounds.reject-unbounded]]

- GIVEN a caller requests a full ordered-table scan without an approved limit
- WHEN admission validates the query
- THEN Aspen MUST reject the operation before scanning storage

### Requirement: Typed Storage Transactions and Batch Reads [r[typed-storage-facade.transactions-batch]]

Aspen MUST provide transaction and batched-read semantics for typed resources where backed by Raft/KV storage.

#### Scenario: Multi-resource transaction commits atomically [r[typed-storage-facade.transactions-batch.atomic-commit]]

- GIVEN a transaction updates multiple typed cells or ordered-table entries in the same transaction domain
- WHEN the transaction commits successfully
- THEN all updates MUST become visible atomically through the typed facade

#### Scenario: Batched reads preserve per-read schema validation [r[typed-storage-facade.transactions-batch.batch-validation]]

- GIVEN a caller batches reads across typed resources
- WHEN the batch executes
- THEN each read MUST still validate its own codec version and schema hash before returning a value

### Requirement: Typed Storage Receipts and Redaction [r[typed-storage-facade.receipts-redaction]]

Aspen MUST emit bounded, secret-safe receipts for typed storage resource creation, migration, schema mismatch, and secret/config access decisions.

#### Scenario: Receipt includes handles not raw values [r[typed-storage-facade.receipts-redaction.handles-only]]

- GIVEN a typed storage operation touches values, config secrets, tokens, or credentials
- WHEN a receipt or diagnostic is emitted
- THEN it MUST include resource handles, schema hashes, operation kind, and status without exposing raw secret material or unbounded values
