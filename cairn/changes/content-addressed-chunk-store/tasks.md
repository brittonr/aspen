## Phase 1: Manifest and chunk model

- [x] [serial] r[molten.chunk_store.manifest_model] Define canonical chunk manifest DTOs with object kind, total length, chunker version, chunk refs/Merkle root, metadata hash, policy refs, and evidence refs.
- [x] [serial] r[molten.chunk_store.chunk_ref_model] Define chunk refs with hash, length, domain, chunker version, optional compression/encryption refs, location hints, and evidence refs.
- [x] [serial] r[molten.chunk_store.fixed_v1] Implement deterministic fixed-size chunking as the first chunker version.
- [x] [parallel] r[molten.chunk_store.no_transport_identity] Document that Iroh blob ids and storage locations are hints, not Molten object identity.

## Phase 2: Verification and local index

- [x] [serial] r[molten.chunk_store.streaming_verify] Verify manifest hash/root, chunk hash/length, chunk order/proofs, and reconstructed total length during streaming fetch.
- [x] [serial] r[molten.chunk_store.redb_index] Add a Redb-backed local index for manifests, chunks, availability, pins, partial fetch state, and receipts.
- [x] [parallel] r[molten.chunk_store.range_reads] Support fixed-size range reads by mapping byte ranges to chunk refs and verifying relevant chunks.
- [x] [parallel] r[molten.chunk_store.resumable_fetch] Support resumable fetch by requesting only missing chunks from a manifest.

## Phase 3: Policy, confidentiality, and GC

- [x] [serial] r[molten.chunk_store.pin_gc] Track object-manifest and chunk-level pins and deny GC for chunks reachable from pinned or retained manifests.
- [x] [serial] r[molten.chunk_store.receipts] Emit receipts for manifest creation, chunk verification, fetch, range read, dedup hit, pin, unpin, GC, tombstone, and denial.
- [x] [parallel] r[molten.chunk_store.confidentiality] Support encryption/redaction policy metadata and avoid plaintext chunk hash leakage when confidentiality policy requires protected commitments.
- [x] [parallel] r[molten.chunk_store.compression_modes] Represent compression/encryption ordering explicitly in manifests.

## Phase 4: Adapter integration

- [x] [serial] r[molten.chunk_store.iroh_adapter] Map chunk and manifest fetch/store to Iroh blobs while preserving canonical manifest identity.
- [x] [serial] r[molten.chunk_store.remote_sync] Use manifests for remote artifact sync missing-chunk calculation and resumable fetch.
- [x] [parallel] r[molten.chunk_store.typed_storage] Store large typed-storage values as manifest refs and verify chunks before load.
- [x] [parallel] r[molten.chunk_store.replay_snapshots] Use manifest refs for replay snapshots/logs and allow partial chunk fetch for first-divergence debugging.
- [x] [parallel] r[molten.chunk_store.catalog] Expose manifest/chunk availability, dedup ratio, and pin state through catalog/MCP subject to visibility policy.

## Phase 5: Tests

- [x] [serial] r[molten.chunk_store.identity_tests] Add tests that fixed_v1 chunking produces stable manifest ids for identical bytes and different ids when bytes or chunker params change.
- [x] [serial] r[molten.chunk_store.dedup_tests] Add tests showing chunks deduplicate across artifact, storage, and replay objects.
- [x] [serial] r[molten.chunk_store.verify_tests] Add tests that corrupted, missing, reordered, or wrong-length chunks are rejected.
- [x] [parallel] r[molten.chunk_store.gc_tests] Add tests that chunks reachable from pinned manifests cannot be deleted and become eligible after all pins are removed.
- [x] [parallel] r[molten.chunk_store.property_tests] Add Hegel property tests for chunking determinism, range-read correctness, resumable fetch completeness, and no-dangling-chunk invariants.
