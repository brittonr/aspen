## Context

The nix binary cache infrastructure has four independently-built pieces that have never been connected:

1. **Upload** (`cache.rs`): After `nix build`, NixBuildWorker runs `nix nar dump-path`, uploads the NAR blob to iroh-blobs, and registers a CacheEntry in KV via KvCacheIndex
2. **Index** (`aspen-cache`): CacheEntry stored at `_cache:{store_hash}` in Raft KV with narinfo metadata (store path, blob hash, nar hash, nar size, references)
3. **Gateway** (`aspen-nix-cache-gateway`): HTTP server that translates `GET /{hash}.narinfo` → KV lookup and `GET /nar/{blob_hash}.nar` → blob fetch
4. **Substituter** (`executor.rs`): Injects `--substituters http://gateway --trusted-public-keys "cache-name:pubkey"` into `nix build`

Current state in dogfood: `gateway_url: None`, `use_cluster_cache` depends on `config.nix_cache.is_enabled` which defaults to false. The upload path runs (cache_index is `Some(KvCacheIndex)`) but `gateway_url` being None means no substituter injection.

## Goals / Non-Goals

**Goals:**

- Prove the full cache round-trip in the serial dogfood VM: build → upload → gateway → substitute
- Measure cache hit rate and time saved on second build
- Surface and fix any bugs in narinfo generation, signing, or blob serving

**Non-Goals:**

- Snix decomposed storage (level B) — separate change
- Multi-node cache replication — blobs are already replicated via iroh-blobs
- Cache garbage collection or eviction policies
- Production-grade cache performance tuning

## Decisions

### 1. Gateway runs as a systemd service alongside aspen-node

The gateway is a separate binary (`aspen-nix-cache-gateway`) that connects to the cluster via ticket. In the serial dogfood VM, it runs as a separate systemd unit listening on `127.0.0.1:8380`.

**Alternative considered:** Embed gateway HTTP routes in aspen-node. Rejected — Aspen uses Iroh for all communication, HTTP is only for nix compatibility. Keeping it as a bridge binary (like `git-remote-aspen`) maintains that boundary.

### 2. Gateway URL passed to NixBuildWorkerConfig via node config

The node already constructs NixBuildWorkerConfig with `gateway_url: None`. When the gateway is running on the same host, set `gateway_url: Some("http://127.0.0.1:8380")`. This can come from:

- CLI flag: `--nix-cache-gateway-url http://127.0.0.1:8380`
- Environment: `ASPEN_NIX_CACHE_GATEWAY_URL`
- NixOS module option: `services.aspen.node.nixCacheGatewayUrl`

The NixOS module approach is cleanest for VM tests.

### 3. Signing key bootstrapped on node startup, not gateway startup

The gateway currently calls `ensure_signing_key()` which generates and stores the key if missing. This creates a race: the node starts uploading before the gateway has created the key. Move the `ensure_signing_key` call to node startup (under `#[cfg(feature = "nix-cache-gateway")]`) so the key exists before any CI builds run. The gateway just loads the existing key.

The node already has this partially: `load_nix_cache_signer` in `setup/client.rs` line 60. Need to verify it calls `ensure_signing_key` (create-if-missing) rather than just loading.

### 4. VM test structure: two sequential builds of the same flake

Build 1: `nix build` a simple flake (cowsay). All deps come from cache.nixos.org. After completion, NixBuildWorker uploads output paths to cluster cache.

Build 2: `nix build` the same flake again (or a dependent flake that shares deps). The substituter should hit the cluster cache for the output paths uploaded in build 1.

Verification: grep gateway logs for `200` responses on `.narinfo` and `/nar/` requests, confirming cache hits.

## Risks / Trade-offs

**[Risk] Gateway serves stale or malformed narinfo** → Validate narinfo output against nix's expected format. The `CacheEntry::to_narinfo()` method exists but has only been unit-tested, never served to a real nix client.

**[Risk] Blob hash mismatch between upload and serve** → The upload stores a BLAKE3 hash in CacheEntry.blob_hash. The gateway serves `/nar/{blob_hash}.nar`. If the hash format differs (hex vs base32, with/without prefix), the fetch will 404. Need to verify hash format consistency.

**[Risk] Signing key format incompatible with nix** → nix expects `cache-name:base64-ed25519-public-key` format for `--trusted-public-keys`. The `CacheSigningKey::to_nix_public_key()` method must produce exactly this format. Test with `nix store verify --trusted-public-keys`.

**[Risk] Large NARs exceed blob store limits** → `MAX_NAR_UPLOAD_SIZE` is already checked. Gateway has `MAX_NAR_STREAM_SIZE = 1GB`. For cowsay-level builds this is fine.

## Open Questions

1. Does `load_nix_cache_signer` create the key if missing, or does it only load? If load-only, need to add `ensure_signing_key` to node startup.
2. What's the actual `nix_cache` config structure in `NodeConfig`? Need to check `config.nix_cache.is_enabled` and related fields.
3. Should the gateway be optional in the dogfood script, or always started when `--enable-ci` is set?
