# Content-store adapter runtime

Aspen content identity remains owned by the existing chunk manifest and chunk primitives. The internal `molten-core::content_store_adapter` module supplies the pure bounded command/transition core; `molten::content_store_adapter` supplies canonical Preserves artifacts and capability-rooted local, Redb-indexed, live Iroh Blobs, optional object-storage, and deterministic simulation shells.

## Identity boundary

A canonical manifest ref, ordered chunk refs, BLAKE3 chunk identities, lengths, fixed chunker parameters, transforms, metadata ref, policy refs, root ref, and evidence remain independent of the selected backend. Backend blob hashes, Iroh tickets, endpoint/provider IDs, tags, object keys, and paths are locator or protection hints only. Status exposes a BLAKE3 hint ref, never the raw locator.

Repeated chunk refs are valid ordered manifest occurrences. Resume state therefore preserves the exact verified prefix and missing suffix rather than treating refs as a set.

## Ports and operations

The versioned `content-store.v1` durable-state port covers bounded put, get, verified range, availability, protect, unprotect, and status operations. The separate `content-exchange.v1` transport port covers bounded import, export, streaming get, and cancellation. Commands carry canonical refs and finite resource declarations, not files, Redb handles, Iroh connections, tickets, or simulator objects.

Profiles declare capabilities explicitly. Unsupported range, transform, durability, or protection behavior denies before I/O; a backend cannot silently emulate a stronger guarantee.

## Verification before exposure

Preflight checks total bytes, chunk count and size, range size, concurrency, queue and memory bytes, logical deadline, retry count, and event count. Local and live shells read one bounded chunk at a time. Each shell computes the existing domain-separated chunk identity and passes an observation into the pure transition core. Bytes enter a `VerifiedChunkPayload` only after hash, length, position, sequence, operation, manifest, profile, and generation checks pass. Full assembly additionally re-verifies every payload and is unavailable until the complete manifest is verified.

A backend success callback, QUIC connection, Iroh blob hash, or Redb row never marks an Aspen chunk available by itself.

## Partial state and failure semantics

Canonical partial state records the exact verified prefix, missing suffix, verified bytes, generation, event count, sequence, and terminal class. Capability-rooted persistence uses a bounded deterministic `MCPS001` record under a caller-supplied `Scope`; load revalidates profile, manifest, operation, partition, generation, refs, bytes, and event limits before resume.

Terminal classes distinguish accepted, streaming, verified, durable, cancelled, retryable, failed, uncertain, and denied. Corruption, truncation, reordering, unexpected chunks, stale tickets, unsupported transforms, root escape, local overload, provider throttle, permission denial, timeout, disconnect, and adapter failure remain distinct. Provider throttle is retryable and is not local capacity overload. A get disconnect after verified progress, and a put disconnect after the remote acknowledged any part of the put, are uncertain; an unacknowledged put disconnect remains retryable. Uncertain outcomes are never normalized to success or definite absence, and retrying them requires an explicit idempotency or no-clobber policy.

## Adapter profiles

- **Capability local** streams verified chunks through `ChunkStoreRoot` and uses existing canonical manifests and receipts.
- **Redb indexed** projects bounded availability and missing counts from the existing index without exposing the database handle.
- **Iroh Blobs** starts an actual `iroh_blobs::BlobsProtocol` router under an admitted persisted transport key and streams BAO leaves over Iroh with a hard per-chunk bound. The client re-verifies Molten chunk identity after transport.
- **Object storage** is the distinct optional `object-storage` profile, gated by `content-store-object-storage`. Its existing thin path-style SigV4 client and injectable local HTTP endpoint use the same bounded put/get command and transition core. The shell verifies local hits without remote requests and verifies remote chunk identity and length before caching or exposure. `execute_object_put` reports durable completion only after every provider acknowledgement; it does not equate that acknowledgement with measured physical durability. The client freezes a numeric endpoint and credentials at admission, disables proxies, redirects and automatic retries, and bounds connection/request time, outstanding requests, configuration and body bytes. Its separate capability-rooted local tier has byte/entry/recovery bounds and no automatic eviction: a full tier leaves new cold reads verified remote-only. The [qualification runbook](content-store-adapter/object-storage-qualification.md) covers the pinned endpoint, mock faults, canonical receipts, dependency allowlist, split latency samples and actual NVMe-device inputs. Source fixtures are not executed evidence; no speedup, NVMe, MTTR or production-durability claim follows from them.
- **Deterministic simulation** runs the same command and transition core for get and put. The get path models corruption, truncation, cancellation, disconnect, capacity, latency, restart, and resume. The put path models provider throttle, torn put with possible remote acceptance, and multipart abort; failure arms never return a durable terminal or verified payloads.

