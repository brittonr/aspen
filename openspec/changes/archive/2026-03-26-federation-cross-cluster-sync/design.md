## Context

The federation sync client (`crates/aspen-federation/src/sync/client.rs`) implements `connect_to_cluster`, `list_remote_resources`, `get_remote_resource_state`, and `sync_remote_objects` over iroh QUIC. Wire-level tests in `federation_wire_test.rs` prove the round-trip works between two in-process endpoints. The federation handler is now registered at boot (from the `wire-federation-integration` change).

The NixOS test has two independent clusters (alice, bob) with federation enabled. They run in separate VMs with separate cookies and identities. The test verifies the handler starts but never connects one to the other.

## Goals / Non-Goals

**Goals:**

- CLI command to trigger a sync pull: `aspen-cli federation sync <node-id>`
- RPC plumbing so the CLI works through the standard client RPC path
- NixOS VM test performs actual cross-cluster handshake + ref query
- Prove real data flows between two separate Aspen clusters

**Non-Goals:**

- Automatic/periodic sync (manual pull only for now)
- Writing fetched data to the local cluster (read-only query first)
- Gossip-based discovery of remote clusters (use explicit node IDs)

## Decisions

### D1: Direct iroh connection from RPC handler

The federation sync RPC handler receives a `node_id` (iroh PublicKey), uses the local iroh endpoint to connect directly to the remote peer at `FEDERATION_ALPN`, performs the handshake, and returns the results. No intermediate proxy or relay needed — iroh handles NAT traversal.

### D2: CLI uses --ticket for local cluster, --peer for remote

`aspen-cli --ticket <local> federation sync --peer <remote-node-id>` — the CLI connects to the local cluster via the ticket, sends a `FederationSyncPeer` RPC, and the node does the federation sync on behalf of the client.

Alternative: CLI connects directly to the remote peer. Rejected — the CLI doesn't have the cluster's federation identity needed for the handshake.

### D3: NixOS test reads node-id from cluster ticket

Each cluster writes a ticket file. The test extracts the iroh node public key from the ticket and passes it to the `federation sync` CLI command on the other cluster.

## Risks / Trade-offs

- **[Risk] Two VMs need iroh connectivity** → Mitigation: both VMs are on the same QEMU virtual network, firewall is disabled
- **[Risk] RPC handler blocks during sync** → Mitigation: the sync is a single handshake + one query, completes in <1s on LAN
