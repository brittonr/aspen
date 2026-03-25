## ADDED Requirements

### Requirement: Content-defined chunking on blob write

The `BlobService` implementation SHALL chunk blobs using FastCDC content-defined chunking when the blob size exceeds the inline threshold (256 KiB). Chunks SHALL be individually stored in the underlying iroh-blobs backend, keyed by their BLAKE3 hash.

#### Scenario: Large blob is chunked on write

- **WHEN** a blob larger than 256 KiB is written via `BlobWriter::close()`
- **THEN** the blob is split into chunks with min 16 KiB, avg 64 KiB, max 256 KiB boundaries
- **AND** each chunk is stored individually in iroh-blobs keyed by its BLAKE3 hash
- **AND** a manifest mapping (ordered list of chunk hash + size pairs) is persisted
- **AND** the returned `B3Digest` is the BLAKE3 hash of the original un-chunked blob

#### Scenario: Small blob skips chunking

- **WHEN** a blob of 256 KiB or smaller is written via `BlobWriter::close()`
- **THEN** the blob is stored as a single entry in iroh-blobs without chunking
- **AND** no manifest is created

### Requirement: Chunk-level deduplication

The system SHALL deduplicate chunks across blobs. When a chunk with an identical BLAKE3 hash already exists in storage, it SHALL NOT be re-uploaded.

#### Scenario: Shared chunk between two blobs

- **WHEN** blob A and blob B share a common byte sequence that produces an identical chunk
- **THEN** the shared chunk is stored only once
- **AND** both manifests reference the same chunk hash

#### Scenario: Dedup ratio tracking

- **WHEN** a chunked blob write completes
- **THEN** the system SHALL record the dedup ratio (deduped_chunks / total_chunks) as a metric

### Requirement: Chunk metadata via BlobService::chunks()

The `BlobService::chunks()` method SHALL return real `ChunkMeta` entries for chunked blobs, providing chunk digests and sizes for verified streaming.

#### Scenario: chunks() for a chunked blob

- **WHEN** `chunks()` is called for a blob that was stored with chunking
- **THEN** it SHALL return `Some(vec![ChunkMeta { digest, size }, ...])` with one entry per chunk in order

#### Scenario: chunks() for an inline blob

- **WHEN** `chunks()` is called for a blob stored without chunking (below threshold)
- **THEN** it SHALL return `Some(vec![])` (empty, no granular chunks)

#### Scenario: chunks() for a missing blob

- **WHEN** `chunks()` is called for a non-existent blob
- **THEN** it SHALL return `Ok(None)`

### Requirement: Transparent blob reassembly on read

Reading a chunked blob SHALL reassemble it from its chunks transparently. Callers of `BlobService::open_read()` SHALL receive the original byte sequence.

#### Scenario: Read a chunked blob

- **WHEN** `open_read()` is called for a blob that was stored with chunking
- **THEN** the returned reader SHALL yield the original un-chunked bytes in order
- **AND** each chunk SHALL be BLAKE3-verified during reassembly

#### Scenario: Roundtrip integrity

- **WHEN** a blob is written, then read back
- **THEN** the read bytes SHALL be byte-identical to the written bytes regardless of chunking

### Requirement: Manifest storage bounds

Chunk manifests SHALL respect Aspen's KV value size limits. Manifests exceeding the inline threshold SHALL be stored as blobs rather than KV entries.

#### Scenario: Small manifest stored in KV

- **WHEN** a manifest is 64 KiB or smaller (serialized)
- **THEN** it SHALL be stored in Raft KV at key `snix:manifest:<blob_b3_digest_hex>`

#### Scenario: Large manifest stored as blob

- **WHEN** a manifest exceeds 64 KiB (serialized)
- **THEN** it SHALL be stored in iroh-blobs
- **AND** a KV entry SHALL store a reference to the manifest blob hash
