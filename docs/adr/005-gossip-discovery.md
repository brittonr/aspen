# ADR-005: Gossip-Based Peer Discovery

**Status:** Accepted

**Date:** 2025-12-03

**Context:** Aspen nodes need to discover each other and exchange Raft endpoint information without manual configuration. The discovery mechanism must work across various network topologies (LAN, WAN, cloud) while maintaining security and performance.

## Decision

Implement **gossip-based peer discovery** using iroh-gossip, broadcasting node IDs and endpoint addresses on a cluster-wide topic every 10 seconds. Automatically connect discovered peers to the Raft network factory.

## Implementation

The implementation is in `src/cluster/gossip_discovery.rs`:

```rust
pub struct GossipPeerDiscovery {
    topic_id: TopicId,
    node_id: NodeId,
    shutdown: Arc<AtomicBool>,
    announcer_task: JoinHandle<()>,
    receiver_task: JoinHandle<()>,
}

impl GossipPeerDiscovery {
    /// Announcement interval in seconds.
    const ANNOUNCE_INTERVAL_SECS: u64 = 10;  // Tiger Style: Fixed interval
}
```

### Architecture

Each node runs two background tasks:

1. **Announcer Task**: Broadcasts `PeerAnnouncement` every 10 seconds

   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize)]
   struct PeerAnnouncement {
       node_id: NodeId,              // Raft node ID (u64)
       endpoint_addr: EndpointAddr,  // Iroh endpoint (public key + relay URLs)
       timestamp_micros: u64,        // Freshness tracking
   }
   ```

2. **Receiver Task**: Listens for peer announcements and automatically adds them to the Raft network factory:

   ```rust
   if let Some(ref factory) = receiver_network_factory {
       factory.add_peer(
           announcement.node_id,
           announcement.endpoint_addr.clone(),
       );
   }
   ```

### Topic Derivation

The gossip topic ID is derived from the cluster cookie using BLAKE3:

```rust
// In ClusterBootstrapConfig::derive_gossip_topic_id()
let hash = blake3::hash(cluster_cookie.as_bytes());
TopicId::from_bytes(*hash.as_bytes())
```

This ensures:

- Nodes with the same cookie join the same gossip swarm
- Different clusters remain isolated (no cross-talk)
- No need to manually configure topic IDs

### Security

Messages are cryptographically signed by the sender's Iroh secret key and verified on receipt:

```rust
// Iroh gossip handles signature verification automatically
match PeerAnnouncement::from_bytes(&msg.content) {
    Ok(announcement) => { /* process */ }
    Err(e) => {
        tracing::warn!("failed to parse peer announcement: {}", e);
        // Tiger Style: Fail fast on invalid messages
    }
}
```

Invalid signatures are rejected immediately (fail-fast principle).

### Auto-Peer Connection

When a peer announcement is received:

1. Parse the `PeerAnnouncement` message
2. Filter out self-announcements (avoid connecting to own node)
3. Add peer to `IrpcRaftNetworkFactory`:

   ```rust
   factory.add_peer(announcement.node_id, announcement.endpoint_addr);
   ```

4. Iroh establishes a QUIC connection on-demand when Raft needs to communicate

This enables **zero-configuration peer connectivity**: nodes discover each other and connect automatically without manual peer lists.

### Integration with Iroh Discovery Services

Gossip works in conjunction with Iroh's discovery mechanisms:

| Discovery Method | Purpose | Latency | Scope |
|-----------------|---------|---------|-------|
| **mDNS** | Find peers on local network | ~5 sec | LAN |
| **DNS** | Query centralized registry | ~5 sec | Global |
| **Pkarr** | DHT-based discovery | ~60 sec | Global |
| **Gossip** | Announce Raft metadata | ~10 sec | Cluster |

Workflow:

1. Node boots, Iroh establishes endpoint (mDNS/DNS/Pkarr find initial peers)
2. Node subscribes to cluster gossip topic
3. Node broadcasts its Raft node ID + endpoint address every 10 seconds
4. Other nodes receive announcement and add to Raft network factory
5. Raft operations can now communicate with discovered peer

### Tiger Style Alignment

From `tigerstyle.md`:

- **Fixed interval**: 10-second announcement period prevents unbounded message rate
- **Explicit types**: `NodeId` is `u64`, `timestamp_micros` is `u64` (not `usize`)
- **Bounded shutdown**: 10-second timeout on graceful shutdown
- **Fail fast**: Invalid announcements rejected immediately
- **Minimal scope**: Announcements filtered at receiver to ignore self

Example from implementation:

```rust
// Tiger Style: Fixed limit to prevent unbounded announcement rate
const ANNOUNCE_INTERVAL_SECS: u64 = 10;

