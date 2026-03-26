## Why

The federation handler starts at boot and serves real Forge ref data, but no production code path performs a cross-cluster sync. The sync client (`connect_to_cluster`, `get_remote_resource_state`) exists and passes wire-level tests, but it's not exposed to users. There's no CLI command to pull refs from a remote cluster, and the NixOS 2-cluster test only checks that the handler starts — it never connects one cluster to the other.

## What Changes

- Add `federation sync <peer-node-id>` CLI command that connects to a remote cluster, performs a handshake, lists resources, and fetches resource state for all federated repos
- Add `FederationSyncPeer` client RPC request so the CLI can trigger a sync through the existing RPC path
- Expand the NixOS federation VM test to have cluster B connect to cluster A over iroh QUIC, perform a federation handshake, and read cluster A's Forge repo refs — proving end-to-end cross-cluster data flow

## Capabilities

### New Capabilities

- `federation-sync-cli`: CLI command and RPC request for triggering a one-shot federation sync pull from a remote cluster

### Modified Capabilities

- `federation-bootstrap`: Add requirement that the node's iroh endpoint is reachable by federated peers (already true, but the test now proves it)

## Impact

- `crates/aspen-client-api/src/messages/federation.rs` — new `FederationSyncPeer` request/response
- `crates/aspen-cli/src/bin/aspen-cli/commands/federation.rs` — new `Sync` subcommand
- `crates/aspen-forge-handler/src/handler/handlers/federation.rs` — handle the sync RPC
- `nix/tests/federation.nix` — actual cross-cluster handshake + GetResourceState
