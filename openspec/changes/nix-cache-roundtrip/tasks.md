## 1. Node Config: Add gateway_url to NixCacheConfig

- [x] 1.1 Add `gateway_url: Option<String>` field to `NixCacheConfig` in `crates/aspen-cluster/src/config/nix_cache.rs` with serde default None
- [x] 1.2 Wire `config.nix_cache.gateway_url` into `NixBuildWorkerConfig` construction in `src/bin/aspen_node/setup/client.rs` (replace the hardcoded `gateway_url: None`)
- [x] 1.3 Add env var support: `ASPEN_NIX_CACHE_GATEWAY_URL` in `from_env_nix_cache()` in `crates/aspen-cluster/src/config/mod.rs`
- [x] 1.4 Add CLI flag `--nix-cache-gateway-url` to args.rs and wire into config merge

## 2. Signing Key Bootstrap

- [x] 2.1 Investigate current `load_nix_cache_signer` — it uses Transit secrets backend and requires `cache_name` + `signing_key_name` config. Determine if a simpler `ensure_signing_key` (from `aspen-cache::signing`) path works without Transit for the dogfood VM
- [x] 2.2 If Transit-based signer is required: wire up NixCacheConfig with `cache_name` and `signing_key_name` defaults that work out of the box when `nix_cache.is_enabled = true`
- [x] 2.3 If simpler `ensure_signing_key` path works: ensure the public key is written to `_system:nix-cache:public-key` in KV so `read_nix_cache_public_key()` finds it for NixBuildWorkerConfig
- [x] 2.4 Verify the signing key is created before worker registration (order in `setup_client_protocol`)

## 3. Gateway in Serial Dogfood VM

- [x] 3.1 Add `aspen-nix-cache-gateway` binary to the serial dogfood VM NixOS config (in `nix/vms/dogfood-node.nix` or equivalent)
- [x] 3.2 Create a systemd service unit for the gateway: starts after aspen-node, reads cluster ticket, listens on 127.0.0.1:8380
- [x] 3.3 Add `nix_cache.is_enabled = true` and `nix_cache.gateway_url = "http://127.0.0.1:8380"` to the node config in the dogfood VM
- [ ] 3.4 Test gateway starts and responds to `curl http://127.0.0.1:8380/nix-cache-info`

## 4. Verify Upload Path

- [ ] 4.1 Run a CI build in the serial dogfood VM with cache upload enabled (cache_index is already Some(KvCacheIndex))
- [ ] 4.2 After build completes, verify CacheEntries exist in KV: `aspen-cli kv scan _cache:` should show entries
- [ ] 4.3 Verify NAR blobs exist: the blob_hash from CacheEntry should resolve via `aspen-cli blob has {hash}`
- [ ] 4.4 Verify gateway serves narinfo: `curl http://127.0.0.1:8380/{store_hash}.narinfo` returns valid narinfo with Sig field

## 5. Verify Substituter Path

- [ ] 5.1 Trigger a second CI build of the same flake (or one sharing outputs)
- [ ] 5.2 Confirm `nix build` receives `--extra-substituters http://127.0.0.1:8380` and `--trusted-public-keys` in its command line (visible in build logs)
- [ ] 5.3 Check gateway logs for HTTP 200 responses on `.narinfo` and `/nar/` requests during the second build
- [ ] 5.4 Measure time difference between first build (cache cold) and second build (cache warm)

## 6. Fix Issues Found During Integration

- [ ] 6.1 Fix any narinfo format issues (field ordering, URL format for NAR download path, hash encoding mismatches)
- [ ] 6.2 Fix any signing issues (key format, fingerprint computation, nix trusted-public-keys format)
- [ ] 6.3 Fix any blob fetch issues (hash format mismatch between CacheEntry.blob_hash and gateway's blob RPC)
- [x] 6.4 Update napkin with findings

## 7. Dogfood Script Integration

- [x] 7.1 Add gateway startup to `scripts/dogfood-local.sh` when `--enable-ci` is passed
- [x] 7.2 Add cache verification step after first successful CI build (check narinfo endpoint)
- [x] 7.3 Document the cache round-trip in the napkin Domain Notes section
