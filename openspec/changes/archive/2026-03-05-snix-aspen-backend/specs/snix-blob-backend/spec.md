## ADDED Requirements

### Requirement: BlobService has operation

The system SHALL check whether a blob exists in Aspen's iroh-blobs store by its BLAKE3 digest. The 32-byte B3Digest from snix MUST map directly to iroh-blobs' BLAKE3 hash.

#### Scenario: Blob exists in iroh-blobs

- **WHEN** `has()` is called with a B3Digest of a previously stored blob
- **THEN** the service SHALL return `Ok(true)`

#### Scenario: Blob does not exist

- **WHEN** `has()` is called with a B3Digest not present in iroh-blobs
- **THEN** the service SHALL return `Ok(false)`

### Requirement: BlobService open_read operation

The system SHALL return an async reader for blob content from iroh-blobs. The reader MUST implement `AsyncRead + AsyncSeek` as required by snix's `BlobReader` trait.

#### Scenario: Read existing blob

- **WHEN** `open_read()` is called with a digest of an existing blob
- **THEN** the service SHALL return `Ok(Some(reader))` where the reader yields the blob content
- **AND** the BLAKE3 hash of the yielded content SHALL match the requested digest

#### Scenario: Read non-existent blob

- **WHEN** `open_read()` is called with a digest not present in iroh-blobs
- **THEN** the service SHALL return `Ok(None)`

### Requirement: BlobService open_write operation

The system SHALL accept blob writes and store them in iroh-blobs. On close, it MUST return the BLAKE3 digest of the written content.

#### Scenario: Write and close blob

- **WHEN** data is written to the `BlobWriter` returned by `open_write()` and `close()` is called
- **THEN** the writer SHALL return the B3Digest of the written content
- **AND** a subsequent `has()` call with that digest SHALL return `Ok(true)`

#### Scenario: Close empty writer

- **WHEN** `close()` is called on a writer with no data written
- **THEN** the writer SHALL return the B3Digest of the empty blob

### Requirement: BlobService chunks returns None

The system SHALL return `Ok(Some(vec![]))` for chunks, indicating no sub-blob chunking metadata is available. iroh-blobs manages its own transfer-level chunking.

#### Scenario: Query chunks for existing blob

- **WHEN** `chunks()` is called for an existing blob
- **THEN** the service SHALL return `Ok(Some(vec![]))` indicating no chunk metadata

### Requirement: B3Digest byte compatibility

The system MUST convert between snix's `B3Digest` (32 bytes) and iroh-blobs' `Hash` type without data loss. The conversion MUST be lossless and round-trip safe.

#### Scenario: Round-trip digest through both systems

- **WHEN** a blob is written via the BlobService and its digest is obtained
- **THEN** that digest, converted to iroh-blobs Hash and back, SHALL equal the original
