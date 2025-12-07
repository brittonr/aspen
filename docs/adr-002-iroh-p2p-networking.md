# ADR-002: Iroh for P2P Networking

**Status:** Accepted
**Date:** 2025-12-03
**Authors:** Aspen Team

## Context

Aspen operates as a distributed orchestration layer requiring robust peer-to-peer communication across diverse network environments including:

- Cloud deployments with dynamic IPs and NAT traversal
- Multi-region topologies with variable latency
- Local development and testing scenarios
- Production environments with firewall restrictions

The networking layer must provide:

- **Reliable Transport**: Low-latency bidirectional streams for Raft RPC
- **NAT Traversal**: Automatic hole-punching for nodes behind NAT/firewalls
- **Peer Discovery**: Automatic or manual node discovery mechanisms
- **Content-Addressed Communication**: Cryptographic node identity
- **QUIC Protocol**: Modern transport with built-in encryption and multiplexing

Traditional approaches (TCP with manual configuration, libp2p complexity) were deemed insufficient for Aspen's requirements of operational simplicity combined with production robustness.

## Decision

We chose **Iroh 0.95.1** as the P2P networking foundation for Aspen, using:

- **QUIC Transport**: Bidirectional streams over UDP with built-in encryption
- **Iroh Endpoint**: Connection management and identity (Ed25519 keypairs)
- **Iroh Gossip**: Peer discovery via gossip protocol for automatic node announcement
- **IRPC Protocol**: Custom RPC layer (`irpc` crate) for Raft message serialization over Iroh streams

Integration points:

```rust
iroh = { version = "0.95.1", features = ["discovery-local-network", "discovery-pkarr-dht"] }
iroh-gossip = "0.95"
```

Core implementation: `src/cluster/mod.rs` (IrohEndpointManager, IrohEndpointConfig)
Network layer: `src/raft/network.rs` (IrpcRaftNetworkFactory, IrpcRaftNetwork)

## Rationale

### Why Iroh

1. **QUIC-First Architecture**
   - QUIC provides multiplexed streams, 0-RTT reconnection, and built-in TLS 1.3 encryption
   - UDP-based transport naturally handles NAT traversal better than TCP
   - Iroh wraps `quinn` (Rust QUIC implementation) with higher-level peer management
   - Reference: `src/raft/network.rs:136-169` shows QUIC bidirectional streams for RPC

2. **Built-In NAT Traversal**
   - Integrated STUN/TURN-like relay servers (configurable via `RelayUrl`)
   - Automatic hole-punching for direct peer connections when possible
   - Falls back to relay for symmetric NAT scenarios
   - Configuration: `IrohEndpointConfig::relay_urls` (max 4 relays, Tiger Style bounded limit)

3. **Multiple Discovery Mechanisms**
   - **mDNS**: Local network discovery (default enabled for LAN testing, `src/cluster/mod.rs:543-546`)
   - **DNS Discovery**: Production peer discovery via DNS lookups (`enable_dns_discovery` flag)
   - **Pkarr Publisher**: DHT-based address publishing for resilience (`enable_pkarr` flag)
   - **Gossip-Based**: iroh-gossip for broadcasting Raft metadata once connectivity established
   - **Manual Peers**: Explicit peer configuration for airgapped deployments

4. **Content-Addressed Identity**
   - Node identity derived from Ed25519 public key (cryptographically verifiable)
   - Prevents man-in-the-middle attacks and identity spoofing
   - Aligns with Plan 9-inspired security model (authentication built into networking layer)

