## Why

The `aspen-dogfood` binary cannot establish QUIC connections between alice/bob nodes or from the client to the nodes when running locally. Without iroh relay servers (IPv6 unreachable) or mDNS (disabled), there is no peer discovery path. The cluster tickets contain the node's public key but no direct socket addresses, so the client has nothing to connect to.

This blocks end-to-end testing of the federation clone pipeline with large repos (34K+ git objects).

## What Changes

- Pass the node's direct socket address via the cluster ticket so clients can connect without relay or mDNS
- Ensure `aspen-dogfood` spawned nodes bind to `127.0.0.1` and include that address in their tickets
- Expand the NixOS VM federation-git-clone test to push a larger repo (100+ files, nested trees) to stress multi-batch federation sync
- Add a large-repo variant that exercises the convergent import loop across batch boundaries

## Capabilities

### New Capabilities

- `dogfood-local-connectivity`: Fix local peer-to-peer connectivity for dogfood binary — nodes include direct addresses in tickets, client connects without relay/mDNS

### Modified Capabilities

## Impact

- `crates/aspen-dogfood/src/node.rs` — node spawning, address extraction
- `crates/aspen-dogfood/src/cluster.rs` — health check, peer trust with direct addrs
- `crates/aspen-client/src/client.rs` — already has ASPEN_RELAY_DISABLED support
- `nix/tests/federation-git-clone.nix` — expand test with larger repo
