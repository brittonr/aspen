## Why

CI builds currently download all dependencies from cache.nixos.org every time, even when the same derivations were built minutes ago by the same cluster. The nix cache gateway, cache index, and upload infrastructure all exist but have never been wired together end-to-end in a live dogfood run. A first build should populate the cluster's binary cache, and subsequent builds should hit that cache as a substituter, cutting build times for shared dependencies.

## What Changes

- Wire the nix cache gateway into the serial dogfood VM so it runs alongside the aspen-node
- Configure NixBuildWorker to use `cache_index` and `use_cluster_cache = true` so build outputs are uploaded after each CI build
- Ensure the cache signing key is generated and stored in KV on first use
- Add `--substituters` and `--trusted-public-keys` to subsequent `nix build` invocations so they pull from the cluster cache before falling back to cache.nixos.org
- Create a VM test that proves the round-trip: build A populates cache → build B fetches from cache (cache hit visible in logs or narinfo GET)

## Capabilities

### New Capabilities

- `nix-cache-roundtrip`: End-to-end verification that CI builds populate and consume the cluster's nix binary cache

### Modified Capabilities

- `nix-substituter-config`: The substituter wiring currently sets `gateway_url: None` in dogfood — needs to be populated with the gateway's listen address
- `ci-cache-publish`: Upload path exists but has never been exercised with real builds — may surface bugs in narinfo generation, blob storage, or signing

## Impact

- `src/bin/aspen_node/setup/client.rs`: NixBuildWorkerConfig construction — `gateway_url` needs a real value when `use_cluster_cache` is true
- `nix/vms/dogfood-node.nix` or equivalent: NixOS module config for running the gateway alongside the node
- `crates/aspen-nix-cache-gateway/`: Gateway binary — tested for the first time with real traffic
- `crates/aspen-ci-executor-nix/src/cache.rs`: Upload path — exercised for the first time with real build outputs
- `crates/aspen-cache/src/narinfo.rs`: Narinfo generation — must produce valid narinfo that nix can parse
- `scripts/dogfood-local.sh`: May need gateway startup step