5. **Mature P2P Foundation**
   - Powers [Iroh](https://iroh.computer/) project (distributed data sync/transfer)
   - Built on battle-tested `quinn` (Cloudflare's QUIC stack)
   - Active development and production usage in multiple projects
   - Well-documented with comprehensive examples

6. **Gossip Protocol Integration**
   - `iroh-gossip` provides eventual consistency for peer discovery
   - Nodes broadcast `node_id + EndpointAddr` every 10 seconds
   - Automatic peer list updates without central coordination
   - Reference: `src/cluster/gossip_discovery.rs` (gossip announcement logic)

### Architecture Benefits

The Iroh networking stack integrates cleanly with Aspen's actor-based architecture:

```
┌─────────────────┐
│  RaftActor      │  Consensus logic (openraft)
│  (ractor)       │
└────────┬────────┘
         │
┌────────▼────────┐
│ IrpcRaftNetwork │  RPC serialization (postcard)
│                 │
└────────┬────────┘
         │
┌────────▼────────┐
│ IrohEndpoint    │  QUIC transport + identity
│ (iroh)          │
└────────┬────────┘
         │
         ├─────────► Gossip (peer discovery)
         ├─────────► IRPC (Raft RPC: vote, append_entries, snapshot)
         └─────────► HTTP Control Plane (cluster management API)
```

### Discovery Strategy

Aspen supports a flexible multi-layered discovery approach (all can work simultaneously):

1. **Establish Connectivity** (via Iroh discovery services):
   - mDNS for LAN nodes
   - DNS Discovery for cloud regions
   - Pkarr for DHT resilience

2. **Broadcast Raft Metadata** (via gossip, default enabled):
   - Once Iroh connectivity exists, gossip announces `node_id` + `EndpointAddr`
   - Received announcements populate Raft network factory's peer map
   - Enables automatic cluster formation without manual configuration

3. **Fallback to Manual** (when all discovery disabled):
   - CLI flag: `--peers "node_id@endpoint_id"`
   - Config file: `peers = ["node_id@endpoint_id"]`
   - Use cases: localhost testing, airgapped deployments, custom discovery

### Implementation Highlights

**IrohEndpointManager** (`src/cluster/mod.rs:487-651`):

- Manages Iroh endpoint lifecycle with explicit shutdown
- Fixed limits: max 4 relay URLs (Tiger Style resource bounds)
- Exposes `node_addr()` for HTTP-based peer exchange
- Optional gossip spawning based on `enable_gossip` flag

**IrpcRaftNetwork** (`src/raft/network.rs:88-297`):

- Implements OpenRaft's `RaftNetworkV2` trait
- Maps Raft RPC (vote, append_entries, snapshot) to QUIC streams
- 10MB message size limit for RPC payloads (bounded resource usage)
- Peer address lookup from dynamic map (supports runtime peer addition)

**Gossip Discovery** (`src/cluster/gossip_discovery.rs`):

- Subscribe to cluster-wide gossip topic (derived from cookie)
- Periodic announcements with configurable interval (default 10s)
- Automatic peer map updates for Raft network factory

## Alternatives Considered

### Alternative 1: libp2p

**Why rejected:**

- Heavy dependency tree and complex configuration surface
- Designed for maximum generality (pub-sub, DHT, etc.) beyond Aspen's needs
- Steeper learning curve for team without direct value-add
- Iroh provides focused P2P primitives with better ergonomics for Rust
- Iroh uses libp2p's `PeerId` for compatibility but avoids full stack complexity

**Trade-off**: Less ecosystem compatibility vs. simpler mental model and faster iteration

### Alternative 2: Raw tokio-quinn (QUIC only)

**Why rejected:**

- Would require implementing NAT traversal, peer discovery, relay logic from scratch
- Iroh provides these as batteries-included features
- Violates Tiger Style principle: avoid reinventing well-tested infrastructure
- Maintenance burden of custom discovery protocols

**Trade-off**: More control vs. significant engineering time investment

### Alternative 3: gRPC over TCP

**Why rejected:**

- TCP has poor NAT traversal characteristics compared to UDP-based QUIC
- No built-in peer discovery or content-addressed identity
- Requires separate service mesh or load balancer for dynamic addressing
- Less suitable for P2P topologies (Aspen targets decentralized deployments)

**Trade-off**: Industry familiarity vs. P2P operational requirements

### Alternative 4: Custom Protocol over UDP

**Why rejected:**

- Would need to implement reliability, ordering, congestion control
- QUIC already solves these problems with years of production hardening
- Violates Zero Technical Debt principle: don't rush custom protocols
- Cryptographic identity and encryption would require additional work

**Trade-off**: Maximum flexibility vs. proven correctness

## Consequences

### Positive

- **Operational Simplicity**: Gossip-based discovery eliminates manual peer configuration for most deployments
- **NAT Transparent**: Works across cloud providers, home networks, and complex firewall setups
- **Security by Default**: All communication encrypted via QUIC/TLS 1.3, identity bound to cryptographic keys
- **Observability**: Iroh metrics integration (`iroh-metrics`) for connection telemetry
- **Future-Proof**: Iroh roadmap includes distributed data structures (CRDTs) potentially useful for Aspen
- **Local Testing**: mDNS enables zero-config multi-node testing on same LAN (not localhost)

### Negative

- **Localhost Limitation**: mDNS doesn't work on 127.0.0.1 (multicast limitation), requires manual peers for single-machine testing
- **Relay Dependency**: NAT traversal relies on relay servers (can self-host but adds operational burden)
- **Iroh Version Lock**: Rapid Iroh development means potential breaking changes between minor versions
- **UDP Blocked Networks**: Some restrictive networks block UDP entirely (rare but possible)
- **Binary Size**: Iroh + quinn + crypto dependencies add ~3MB to binary

### Neutral

- **Gossip Bandwidth**: Periodic announcements consume network (~1KB every 10s per node, acceptable for server environments)
- **Discovery Latency**: Gossip-based discovery takes 10-30s for full cluster awareness (acceptable for long-lived clusters)
- **Relay Costs**: Public relay servers (n0's service) have usage quotas, production deployments should self-host
- **QUIC Learning Curve**: Team needs to understand QUIC streams, connection migration concepts

## Implementation Notes

### Iroh Endpoint Configuration

**Tiger Style Resource Bounds**:

- Max 4 relay URLs (`MAX_RELAY_URLS` constant enforced)
- 10MB RPC message size limit (`MAX_RPC_MESSAGE_SIZE` in `src/raft/network.rs:30`)
- Explicit bind port configuration (0 = OS-assigned random port)

**Discovery Services** (`src/cluster/mod.rs:541-574`):

```rust
// mDNS: Local network (enabled by default)
if config.enable_mdns {
    builder = builder.discovery(iroh::discovery::mdns::MdnsDiscovery::builder());
}

// DNS Discovery: Production (opt-in)
if config.enable_dns_discovery {
    let dns_builder = iroh::discovery::dns::DnsDiscovery::n0_dns();
    builder = builder.discovery(dns_builder);
}

// Pkarr: DHT-based (opt-in)
if config.enable_pkarr {
    let pkarr_builder = iroh::discovery::pkarr::PkarrPublisher::n0_dns();
    builder = builder.discovery(pkarr_builder);
}
```

### IRPC Protocol

**Serialization**: `postcard` (space-efficient binary format)
**Transport**: QUIC bidirectional streams
**Pattern**: Request-response over single stream (not long-lived)

RPC Flow (`src/raft/network.rs:129-170`):

1. Open connection to peer via `endpoint.connect(peer_addr, b"raft-rpc")`
2. Open bidirectional stream
3. Serialize request with postcard, write to send stream
4. Read response from receive stream (with 10MB limit)
5. Deserialize response and return

### Peer Discovery Integration

**Dynamic Peer Map** (`src/raft/network.rs:36-86`):

```rust
peer_addrs: Arc<RwLock<HashMap<NodeId, iroh::EndpointAddr>>>
```

Updated via:

- Initial config: CLI `--peers` or config file peers
- Gossip announcements: `gossip_discovery.rs` listener
- Manual additions: `add_peer()` / `update_peers()` API

### Gossip Topic Derivation

Topic ID derived from cluster cookie (ensures isolation between clusters):

```rust
let topic = TopicId::from(blake3::hash(cookie.as_bytes()).as_bytes());
```

### Cluster Tickets

Convenience mechanism for joining clusters (`src/cluster/ticket.rs`):

- First node starts with default gossip (topic from cookie)
- HTTP GET `/cluster-ticket` returns serialized ticket (includes relay URLs, gossip topic)
- New nodes use `--ticket "aspen{...}"` to bootstrap configuration

## References

- [Iroh Documentation](https://docs.iroh.computer/)
- [Iroh GitHub](https://github.com/n0-computer/iroh)
- [Quinn QUIC Implementation](https://github.com/quinn-rs/quinn)
- [QUIC Protocol Spec (RFC 9000)](https://www.rfc-editor.org/rfc/rfc9000.html)
- [iroh-gossip Design](https://docs.iroh.computer/gossip)
- [IRPC Protocol Crate](https://crates.io/crates/irpc)
- Local files:
  - `src/cluster/mod.rs` - Endpoint manager (lines 369-651)
  - `src/raft/network.rs` - Raft RPC over Iroh (lines 1-297)
  - `src/cluster/gossip_discovery.rs` - Gossip-based peer discovery
  - `src/cluster/ticket.rs` - Cluster ticket serialization
  - `src/cluster/bootstrap.rs` - Discovery configuration
  - `scripts/aspen-cluster-smoke.sh` - 3-node cluster test with gossip
