# Content Addressed Chunk Store Specification

## Purpose

Defines the `content-addressed-chunk-store` capability.

## Requirements

### Requirement: Chunk manifests are canonical
r[molten.chunk_store.manifest_model] Molten MUST represent chunked objects with canonical `chunk-manifest-v1` records that bind object kind, total length, chunker version, chunk refs, Merkle/root refs, metadata refs, policy refs, and evidence refs.

#### Scenario: Manifest identity is stable
- GIVEN identical object bytes, metadata, chunker parameters, policy refs, and evidence refs
- WHEN Molten renders the chunk manifest
- THEN the manifest has the same canonical content ref.

### Requirement: Chunk refs are canonical
r[molten.chunk_store.chunk_ref_model] Molten MUST represent chunks with canonical refs that bind hash, length, domain, chunker version, transform metadata, location hints, and evidence refs.

#### Scenario: Chunk ref binds fixed chunk metadata
- GIVEN a chunk produced by the fixed chunker
- WHEN Molten renders its chunk ref
- THEN the ref records length, domain, chunker version, transforms, and evidence independently of transport location.

### Requirement: Fixed v1 chunking is deterministic
r[molten.chunk_store.fixed_v1] Molten MUST provide deterministic fixed-size chunking as the first chunker version.

#### Scenario: Same bytes chunk identically
- GIVEN the same byte stream and fixed_v1 chunk size
- WHEN Molten chunks the stream twice
- THEN chunk boundaries and chunk refs are identical.

### Requirement: Transport ids are not object identity
r[molten.chunk_store.no_transport_identity] Molten MUST treat Iroh blob ids, filesystem paths, and storage locations as hints, not canonical Molten object identity.

#### Scenario: Iroh ticket preserves manifest identity
- GIVEN a manifest fetched from an Iroh-style ticket
- WHEN Molten imports the chunks
- THEN the manifest ref remains the canonical identity rather than the ticket or blob id.

### Requirement: Streaming verification is fail-closed
r[molten.chunk_store.streaming_verify] Molten MUST verify manifest hash/root, chunk hash/length, chunk order/proofs, and reconstructed total length during streaming fetch or read.

#### Scenario: Corrupt chunk is rejected
- GIVEN a stored chunk whose bytes do not match its ref
- WHEN Molten verifies or reads the manifest
- THEN the operation denies before returning object bytes.

### Requirement: Local index is durable
r[molten.chunk_store.redb_index] Molten MUST maintain a Redb-backed local index for manifests, chunks, availability, pins, partial fetch state, and receipts.

#### Scenario: Rebuild restores derived state
- GIVEN chunk store files and historical receipts
- WHEN Molten rebuilds the index
- THEN manifests, chunks, availability, pins, partial fetch state, and receipts are represented consistently.

### Requirement: Range reads verify relevant chunks
r[molten.chunk_store.range_reads] Molten MUST support fixed-size range reads by mapping byte ranges to chunk refs and verifying all relevant chunks before returning bytes.

#### Scenario: Range read returns verified slice
- GIVEN a manifest and byte range crossing chunk boundaries
- WHEN Molten performs a range read
- THEN only verified bytes in the requested range are returned.

### Requirement: Fetches are resumable
r[molten.chunk_store.resumable_fetch] Molten MUST support resumable fetch by requesting only missing chunks from a manifest.

#### Scenario: Missing chunks are fetched only once
- GIVEN a destination store that already has some manifest chunks
- WHEN Molten syncs from a source store
- THEN it fetches missing chunks and preserves existing verified chunks.

### Requirement: Pins protect GC
r[molten.chunk_store.pin_gc] Molten MUST track object-manifest and chunk-level pins and deny GC for chunks reachable from pinned or retained manifests.

#### Scenario: Pinned chunk is not removed
- GIVEN a pinned manifest or chunk
- WHEN GC evaluates removal
- THEN reachable chunks remain present and receipt diagnostics explain the retention.

### Requirement: Operations emit receipts
r[molten.chunk_store.receipts] Molten MUST emit receipts for manifest creation, chunk verification, fetch, range read, dedup hit, pin, unpin, GC, tombstone, and denial decisions.

#### Scenario: Denied operation has evidence
- GIVEN a chunk operation that fails validation
- WHEN Molten denies the operation
- THEN it emits a receipt binding the decision, diagnostics, manifest or chunk refs, and checks.

### Requirement: Confidentiality metadata is explicit
r[molten.chunk_store.confidentiality] Molten MUST support encryption/redaction policy metadata and avoid plaintext chunk hash leakage when confidentiality policy requires protected commitments.

#### Scenario: Protected commitment denies plaintext exposure
- GIVEN a manifest marked with protected-commitment confidentiality
- WHEN an operation would expose plaintext chunks without reveal authority
- THEN Molten denies and emits evidence instead of plaintext bytes.

### Requirement: Transform ordering is represented
r[molten.chunk_store.compression_modes] Molten MUST represent compression/encryption ordering explicitly in manifests and chunk refs.

#### Scenario: Unsupported transform denies safely
- GIVEN a manifest declaring an unsupported compression or encryption transform
- WHEN Molten reads or verifies plaintext bytes
- THEN the operation denies before exposing bytes.