The older `chunk_store::publish_iroh_blobs` / `fetch_iroh_blobs` compatibility helpers copy through a capability-rooted local blob directory. They are retained for compatibility and are **not** evidence of live Iroh transport. New live claims require `publish_chunks` plus `execute_stream_get` receipts or test evidence.

## Protection and authority

Backend tags and protection handles are effects subordinate to canonical retention. Protection never grants retention authority; a pin never grants read/reveal authority; unprotect never grants deletion authority. Read requires a separate read-authority ref. Deletion requires canonical retention policy, no active canonical pin, and a separate deletion gate. Availability does not grant import, provenance, confidentiality, installation, execution, or deletion authority.

## Object-storage parity

The command shell retains corruption/truncation as failed verification, provider throttle as retryable, get disconnect before progress as retryable, get disconnect after progress as uncertain, and timeout as uncertain. Dispatched PUT disconnects conservatively remain uncertain because mutation cannot be excluded, even without a response. Simulated torn-put, provider-throttle and multipart-abort arms use the same pure classifier. Neither shell silently retries uncertain puts.

Deletion requires the canonical retention policy/no-pin/deletion-gate decision **and** passes the local protection veto before dispatch. Unprotect is not deletion admission. An independently requested protected-remote-presence audit bypasses the cache and fails if a provider lifecycle rule removed the remote copy, even when a valid local copy exists. The adapter cannot prevent an out-of-band provider lifecycle rule; operators must prohibit destructive rules for protected inventory and run that audit.

The earlier checked-in `evidence/` inventory contained no matching object-storage receipts and `runs/` was absent; that historical inventory does not invalidate later retained local executions or promote the earlier prose-only SeaweedFS/device claims.

The [parent execution record](../.cairn/archive/2026-11-15-complete-object-storage-bounds/tasks.md#parent-execution-2026-11-13) records 32 passing object-feature controls, passing actual local S3 command/cache roundtrips, and the actual NVMe-path capacity/reopen control. Canonical object-conformance and NVMe-path receipts were retained for those specific execution cohorts. Task-owned tmpfs is not their NVMe storage claim.

Hosted local-S3 acceptance is now recorded: [run 43810362842](https://github.com/brittonr/aspen/actions/runs/43810362842) at `fcbf60a3d` and [run 43813756730](https://github.com/brittonr/aspen/actions/runs/43813756730) at `f663d2f47` passed. The latter executed 47 tests, including the actual local endpoint command/cache arms, corruption and capacity controls, and protected remote loss with a valid local copy retained. [Retained provenance, test names and original canonical receipts](../.cairn/archive/2026-11-16-admit-anonymous-runtime-sources/evidence/hosted-ci-20261116/observations.json) bind the exact source, artifact identities and command digests; earlier failed runs remain separate.

The latter run's canonical conformance receipt reports these samples for the five-byte loopback fixture, not an NVMe or cloud-provider workload:

| Read path | Samples | p50 (microseconds) | p99 (microseconds) |
| --- | --- | --- | --- |
| Remote local-S3 endpoint | 128 | 1513 | 6627 |
| Verified local tier | 128 | 27 | 49 |

The retained simulated put-fault receipt binds every selected fault's canonical fixture-command ref, terminal, state ref and event refs. Thirteen mock/fault receipts are byte-identical across the two hosted jobs; the live conformance receipt retains each run's own observation. Logs and decoded text are diagnostic views, not replacement receipt identities. These runs do not establish cloud-provider behavior, physical power-loss durability, machine-loss recovery, sustained throughput, a speedup, the default development shell, or qualification of later source edits.
