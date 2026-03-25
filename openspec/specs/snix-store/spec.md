## Requirements

### Requirement: Blob size bound

r[snix.store.blob-size-bound]

Blob uploads SHALL reject data exceeding `MAX_BLOB_SIZE_BYTES` (1 GB).

### Requirement: Directory entry bound

r[snix.store.directory-entry-bound]

Directory storage SHALL enforce a maximum of `MAX_DIRECTORY_ENTRIES` (100,000) entries per directory.

### Requirement: Directory depth bound

r[snix.store.depth-bound]

Recursive directory traversal SHALL enforce `MAX_DIRECTORY_DEPTH` (256) to prevent unbounded recursion.

### Requirement: Path reference bound

r[snix.store.reference-bound]

PathInfo entries SHALL have at most `MAX_PATH_REFERENCES` (10,000) store path references.

### Requirement: Signature bound

r[snix.store.signature-bound]

PathInfo entries SHALL have at most `MAX_SIGNATURES` (100) signatures.

### Requirement: Chunk size bounds

r[snix.store.chunk-size-bound]

Content-defined chunks SHALL have sizes between `MIN_CHUNK_SIZE` (16 KiB) and `MAX_CHUNK_SIZE` (256 KiB), except the final chunk which may be smaller.

### Requirement: Manifest entry bound

r[snix.store.manifest-entry-bound]

Chunk manifests SHALL have at most `ceil(blob_size / MIN_CHUNK_SIZE)` entries.

### Requirement: Circuit breaker protection

r[snix.store.circuit-breaker]

All snix service calls (blob, directory, pathinfo) SHALL be protected by circuit breakers that reject requests after consecutive failures exceed a threshold.
