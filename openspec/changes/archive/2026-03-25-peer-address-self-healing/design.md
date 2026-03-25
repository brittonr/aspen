## Context

Iroh endpoints discover peers through pluggable `Discovery` services. The endpoint builder accepts multiple via `.add_discovery()`, and iroh queries them all when connecting to a peer. Aspen already configures three: mDNS (local network), DNS (n0's service), and Pkarr DHT. All three require either multicast, internet access, or external infrastructure.

After a rolling deploy with relay disabled, none of these work:

- mDNS: explicitly disabled via `--disable-mdns`
- DNS: requires internet + n0's DNS infrastructure
- Pkarr: requires internet + DHT nodes
- Relay: disabled via `--relay-mode disabled`

The Raft factory's `peer_addrs.json` exists but only feeds the factory's internal map — iroh itself never sees those addresses.

```
CURRENT: iroh tries to connect to peer
  ┌──────────┐     ┌─────────┐     ┌─────────┐
  │ mDNS     │     │ DNS     │     │ Pkarr   │
  │(disabled)│     │(no net) │     │(no net) │
  └──────────┘     └─────────┘     └─────────┘
       ✗                ✗               ✗
                    FAIL

AFTER: iroh tries to connect to peer
  ┌──────────┐     ┌─────────┐     ┌────────────────┐
  │ mDNS     │     │ DNS     │     │ ClusterDisc    │
  │(disabled)│     │(no net) │     │ (file-based)   │
  └──────────┘     └─────────┘     └────────────────┘
       ✗                ✗               ✓
                                   RESOLVES FROM DISK
```

## Goals / Non-Goals

**Goals:**

- Nodes recover peer addresses after restart using only local disk and iroh's existing `Discovery` machinery
- Works in air-gapped environments with relay and internet disabled
- Works alongside existing mDNS/DNS/Pkarr discovery (additive, not replacing)
- Zero new network protocols — all peer communication through iroh QUIC

**Non-Goals:**

- Replacing relay for NAT traversal across the internet
- Cross-machine discovery without shared filesystem (that's what mDNS/DNS/Pkarr are for)
- Changing Raft membership or consensus protocols

## Decisions

### 1. Implement `iroh::Discovery` trait, not a custom protocol

**Choice**: `ClusterDiscovery` implements `Discovery` with `publish()` and `resolve()`. Iroh calls these automatically — `publish` when own address changes, `resolve` when connecting to a peer.

**Why**: Iroh already has the machinery. `publish` is called on every address change. `resolve` is called before every connection attempt. We just need to persist and read addresses. No new sockets, no new protocols.

### 2. File-based persistence using the data directory

**`publish()`**: Writes own `NodeAddr` (relay URL + direct addresses) to `<data_dir>/discovery/<public_key_hex>.json`. Atomic write via write-to-temp + rename.

**`resolve()`**: When iroh needs to connect to a peer, reads `<data_dir>/discovery/<peer_public_key_hex>.json`. Returns addresses as a `DiscoveryItem`. Falls back to Raft membership addresses if file doesn't exist.

This works for same-machine clusters where all nodes share a parent directory (e.g., `/tmp/aspen-dogfood/node1/`, `node2/`, `node3/`). For cross-machine deployments, shared filesystems (NFS, etc.) work, or the other discovery mechanisms (mDNS, DNS, Pkarr) handle it.

### 3. Seed iroh's address book on startup

On bootstrap, iterate Raft membership and call `endpoint.add_node_addr(NodeAddr { node_id, direct_addresses, relay_url })` for each peer using addresses from:

1. The peer's discovery file (if readable)
2. The factory's `peer_addrs.json` cache (fallback)
3. The Raft membership addresses (last resort — likely stale but gives iroh something to try)

This pre-populates iroh's internal address book so connection attempts don't start from zero.

### 4. Cross-node discovery file resolution

For same-machine clusters, nodes can read each other's discovery files by computing the sibling path: `<data_dir>/../node<N>/discovery/<key>.json`. The `ClusterDiscovery` takes an optional `cluster_data_dir` parameter — the parent directory containing all node data dirs. When resolving, it scans sibling directories for matching public key files.

For cross-machine deployments, this falls back to mDNS/DNS/Pkarr. The `ClusterDiscovery` is additive — it doesn't replace the other discovery mechanisms, just supplements them for the air-gapped case.

### 5. Flush peer cache on clean shutdown

During `NodeResources::shutdown()`, before closing the iroh endpoint:

1. Call `ClusterDiscovery::publish()` one final time with current addresses
2. Flush the factory's `peer_addrs.json` with all known peer addresses

This ensures the freshest addresses are on disk for the next startup.

## Risks / Trade-offs

- **[Same-machine only for file-based]** → Cross-machine clusters without shared storage fall back to mDNS/DNS/Pkarr. File-based discovery is an optimization for the common dev/test case, not a universal solution.
- **[Stale files]** → If a node crashes without clean shutdown, its discovery file has stale addresses. Mitigation: the `resolve()` stream can yield multiple results — first the file-based address, then Raft membership addresses. Iroh tries them all.
- **[File permission issues]** → Nodes in different containers or user contexts can't read sibling directories. Mitigation: this is the same constraint as shared data directories; containerized deployments already use mDNS/DNS.