// Tiger Style: Bounded wait time (10 seconds max)
pub async fn shutdown(self) -> Result<()> {
    let timeout = Duration::from_secs(10);
    tokio::select! {
        _ = self.announcer_task => { /* ... */ }
        _ = tokio::time::sleep(timeout) => {
            tracing::warn!("announcer task did not complete within timeout");
        }
    }
}
```

## Rationale

### Why Gossip?

1. **Zero-configuration**: Nodes discover each other without operator intervention
2. **Resilient**: No single point of failure (unlike centralized registries)
3. **Dynamic membership**: Nodes can join/leave without reconfiguration
4. **Iroh integration**: Leverages existing iroh-gossip infrastructure
5. **Cross-topology**: Works across LAN, WAN, and cloud environments

### Why 10-Second Interval?

Based on "napkin math" from Tiger Style:

- **Network overhead**: 10-node cluster = 10 announcements/sec = ~5 KB/sec (negligible)
- **Discovery latency**: New nodes discovered within 10 seconds (acceptable for Raft cluster formation)
- **CPU overhead**: Minimal (single broadcast + multiple receives per interval)
- **Chattiness**: Low enough to avoid noise, high enough for prompt discovery

Trade-offs:

- Shorter interval (e.g., 1 sec) → faster discovery but more network traffic
- Longer interval (e.g., 60 sec) → less overhead but slower discovery

10 seconds balances these concerns for typical cluster sizes (3-10 nodes).

## Alternatives Considered

### 1. Manual Peer Configuration Only

**Pros:**

- Simple and explicit
- No discovery overhead
- Predictable behavior

**Cons:**

- Operational burden (managing peer lists)
- Manual reconfiguration on topology changes
- Doesn't scale to dynamic environments

**Rejected:** Acceptable as fallback but insufficient as sole mechanism for cloud-native deployments.

### 2. DNS Service Discovery (DNS-SD)

**Pros:**

- Industry standard (RFC 6763)
- Well-understood operational model
- Works in Kubernetes/cloud environments

**Cons:**

- Requires external DNS infrastructure
- Centralized single point of failure
- Stale records if nodes crash
- Doesn't provide Iroh endpoint addresses (only IP/ports)

**Rejected:** Complementary to gossip but insufficient alone. Iroh's DNS discovery handles this separately.

### 3. Consul/etcd Registry

**Pros:**

- Centralized service registry
- Health checking
- Rich query capabilities

**Cons:**

- Additional infrastructure dependency
- Operationally complex
- Single point of failure
- Against Aspen's decentralized philosophy

**Rejected:** Violates decentralization principles. Aspen should not depend on external coordination services.

### 4. Custom Discovery Protocol

**Pros:**

- Tailored to exact requirements
- Full control over behavior

**Cons:**

- Reinventing the wheel
- Complex to get right (NAT traversal, security, etc.)
- Maintenance burden

**Rejected:** iroh-gossip already provides a robust, tested gossip protocol. No need to build from scratch.

## Consequences

### Positive

1. **Zero-configuration**: Nodes discover each other automatically
2. **Resilience**: No single point of failure in discovery
3. **Flexibility**: Works across diverse network environments
4. **Integration**: Leverages existing Iroh infrastructure
5. **Testing**: Gossip can be disabled for deterministic tests

### Negative

1. **Discovery latency**: New nodes take up to 10 seconds to be discovered
2. **Network overhead**: Periodic broadcasts consume bandwidth (minimal for small clusters)
3. **Cluster isolation**: Misconfigured cookies can cause accidental cluster merging (mitigated by BLAKE3 topic derivation)

### Operational Considerations

**Cluster Cookie Management**: The cluster cookie is security-critical:

- Acts as shared secret for cluster membership
- Should be randomly generated per cluster
- Must be distributed securely to all nodes
- Rotation requires cluster-wide coordination

**Testing Strategy** (see `docs/discovery-testing.md`):

- **CI tests**: Disable gossip, use manual peers (deterministic)
- **LAN tests**: Enable mDNS + gossip (real network)
- **Production tests**: Enable all discovery methods (DNS, Pkarr, gossip)

**Monitoring Metrics**:

- `gossip_announcements_sent`: Number of announcements broadcast
- `gossip_announcements_received`: Number of peer announcements processed
- `gossip_peers_discovered`: Unique peers discovered via gossip
- `gossip_invalid_messages`: Failed announcement parsing (security concern)

## Implementation Notes

### Bootstrap Sequence

1. Node starts, Iroh endpoint initialized
2. `GossipPeerDiscovery::spawn()` subscribes to gossip topic
3. Announcer task begins broadcasting every 10 seconds
4. Receiver task processes incoming announcements
5. Network factory updated with peer addresses
6. Raft RPC can now communicate with discovered peers

### Shutdown Sequence

1. `GossipPeerDiscovery::shutdown()` signals both tasks
2. Tasks check shutdown flag on next iteration
3. Tasks exit gracefully within 10-second timeout
4. Iroh gossip subscription dropped

### Error Handling

- **Gossip not enabled**: `spawn()` returns error immediately (fail-fast)
- **Subscription failure**: Propagated to caller (explicit error handling)
- **Broadcast failure**: Logged as warning, retry on next interval
- **Invalid announcement**: Logged as warning, message discarded

## References

- Implementation: `src/cluster/gossip_discovery.rs`
- Testing guide: `docs/discovery-testing.md`
- Integration test: `tests/gossip_auto_peer_connection_test.rs`
- Iroh gossip: <https://docs.rs/iroh-gossip>
- Tiger Style: `tigerstyle.md`
