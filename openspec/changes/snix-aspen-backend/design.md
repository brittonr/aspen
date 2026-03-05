## Context

Aspen already depends on snix crates (`snix-castore`, `snix-store`, `nix-compat`) in `aspen-ci-executor-nix`. The CI executor has an `upload_store_paths_snix()` method that calls `ingest_nar_and_hash()` to decompose NARs into snix's content-addressed format. However, the snix service instances used there must be constructed externally — there's no Aspen-backed implementation.

snix defines three composable service traits:

- **`BlobService`** — content-addressed blob storage (BLAKE3 digests, chunking)
- **`DirectoryService`** — directory tree nodes (protobuf-encoded, BLAKE3-keyed)
- **`PathInfoService`** — Nix store path metadata (output hash → root node + NAR info + references)

Each trait has existing backends: `memory`, `redb`, `objectstore`, `grpc`, `bigtable`. We add an `aspen` backend for each.

## Goals / Non-Goals

**Goals:**

- Implement all three snix service traits backed by Aspen's distributed storage
- BlobService delegates to iroh-blobs (already content-addressed with BLAKE3)
- DirectoryService and PathInfoService use Aspen Raft KV with protobuf serialization
- Register backends with snix's composition registry so `snix-store` can use URL schemes like `aspen+iroh://` or `aspen+raft://`
- Enable CI builds to store artifacts in the distributed cluster (not local redb/memory)
- Enable `snix-store virtiofs` to serve /nix/store from Aspen's distributed backends

**Non-Goals:**

- Nix daemon protocol compatibility (no `nix-daemon` integration)
- HTTP binary cache serving (no `nar-bridge`)
- Replacing Aspen's own blob store with snix's — these coexist
- NAR chunking strategy changes — use snix's defaults
- Signing store paths — CI builds are unsigned for now

## Decisions

### 1. Single crate `aspen-snix-backend`

All three trait implementations in one crate. They share the same Aspen client connection and KV prefix conventions. Splitting into three crates adds dependency noise for no benefit.

**Alternative considered:** Three separate crates (aspen-snix-blob, aspen-snix-dir, aspen-snix-pathinfo). Rejected — they'd all depend on aspen-client-api and share the same connection setup.

### 2. BlobService → iroh-blobs (not Raft KV)

Blobs are potentially large (NARs can be GBs). Storing them in Raft KV would be wrong — Raft replicates every write to every node. iroh-blobs is already content-addressed with BLAKE3, which is what snix uses. The mapping is natural:

- `has(digest)` → check if iroh-blobs has the BLAKE3 hash
- `open_read(digest)` → stream from iroh-blobs
- `open_write()` → write to iroh-blobs, return BLAKE3 digest on close
- `chunks()` → return None (iroh-blobs handles its own chunking)

snix uses B3Digest (32 bytes, BLAKE3). iroh-blobs uses BLAKE3 natively. Direct mapping.

**Alternative considered:** Store blobs in Raft KV with chunking. Rejected — Raft KV has MAX_VALUE_SIZE=1MB, and replicating multi-GB NARs through consensus is wrong.

### 3. DirectoryService → Raft KV with protobuf encoding

Directory nodes are small (protobuf-encoded directory listings, typically <10KB). They're keyed by BLAKE3 digest of their content. Perfect for KV:

- Key: `snix:dir:<blake3-hex>`
- Value: protobuf-encoded `snix_castore::proto::Directory`
- `get(digest)` → KV read
- `put(directory)` → compute BLAKE3 of protobuf bytes, KV write, return digest

This matches what the existing `snix-store.nix` test already does with manual KV writes.

### 4. PathInfoService → Raft KV with protobuf encoding

PathInfo entries map Nix output hashes (20 bytes) to metadata. Small, finite, naturally KV:

- Key: `snix:pathinfo:<output-hash-hex>`
- Value: protobuf-encoded `snix_store::proto::PathInfo`
- `get(digest)` → KV read + decode
- `put(path_info)` → encode + KV write
- `list()` → KV scan with `snix:pathinfo:` prefix

### 5. Client connection via Aspen ticket

The backend crate takes an Aspen cluster ticket (or client handle) at construction time. All operations go through the existing Iroh-based client RPC:

```rust
pub struct AspenBlobService {
    client: AspenClient,  // iroh-blobs operations
}

pub struct AspenDirectoryService {
    client: AspenClient,  // KV operations
}

pub struct AspenPathInfoService {
    client: AspenClient,  // KV operations
}
```

### 6. snix composition registry integration

Register with URL schemes so `snix-store` can use:

```
--blob-service-addr aspen+ticket://<ticket>
--directory-service-addr aspen+ticket://<ticket>
--path-info-service-addr aspen+ticket://<ticket>
```

This plugs into snix's `from_addr()` pattern. The ticket encodes the cluster connection info.

**Alternative considered:** Use gRPC proxy (run a gRPC server that translates to Aspen calls). Rejected — adds a whole extra service process and latency hop. Direct Iroh RPC is simpler.

## Risks / Trade-offs

- **[Risk] iroh-blobs BLAKE3 ≠ snix BLAKE3 digest format** → Both use 32-byte BLAKE3. Verify the digest types are byte-compatible at the boundary. Mitigation: unit test that round-trips a known blob through both systems.

- **[Risk] Raft KV latency for directory lookups** → Directory gets are on the hot path when serving VirtioFS. Each file open may trigger multiple directory lookups. Mitigation: snix already has LRU caching (`lru` PathInfoService wrapper). Use `CombinedDirectoryService` with a memory cache in front of the Raft backend.

- **[Risk] Large closure imports flood Raft** → Importing a full NixOS closure can produce thousands of directory entries and hundreds of PathInfo entries. Mitigation: batch KV writes where possible. The snix ingest pipeline already processes entries in order, so we can buffer and batch.

- **[Trade-off] No chunking metadata in BlobService** → We return `chunks() → None` because iroh-blobs manages its own transfer chunking. This means snix's chunk-level deduplication won't work across blobs. Acceptable — iroh-blobs has its own dedup at the transfer layer.

- **[Trade-off] PathInfo list() scans entire prefix** → KV scan with prefix is O(n) in stored paths. For large stores this could be slow. Acceptable for now — list() is only used for GC and diagnostics, not hot path.