### Requirement: Iroh adapter preserves identity
r[molten.chunk_store.iroh_adapter] Molten MUST map chunk and manifest fetch/store operations to Iroh blobs while preserving canonical manifest identity.

#### Scenario: Published blobs fetch to same manifest
- GIVEN a manifest published through the Iroh-style adapter
- WHEN another store fetches the ticket
- THEN the fetched manifest ref matches the original manifest ref.

### Requirement: Remote sync uses manifests
r[molten.chunk_store.remote_sync] Molten MUST use manifests for remote artifact sync missing-chunk calculation and resumable fetch.

#### Scenario: Remote sync resumes from manifest state
- GIVEN a remote manifest and a partial local store
- WHEN Molten syncs missing chunks
- THEN only unavailable chunks are fetched and indexed.

### Requirement: Typed storage large values use manifests
r[molten.chunk_store.typed_storage] Molten MUST store large typed-storage values as manifest refs and verify chunks before loading them.

#### Scenario: Typed storage load verifies chunks
- GIVEN a typed-storage record backed by a chunk manifest
- WHEN Molten loads the value
- THEN chunk hashes and manifest root are verified before the value is returned.

### Requirement: Replay snapshots use manifests
r[molten.chunk_store.replay_snapshots] Molten MUST use manifest refs for replay snapshots and logs and support partial chunk fetch for first-divergence debugging.

#### Scenario: Divergence debug fetches needed chunks
- GIVEN a replay snapshot manifest and missing local chunks
- WHEN Molten investigates first divergence
- THEN it fetches or reports only the chunks needed for the relevant snapshot or log range.

### Requirement: Catalog exposes chunk store state
r[molten.chunk_store.catalog] Molten MUST expose manifest/chunk availability, dedup ratio, and pin state through catalog and MCP views subject to visibility policy.

#### Scenario: Hidden refs are not rendered
- GIVEN a chunk catalog request with hidden refs
- WHEN Molten renders chunk availability, dedup, and pin summaries
- THEN hidden manifest or chunk refs are omitted from the catalog and MCP response.

### Requirement: Manifest identity tests cover stable refs
r[molten.chunk_store.identity_tests] Molten MUST test that fixed_v1 chunking produces stable manifest ids for identical bytes and different ids when bytes or chunker parameters change.

#### Scenario: Identity test catches chunker drift
- GIVEN changed chunker parameters
- WHEN the identity test computes the manifest ref
- THEN it differs from the original manifest ref.

### Requirement: Dedup tests cover shared chunks
r[molten.chunk_store.dedup_tests] Molten MUST test that chunks deduplicate across artifact, storage, and replay objects.

#### Scenario: Shared bytes produce dedup evidence
- GIVEN two objects sharing chunk bytes
- WHEN both are stored
- THEN dedup receipts or summaries show the shared chunk was not rewritten.

### Requirement: Verification tests reject invalid chunks
r[molten.chunk_store.verify_tests] Molten MUST test that corrupted, missing, reordered, or wrong-length chunks are rejected.

#### Scenario: Wrong length chunk denies
- GIVEN a chunk file with the wrong length
- WHEN Molten verifies its manifest
- THEN verification fails closed.

### Requirement: GC tests cover pin safety
r[molten.chunk_store.gc_tests] Molten MUST test that chunks reachable from pinned manifests cannot be deleted and become eligible after all pins are removed.

#### Scenario: Unpinned chunks become eligible
- GIVEN a manifest whose pins have been removed
- WHEN GC runs with valid deletion evidence
- THEN eligible chunks may be removed and tombstone evidence is emitted.

### Requirement: Property tests cover chunk invariants
r[molten.chunk_store.property_tests] Molten MUST add property tests for chunking determinism, range-read correctness, resumable fetch completeness, and no-dangling-chunk invariants.

#### Scenario: Generated sync leaves no missing chunks
- GIVEN generated bounded byte streams and partial destination stores
- WHEN property tests run resumable sync
- THEN the destination has no missing chunks for the synced manifest.

### Requirement: Operator gateway verified range readback
r[molten.operator_gateway.verified_range_read] Molten MUST verify chunk-store manifest identity, relevant chunk hashes, chunk lengths, transform support, and reconstructed byte ranges before any operator gateway response exposes bytes.

#### Scenario: Valid range returns verified bytes
- GIVEN a visible chunk manifest and a bounded byte-range request
- WHEN the operator gateway maps the byte range to chunk refs
- THEN every relevant chunk is verified before response bytes are emitted
- AND the gateway range receipt binds the manifest ref, normalized range, chunk refs, and verification checks.

#### Scenario: Corrupt chunk denies before response
- GIVEN a requested range whose backing chunk bytes do not match the chunk ref or declared length
- WHEN the operator gateway verifies the range
- THEN it emits a deny receipt with corrupt-chunk diagnostics
- AND no plaintext response bytes are exposed.

#### Scenario: Unsupported transform denies before response
- GIVEN a manifest range that requires an unsupported compression, encryption, or transform mode
- WHEN the operator gateway evaluates the range
- THEN it emits a deny receipt for unsupported transform
- AND the gateway does not expose transformed or plaintext bytes.
