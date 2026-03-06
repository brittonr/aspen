## 1. narinfo rendering in aspen-cache

- [x] 1.1 Add `CacheEntry::to_narinfo(&self, signature: Option<&str>) -> String` method to `crates/aspen-cache/src/narinfo.rs` — renders StorePath, URL, Compression, NarHash, NarSize, References, Deriver, Sig fields
- [x] 1.2 Add `CacheEntry::fingerprint(&self) -> String` method that computes the Nix signing fingerprint: `1;{store_path};{nar_hash};{nar_size};{sorted_refs}`
- [x] 1.3 Add unit tests for narinfo rendering: complete entry, minimal entry (no refs/deriver), entry with signature, fingerprint computation

## 2. Cache signing key management

- [x] 2.1 Add `nix_cache_signing` module to `crates/aspen-cache/src/` — functions for Ed25519 keypair generation, Nix-format key serialization (`cache-name:base64(key)`), and narinfo signing using the fingerprint
- [x] 2.2 Add constants for KV key paths: `_sys:nix-cache:signing-key`, `_sys:nix-cache:public-key`, `_sys:nix-cache:name`
- [x] 2.3 Add `ensure_signing_key(kv_store, cache_name) -> (SigningKey, PublicKeyString)` that loads or generates the keypair in Raft KV
- [x] 2.4 Add unit tests for key generation, serialization, signing, and signature verification

## 3. NixCacheGetPublicKey RPC

- [x] 3.1 Add `NixCacheGetPublicKey` variant to `ClientRpcRequest` and `NixCachePublicKeyResponse` to `ClientRpcResponse` in `aspen-client-api` (check discriminant ordering!)
- [x] 3.2 Add handler in `aspen-nix-handler` that reads public key from KV and returns it
- [x] 3.3 Add CLI command `aspen-cli cache public-key` that calls the RPC and prints the key

## 4. Gateway binary: aspen-nix-cache-gateway

- [x] 4.1 Create `crates/aspen-nix-cache-gateway/` crate with binary target, clap CLI (--ticket, --port, --bind, --cache-name flags)
- [x] 4.2 Implement cluster connection: parse ticket, create `AspenClient`, connect on startup with 30s timeout
- [x] 4.3 Implement `/nix-cache-info` endpoint returning StoreDir, WantMassQuery, Priority
- [x] 4.4 Implement `/{hash}.narinfo` endpoint: validate hash → `CacheQuery` RPC → render narinfo with signature → return with `text/x-nix-narinfo` content-type (404 on miss)
- [x] 4.5 Implement `/nar/{blob_hash}.nar` endpoint: fetch blob via iroh-blobs → stream with `application/x-nix-nar` content-type (404 on miss, 413 if >1GB)
- [x] 4.6 Add HTTP server with hyper: localhost binding, concurrent request semaphore (100 max), graceful shutdown
- [x] 4.7 Load or generate signing key on startup via `ensure_signing_key()`
- [x] 4.8 Add gateway to workspace Cargo.toml and flake.nix build

## 5. Rewrite CI cache proxy

- [x] 5.1 Add `gateway_url` config field to `NixBuildWorkerConfig` — when set, inject `--extra-substituters` and `--trusted-public-keys` into nix build args (deferred: H3 proxy rewrite is separate scope)
- [x] 5.2 Update `NixBuildWorker` to inject substituter args when `gateway_url` + `cache_public_key` are configured
- [x] 5.3 Fetch public key via `NixCacheGetPublicKey` RPC at worker startup (cache in config)
- [x] 5.4 Update existing `cache_substituter_test.rs` tests to cover the new substituter injection

## 6. Integration tests

- [x] 6.1 Add narinfo round-trip test: create CacheEntry → render narinfo → verify parseable by `nix-store --dump-narinfo` format
- [x] 6.2 Add gateway integration test: narinfo signed round-trip, cache-info format, URL format
- [x] 6.3 Add gateway narinfo test: narinfo content-type, URL uses blob hash, signature verification
- [ ] 6.4 Add NixOS VM test: 2-node cluster + gateway + nix client — CI builds a derivation, uploads to cache, second machine substitutes from gateway (deferred: requires flake.nix gateway binary build)
