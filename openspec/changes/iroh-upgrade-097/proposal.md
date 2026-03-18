## Why

Aspen is pinned to iroh 0.95.1, iroh-blobs 0.97, and iroh-docs 0.95. The current versions are iroh 0.97.0, iroh-blobs 0.99.0, and iroh-docs 0.97.0. This blocks h3-iroh (HTTP/3 over QUIC via iroh endpoints), which requires iroh 0.96+. h3-iroh is needed for serving the Forge web frontend and nix binary cache directly through the iroh endpoint — aligned with aspen's "all communication via iroh" architecture. Two minor versions of upstream improvements (multipath, latency-based path selection, holepunch improvements, endpoint hooks, custom transports) are also blocked.

## What Changes

- **BREAKING**: Upgrade `iroh` from 0.95.1 to 0.97.0
- **BREAKING**: Upgrade `iroh-blobs` from 0.97 to 0.99.0
- **BREAKING**: Upgrade `iroh-docs` from 0.95 to 0.97.0
- Un-comment and enable `h3-iroh` dependency (currently disabled across 4 crates due to iroh 0.96 requirement)
- Update `nix-cache-gateway` to serve via h3-iroh instead of axum TCP
- Migrate all API breakage across 49 crates (134 files, 290 import sites):
  - `Discovery` trait → `AddressLookup` trait rename
  - `Endpoint::add_node_addr` removed (use `AddressLookup` providers)
  - `ServerConfig` / `TransportConfig` wrapped in newtypes
  - `Endpoint::latency` removed (use connection-level stats)
  - Quinn replaced with noq (iroh's QUIC fork)
  - Error types restructured
  - Tickets moved to `iroh-tickets` crate
  - `iroh-base` key API changes (reduced external types)
  - `EndpointAddr` now transport-generic

## Capabilities

### New Capabilities
- `h3-iroh-serving`: HTTP/3 serving through iroh QUIC endpoints via ALPN routing. Replaces TCP-bound axum listeners with h3-iroh for the nix cache gateway. Foundation for Forge web frontend.

### Modified Capabilities

## Impact

- 49 crates with iroh dependencies need version bumps in Cargo.toml
- 134 source files with `use iroh::` imports need API migration
- `crates/aspen-cluster/` — endpoint management, gossip, bootstrap (heaviest iroh usage)
- `crates/aspen-transport/` — ALPN routing, connection handling
- `crates/aspen-raft-network/` — raft RPC over iroh connections
- `crates/aspen-forge/` — gossip-based repo sync
- `crates/aspen-nix-cache-gateway/` — switch from axum TCP to h3-iroh
- `crates/aspen-ci-executor-shell/` — nix cache proxy (h3-iroh feature)
- `flake.nix` — remove h3-iroh version incompatibility comments, update crate-hashes
- `Cargo.lock` — full regeneration after version bumps
- NixOS VM tests — verify cluster formation, blob transfer, gossip still work post-upgrade
