## Context

Aspen's Nix binary cache has a complete **upload** path: CI builds create NARs, upload them to iroh-blobs, and register metadata in the Raft KV cache index. But the **download** path — letting `nix build` pull cached store paths — doesn't work.

The existing `CacheProxy` in `aspen-ci-executor-shell/src/cache_proxy.rs` was designed to forward HTTP requests to a gateway over H3/iroh QUIC, but it can't compile because `h3-iroh` requires iroh 0.96+ while aspen uses 0.95.1. The gateway service itself was never implemented.

CI executor config already has `use_cluster_cache`, `gateway_node`, `cache_public_key`, `iroh_endpoint` fields and a `can_use_cache_proxy()` method — all scaffolded but non-functional.

## Goals / Non-Goals

**Goals:**

- `nix build --substituters http://localhost:PORT` transparently pulls cached store paths from aspen's cluster
- Cache signing so Nix trusts substituted paths
- CI builds automatically configure the substituter when a cache is available
- Standalone gateway binary that any machine can run (not just CI workers)

**Non-Goals:**

- Upgrading iroh to 0.96+ (unrelated, large scope)
- HTTP/3 support (the H3 approach is deferred until iroh upgrades)
- Public-facing cache (gateway binds localhost only; external access uses iroh QUIC)
- Compression (xz/zstd) of NARs — serve uncompressed for now, add compression later
- Cache GC / eviction policies

## Decisions

### 1. Direct RPC gateway instead of H3 forwarding

**Decision**: Create `aspen-nix-cache-gateway` as a standalone binary that translates HTTP → aspen client RPC (iroh QUIC), bypassing H3 entirely.

**Alternatives considered:**

- **H3 forwarding** (original `CacheProxy` approach): Blocked on iroh 0.96+. Would add complexity (two hops: HTTP→H3→RPC) for no benefit.
- **snix daemon as substituter** (`nix --store daemon?socket=...`): Nix's `--substituters` only accepts HTTP(S)/S3/SSH URIs, not daemon sockets. Would require `nix copy --from` instead of transparent substitution.
- **Rewrite existing `CacheProxy`**: The proxy lives in `aspen-ci-executor-shell` (CI-specific crate). A standalone binary is more useful — developers, not just CI, need cache access.

**Rationale**: The gateway connects to the cluster as a regular `AspenClient`, which already works over iroh QUIC. No new transport dependencies. Same pattern as `git-remote-aspen` and `aspen-snix-bridge` — ecosystem protocol bridges.

### 2. narinfo rendering in `aspen-cache` crate

**Decision**: Add `CacheEntry::to_narinfo(&self, signature: Option<&str>) -> String` to the existing `aspen-cache` crate.

**Rationale**: The narinfo text format is tightly coupled to `CacheEntry` fields. Keeping rendering next to the data type avoids duplication and makes it testable without the gateway binary.

### 3. Ed25519 cache signing key stored in Raft KV

**Decision**: Store the cache signing keypair at well-known KV keys (`_sys:nix-cache:signing-key`, `_sys:nix-cache:public-key`). Generate on first gateway startup if absent. Use `ed25519-dalek` (already in workspace via `aspen-crypto`).

**Alternatives considered:**

- **Transit secrets backend**: Overkill — cache signing key doesn't need rotation or multi-party access. It's a cluster-wide singleton.
- **Config file**: Fragile, requires manual distribution. Raft KV gives automatic replication.
- **age encryption**: The signing key doesn't need encryption at rest in the KV (it's only for cache trust, not secrets). Nix binary cache keys are typically unencrypted.

**Rationale**: Nix narinfo signatures use `cache-name:base64(ed25519_sign(fingerprint))` where fingerprint = `1;{store_path};{nar_hash};{nar_size};{sorted_references}`. The key format matches `nix-store --generate-binary-cache-key`.

### 4. Gateway as standalone binary, not embedded in aspen-node

**Decision**: `aspen-nix-cache-gateway` is a separate binary (like `aspen-snix-bridge`, `aspen-cli`, `aspen-tui`).

**Rationale**:

- Runs where Nix runs (build machines, developer laptops) — not necessarily on cluster nodes
- No feature flag complexity in `aspen-node`
- Simple lifecycle: start with `--ticket`, runs until stopped

### 5. Rewrite `CacheProxy` to use direct `AspenClient`

**Decision**: Replace the H3-based `CacheProxy` in `aspen-ci-executor-shell` with one that uses `AspenClient` directly. Remove `h3-iroh` dependency.

**Rationale**: The CI executor needs an in-process cache proxy (auto-started for `nix build` commands). The standalone gateway serves other use cases. Both share `aspen-cache` for narinfo rendering but have different transports: in-process RPC vs CLI-started HTTP.

## Risks / Trade-offs

**[Risk] NAR download latency**: Each NAR download requires: HTTP request → RPC to cluster → blob fetch via iroh → stream back. Multiple network hops. → **Mitigation**: Iroh QUIC is already fast for blob transfer. Gateway can pipeline blob fetches. For large closures, the bottleneck is bandwidth not latency.

**[Risk] Gateway is single point of failure for builds**: If gateway crashes, `nix build` falls back to other substituters (cache.nixos.org). → **Mitigation**: Nix handles substituter failures gracefully. Configure aspen cache as *extra* substituter, not replacement.

**[Risk] Signing key compromise**: Anyone with KV read access can read the signing key. → **Mitigation**: Cache signing only prevents accidental corruption, not adversarial attacks. Nix's threat model assumes trusted substituters. For production, encrypt the key with Transit (future work).

**[Risk] Large NAR streaming**: NARs can be gigabytes. Buffering in gateway memory is not viable. → **Mitigation**: Stream directly from iroh-blobs to HTTP response body using chunked transfer encoding. Set `MAX_NAR_STREAM_SIZE` bound (1 GB, matching `MAX_NAR_UPLOAD_SIZE`).
