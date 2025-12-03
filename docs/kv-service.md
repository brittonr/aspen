# KV Service Harness

Aspen's `KvServiceBuilder` converts configuration inputs into a running `KvNode`
and hands back a `KvService` wrapper that exposes the node metadata plus a ready
to use `KvClient`. This harness powers the `examples/kv_service.rs` binary and
serves as the recommended entry point for operators or integration tests that
need to spin up KV nodes quickly.

## Environment Variables

The example binary reads configuration from environment variables:

| Variable | Description | Default |
| --- | --- | --- |
| `ASPEN_NODE_ID` | Unsigned integer Raft node ID (required) | None |
| `ASPEN_DATA_DIR` | Filesystem directory for Raft log and state machine | Temp directory |
| `ASPEN_COOKIE` | Cluster authentication cookie (required for gossip) | "default-cookie" |
| `ASPEN_PEERS` | Manual peer list: `"node_id@endpoint_id,..."` | None (uses discovery) |
| `ASPEN_IROH_RELAY_URL` | Relay server URL for NAT traversal | None |
| `ASPEN_IROH_ENABLE_MDNS` | Enable mDNS local discovery | true |
| `ASPEN_IROH_ENABLE_DNS_DISCOVERY` | Enable DNS-based peer discovery | false |
| `ASPEN_IROH_DNS_DISCOVERY_URL` | Custom DNS discovery service URL | n0's public DNS |
| `ASPEN_IROH_ENABLE_PKARR` | Enable Pkarr DHT publishing | false |
| `ASPEN_IROH_PKARR_RELAY_URL` | Custom Pkarr relay URL | n0's public Pkarr |

### Discovery Configuration

By default, nodes use **mDNS + gossip** for zero-config local discovery:

```bash
# Default: mDNS + gossip (works on same LAN, separate machines)
ASPEN_NODE_ID=1 cargo run --example kv_service
ASPEN_NODE_ID=2 cargo run --example kv_service  # On another machine
```

For production deployments, enable DNS + Pkarr + relay:

```bash
ASPEN_NODE_ID=1 \
ASPEN_COOKIE="production-cluster" \
ASPEN_IROH_RELAY_URL=https://relay.example.com \
ASPEN_IROH_ENABLE_DNS_DISCOVERY=true \
ASPEN_IROH_ENABLE_PKARR=true \
cargo run --example kv_service
```

For single-host testing, use manual peers (mDNS doesn't work on localhost):

```bash
# First, get endpoint IDs from node logs
ASPEN_NODE_ID=1 cargo run --example kv_service
# Note the endpoint ID from logs (e.g., "abc123...")

ASPEN_NODE_ID=2 \
ASPEN_PEERS="1@abc123..." \
cargo run --example kv_service
```

Each peer listed in `ASPEN_PEERS` is registered immediately after the node
starts so Raft RPCs can flow without manual setup. The builder accepts the same
inputs programmatically (see `KvServiceBuilder::with_peer`/`with_peers`) to ease
test orchestration.

### Peer Specification Format

**Current format:** `"node_id@endpoint_id"`
- Example: `"2@abc123def456...,3@xyz789..."`
- The endpoint_id is the Iroh node's public key (hex encoded)

**Legacy format (deprecated):** `"id=endpoint_addr"`
- Example: `"2=default/ipv4/127.0.0.1/4002/quic"`
- No longer supported; use discovery or new format

## Notes for Scripts and Tests

1. The harness only starts a node; callers are responsible for invoking
   `raft.initialize` with the membership configuration before issuing writes.
2. When scripting multi-node clusters, ensure every node lists the others in
   `ASPEN_KV_PEERS` so bidirectional connections exist.
3. Integration tests should use deterministic data directories (e.g. via
   `tempfile::TempDir`) to avoid cross-test interference.
