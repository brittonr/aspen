# 8. Content-Addressed Blobs via iroh-blobs

**Status:** accepted

## Context

Aspen needs to store and transfer large binary objects — CI build artifacts, Nix store paths, FUSE file contents, and document attachments. These objects range from kilobytes to gigabytes and must be transferable between nodes efficiently.

iroh-blobs provides content-addressed blob storage using BLAKE3 hashing. Blobs are identified by their hash, deduplicated automatically, and transferred over Iroh's QUIC transport with built-in integrity verification. iroh-docs adds CRDT-based replication for mutable document collections built on top of iroh-blobs.

## Decision

Use iroh-blobs for all large object storage and transfer in Aspen. The `aspen-blob` crate wraps iroh-blobs with Aspen-specific APIs. The `aspen-docs` crate wraps iroh-docs for CRDT document replication.

Both are behind feature flags (`blob` and `docs` respectively) so they're opt-in.

Content addressing means:

- Same content = same hash = automatic deduplication
- Transfer integrity is verified by the hash — no separate checksum step
- Blobs are immutable once stored — mutation happens at the reference layer (iroh-docs)

Alternatives considered:

- (+) S3-compatible object storage: standard, widely supported
- (-) S3: requires external infrastructure, not P2P, no built-in Iroh integration
- (+) Git LFS: content-addressed, familiar to developers
- (-) Git LFS: requires a Git server, not designed for arbitrary blob transfer
- (+) iroh-blobs: content-addressed, P2P transfer over existing Iroh transport, BLAKE3
- (-) iroh-blobs: tied to Iroh ecosystem, not a standard protocol
- (+) Custom blob store over Raft KV: simple, uses existing infrastructure
- (-) Custom: KV not designed for large values, would need chunking, no streaming

## Consequences

- CI artifacts, Nix store paths, and file contents share the same blob infrastructure
- P2P transfer means nodes fetch blobs directly from the source — no central blob server
- BLAKE3 hashing is fast (parallelizable, SIMD-optimized) — hashing is not a bottleneck
- Deduplication is free — identical artifacts across pipeline runs consume storage once
- iroh-docs CRDT replication enables real-time sync for collaborative features
- Feature flags keep the binary small for deployments that don't need blob storage
