# Content-store adapter runtime

Molten content identity remains owned by the existing chunk manifest and chunk primitives. `molten-core::content_store_adapter` adds a pure bounded command, preflight, range-plan, partial-state, verification-transition, failure, availability, and authority core. `molten::content_store_adapter` supplies canonical Preserves artifacts and capability-rooted local, Redb-indexed, live Iroh Blobs, and deterministic simulation shells.

## Identity boundary

A canonical manifest ref, ordered chunk refs, BLAKE3 chunk identities, lengths, fixed chunker parameters, transforms, metadata ref, policy refs, root ref, and evidence remain independent of the selected backend. Backend blob hashes, Iroh tickets, endpoint/provider IDs, tags, object keys, and paths are locator or protection hints only. Status exposes a BLAKE3 hint ref, never the raw locator.

Repeated chunk refs are valid ordered manifest occurrences. Resume state therefore preserves the exact verified prefix and missing suffix rather than treating refs as a set.

## Ports and operations

The versioned `content-store.v1` durable-state port covers bounded put, get, verified range, availability, protect, unprotect, and status operations. The separate `content-exchange.v1` transport port covers bounded import, export, streaming get, and cancellation. Commands carry canonical refs and finite resource declarations, not files, Redb handles, Iroh connections, tickets, or simulator objects.

Profiles declare capabilities explicitly. Unsupported range, transform, durability, or protection behavior denies before I/O; a backend cannot silently emulate a stronger guarantee.

## Verification before exposure

Preflight checks total bytes, chunk count and size, range size, concurrency, queue and memory bytes, logical deadline, retry count, and event count. Local and live shells read one bounded chunk at a time. Each shell computes the existing domain-separated chunk identity and passes an observation into the pure transition core. Bytes enter a `VerifiedChunkPayload` only after hash, length, position, sequence, operation, manifest, profile, and generation checks pass. Full assembly additionally re-verifies every payload and is unavailable until the complete manifest is verified.

A backend success callback, QUIC connection, Iroh blob hash, or Redb row never marks a Molten chunk available by itself.

## Partial state and failure semantics

Canonical partial state records the exact verified prefix, missing suffix, verified bytes, generation, event count, sequence, and terminal class. Capability-rooted persistence uses a bounded deterministic `MCPS001` record under a caller-supplied `NodeStateNamespace`; load revalidates profile, manifest, operation, partition, generation, refs, bytes, and event limits before resume.

Terminal classes distinguish accepted, streaming, verified, durable, cancelled, retryable, failed, uncertain, and denied. Corruption, truncation, reordering, unexpected chunks, stale tickets, unsupported transforms, root escape, overload, permission denial, timeout, disconnect, and adapter failure remain distinct. Disconnect or timeout after possible progress is uncertain, never normalized to success or definite absence.

## Adapter profiles

- **Capability local** streams verified chunks through `ChunkStoreRoot` and uses existing canonical manifests and receipts.
- **Redb indexed** projects bounded availability and missing counts from the existing index without exposing the database handle.
- **Iroh Blobs** starts an actual `iroh_blobs::BlobsProtocol` router under an admitted persisted transport key and streams BAO leaves over Iroh with a hard per-chunk bound. The client re-verifies Molten chunk identity after transport.
- **Deterministic simulation** runs the same command and transition core and models corruption, truncation, cancellation, disconnect, capacity, latency, restart, and resume.

The older `chunk_store::publish_iroh_blobs` / `fetch_iroh_blobs` compatibility helpers copy through a capability-rooted local blob directory. They are retained for compatibility and are **not** evidence of live Iroh transport. New live claims require `publish_live_iroh_chunks` plus `execute_live_iroh_stream_get` receipts or test evidence.

## Protection and authority

Backend tags and protection handles are effects subordinate to canonical retention. Protection never grants retention authority; a pin never grants read/reveal authority; unprotect never grants deletion authority. Read requires a separate read-authority ref. Deletion requires canonical retention policy, no active canonical pin, and a separate deletion gate. Availability does not grant import, provenance, confidentiality, installation, execution, or deletion authority.
