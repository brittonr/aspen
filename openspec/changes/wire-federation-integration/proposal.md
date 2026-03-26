## Why

The federation primitives (identity, trust, discovery, gossip, sync protocol, resolver) are implemented and tested in isolation (~9,500 lines, 147 unit tests), but none of them run during normal node operation. The `FederationProtocolHandler` is never registered on the iroh Router, no federation services start at boot, and the NixOS 2-cluster test only checks that CLI commands don't crash — it never syncs content between clusters. Federation is dead code until we wire it into the node lifecycle.

## What Changes

- Add `federation()` method to `RouterBuilder` to register `FederationProtocolHandler` on the iroh ALPN router
- Add `federation_init.rs` bootstrap module in `aspen-cluster` that creates `ClusterIdentity`, `TrustManager`, `FederationGossipService`, and `FederationProtocolHandler` from `NodeConfig`
- Wire federation bootstrap into the node startup sequence (`bootstrap_node`)
- Implement `FederationResourceResolver` for Forge repositories backed by the cluster's KV/blob stores
- Extend the NixOS federation VM test to perform actual cross-cluster content sync: create a repo on cluster A, federate it, pull refs/objects from cluster B, verify content matches

## Capabilities

### New Capabilities

- `federation-bootstrap`: Node lifecycle integration — creating federation services at startup, registering the ALPN handler, and shutting down cleanly

### Modified Capabilities

- `federation`: Existing spec gains requirements for the resolver returning real data from cluster storage instead of stubs

## Impact

- `crates/aspen-cluster/src/router_builder.rs` — new `federation()` method
- `crates/aspen-cluster/src/bootstrap/node/` — new `federation_init.rs` module
- `crates/aspen-cluster/src/bootstrap/node/mod.rs` — call federation init during bootstrap
- `crates/aspen-cluster/src/config/` — federation config fields (cluster key, name, trusted clusters)
- `crates/aspen-forge/` or `crates/aspen-forge-handler/` — `FederationResourceResolver` impl for Forge repos
- `nix/tests/federation.nix` — expand test to exercise actual sync
- `src/node/mod.rs` — register federation handler in `spawn_router_with_blobs`
