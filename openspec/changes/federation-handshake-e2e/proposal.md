## Why

The NixOS federation VM test (2 independent clusters) can't complete a cross-cluster QUIC handshake because there's no way to get a node's iroh public key from the CLI. `cluster status --json` returns `endpoint_id` as a Debug-formatted `EndpointAddr` string, and `cluster health --json` has no iroh identity field at all. Without the base32 iroh node ID, `federation sync --peer <id>` can't be invoked.

## What Changes

- Add `iroh_node_id` field to the `HealthResponse` so the CLI can return the node's iroh public key (base32 string).
- Update the NixOS federation test to extract Alice's iroh node ID from `cluster health --json` and pass it to Bob's `federation sync --peer` command.
- Verify the federation QUIC handshake completes successfully between the two VMs (identity exchange, trust negotiation).

## Capabilities

### New Capabilities

- `federation-node-id-exposure`: Expose a node's iroh public key via the health RPC so federation sync callers can address peers.

### Modified Capabilities

## Impact

- `crates/aspen-client-api/src/messages/cluster.rs` — add `iroh_node_id` field to `HealthResponse`
- `crates/aspen-core-essentials-handler/src/core.rs` — populate `iroh_node_id` from `EndpointProvider::peer_id()`
- `crates/aspen-cli/src/bin/aspen-cli/output.rs` — surface `iroh_node_id` in health JSON output
- `nix/tests/federation.nix` — fix cross-cluster sync subtest to use the new field
