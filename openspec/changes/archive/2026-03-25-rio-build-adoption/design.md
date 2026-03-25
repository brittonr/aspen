## Context

Aspen already has substantial snix integration: `aspen-snix` implements `BlobService`, `DirectoryService`, and `PathInfoService` backed by iroh-blobs and Raft KV. `aspen-castore` provides irpc transport. `aspen-snix-bridge` exposes gRPC for snix-store CLI tools. CI executors upload build outputs as decomposed NAR → blob + directory + pathinfo.

The gap: blobs are stored as monolithic objects. `BlobService::chunks()` returns an empty vec. No sub-blob deduplication exists. When a rebuild changes a few files in a large closure, the entire NAR is re-stored. rio-build solved this with FastCDC + BLAKE3 chunking. snix's own design documents describe BLAKE3 tree-hash verified streaming as the intended direction.

Separately, the snix integration has no resilience patterns. A degraded Raft leader or unreachable iroh-blobs backend causes unbounded retries and cascading failures. rio-build's circuit breaker pattern is a proven solution.

## Goals / Non-Goals

**Goals:**

- Sub-blob content-defined chunking with cross-store-path deduplication
- Generic circuit breaker usable across all Aspen components
- Tracey spec coverage annotations on existing snix code
- Fuzz coverage for snix-facing protocol parsers
- Backpressure on castore server under load
- Coupled timeout derivation pattern

**Non-Goals:**

- Replacing iroh-blobs as the blob backend (chunks are stored IN iroh-blobs)
- Implementing the Nix `ssh-ng://` wire protocol (nix-compat is available but not needed now)
- Kubernetes integration or any rio-build operational model
- DAG-aware build scheduling (Aspen's CI model is pipeline-based, not derivation-DAG-based)
- FUSE worker stores with overlayfs (Aspen already has `aspen-fuse` for a different purpose)
- Multi-tier worker pools or size-class routing

## Decisions

### D1: FastCDC chunking at the `BlobService` layer

**Decision:** Implement chunking as a wrapping `BlobService` that chunks on write and reassembles on read. Store chunk metadata (manifest) alongside the blob in Raft KV. Store individual chunks as separate iroh-blobs entries.

**Rationale:** This preserves the existing `IrohBlobService` as the underlying storage while adding dedup on top. snix's `BlobService` trait already has `chunks()` returning `ChunkMeta` — we fill in the real implementation rather than adding a separate storage layer.

**Alternatives considered:**

- *Use snix's own composable BlobService backends (redb, object store)*: Would replace iroh-blobs. Rejected because iroh-blobs gives us P2P transfer for free — chunks stored in iroh-blobs are automatically transferable across nodes via iroh's content-addressed protocol.
- *Chunk at the NAR level before calling BlobService*: Would require changes to every upload callsite. Rejected because chunking inside BlobService is transparent to callers.

**Parameters** (adopted from rio-build, tuned):

- Min chunk: 16 KiB, average: 64 KiB, max: 256 KiB
- Inline threshold: 256 KiB (blobs below this skip chunking)
- Hash: BLAKE3 per-chunk (same as snix's native hash)
- Manifest format: Vec of (blake3_hash, size) tuples, postcard-serialized, stored at KV key `snix:manifest:<blob_b3_digest>`

### D2: Circuit breaker in aspen-core

**Decision:** Port rio-build's `CacheCheckBreaker` pattern as a generic `CircuitBreaker` struct in `aspen-core`. Parameterize threshold, timeout, and metric name. No async, no I/O — pure state machine.

**Rationale:** rio-build's implementation is 80 lines, well-tested, and uses the right state machine (closed → open on threshold → half-open probe → close on success or timeout). Making it generic means every component can use the same pattern.

**API:**

```rust
pub struct CircuitBreaker {
    consecutive_failures: u32,
    open_until: Option<Instant>,
    threshold: u32,
    open_duration: Duration,
}

impl CircuitBreaker {
    pub fn new(threshold: u32, open_duration: Duration) -> Self;
    pub fn record_failure(&mut self) -> bool; // true = breaker is open, reject
    pub fn record_success(&mut self);
    pub fn is_open(&self) -> bool;
}
```

**Alternatives considered:**

- *Use an existing crate (failsafe, recloser)*: Adds a dependency for 80 lines of code. These crates also tend to be async-aware with timers, which conflicts with Aspen's verified/pure-function philosophy.

### D3: Tracey annotations on existing code

**Decision:** Add `r[...]` spec markers to the snix design docs, then annotate existing Rust code with `r[impl ...]` and tests with `r[verify ...]`. Add `tracey query validate` as a CI check.

**Rationale:** Aspen has 30 spec markers in docs but 0 code annotations. rio-build has 131 markers and 286 annotations with CI enforcement. The snix crate is the right place to start because it has Verus specs that map cleanly to tracey requirements.

### D4: Fuzz smoke tier

**Decision:** Add fuzz targets for NAR ingestion, protobuf Directory deserialization, and PathInfo encoding. Run 30-second smoke per target in `nix flake check`. 10-minute nightly tier for deeper coverage.

**Rationale:** rio-build fuzzes every protocol parser. Aspen's snix integration processes untrusted data from NAR archives and protobuf messages but has no fuzz coverage. The two-tier model from rio-build (smoke in CI, nightly for depth) is the right tradeoff.

### D5: Backpressure via queue-depth hysteresis

**Decision:** Add hysteresis-based backpressure to the `CastoreServer` irpc handler. When the internal message queue reaches 80% capacity, reject new requests with a backpressure signal. Resume accepting at 60%.

**Rationale:** Without this, N concurrent CI workers uploading build artifacts can overwhelm the castore server, which blocks on Raft proposals. The hysteresis prevents oscillation — once rejecting, keep rejecting until the queue drains meaningfully.

### D6: Derived timeouts

**Decision:** Replace independently-defined coupled timeouts in `aspen-constants` with derived constants: `TIMEOUT = INTERVAL × MAX_MISSED`. Apply to heartbeat, staleness, and any other coupled timeout pairs.

**Rationale:** rio-build uses this pattern (`HEARTBEAT_TIMEOUT_SECS = MAX_MISSED_HEARTBEATS * HEARTBEAT_INTERVAL_SECS`). Prevents the bug where someone changes one constant but forgets the other.

## Risks / Trade-offs

- **[Chunk manifest storage in Raft KV]** → Manifests for large blobs (4 GiB NAR / 64 KiB avg chunk = 65k entries × 36 bytes = ~2.3 MB) are large KV values. Mitigation: `MAX_VALUE_SIZE` is 1 MB; blobs over ~28k chunks need the manifest stored in iroh-blobs itself rather than KV. Set a manifest inline threshold.
- **[FastCDC dependency]** → Adds a new crate dependency. Mitigation: `fastcdc` is a well-maintained, pure-Rust crate with no transitive deps.
- **[Chunk dedup ratio unknown]** → Actual dedup savings depend on workload. Mitigation: Add a `dedup_ratio` metric (rio-build does this) to measure real-world benefit.
- **[Circuit breaker hides failures]** → An open breaker silently rejects requests. Mitigation: Emit a metric + warn log on state transitions (rio-build's pattern). Monitor `circuit_open_total` in dashboards.
- **[Tracey annotation overhead]** → Annotating existing code is tedious. Mitigation: Start with `aspen-snix` only (6 Verus invariants, ~30 functions). Expand to other crates incrementally.
