# Discovery Implementation Analysis for Aspen

**Created**: 2025-12-26T16:55:47-05:00
**Context**: Analysis of iroh discovery mechanisms and recommendations for Aspen implementation

## Executive Summary

Aspen already has a **comprehensive, production-ready discovery implementation** that exceeds the basic requirements described in the iroh documentation. The codebase implements all four discovery mechanisms (mDNS, DNS, Pkarr/DHT, Gossip) with extensive configuration options, security features, and rate limiting.

**Key Finding**: No significant gaps exist between what iroh-dns-server offers and what Aspen currently implements. The only potential enhancement is running a private `iroh-dns-server` for completely isolated deployments.

## Current Aspen Discovery Capabilities

### 1. mDNS Discovery (Local Network)
- **Status**: Fully Implemented
- **Default**: Enabled
- **Location**: `src/cluster/mod.rs:515-518`
- **Config**: `enable_mdns: bool`

Uses `iroh::discovery::mdns::MdnsDiscovery` for automatic peer discovery on local networks without relay servers.

### 2. DNS Discovery (Production)
- **Status**: Fully Implemented
- **Default**: Disabled (opt-in)
- **Location**: `src/cluster/mod.rs:520-532`
- **Config**:
  - `enable_dns_discovery: bool`
  - `dns_discovery_url: Option<String>` (custom URL or n0's dns.iroh.link)

Uses `iroh::discovery::dns::DnsDiscovery` for production peer discovery via DNS lookups.

### 3. Pkarr DHT Discovery (Distributed)
- **Status**: Fully Implemented with DhtDiscovery upgrade
- **Default**: Disabled (opt-in)
- **Location**: `src/cluster/mod.rs:534-589`
- **Config** (all in `IrohConfig`):
  - `enable_pkarr: bool` - Master switch
  - `enable_pkarr_dht: bool` (default: true) - BitTorrent Mainline DHT
  - `enable_pkarr_relay: bool` (default: true) - Relay server fallback
  - `include_pkarr_direct_addresses: bool` (default: true) - Include direct IPs
  - `pkarr_republish_delay_secs: u64` (default: 600) - Refresh interval
  - `pkarr_relay_url: Option<String>` - Custom relay for private infrastructure

Uses `iroh::discovery::pkarr::dht::DhtDiscovery` which provides both **publishing AND resolution** (upgraded from PkarrPublisher which was publish-only).

### 4. Gossip Discovery (Application Layer)
- **Status**: Fully Implemented with security features
- **Default**: Enabled
- **Location**: `src/cluster/gossip_discovery.rs` (924 lines)
- **Config**: `enable_gossip: bool`

Features:
- Derives topic ID from cluster cookie via Blake3 hash
- Signed messages with Ed25519 (fail-fast on invalid signatures)
- Rate limiting: 12/min per peer, 300/min global
- LRU eviction for tracking >256 peers
- Adaptive backoff on failures
- Topology version announcements

### 5. Security Features
- **Raft Authentication**: Auto-enabled when Pkarr is enabled (`config.rs:416-423`)
- **Signature Verification**: All gossip messages are signed and verified
- **Rate Limiting**: Token bucket algorithm prevents DoS

### 6. Configuration Architecture
- **Multi-source**: Environment variables -> TOML config -> CLI flags
- **Validation**: Fail-fast semantics on invalid config
- **Security Defaults**: `apply_security_defaults()` enforces secure settings

## Comparison: iroh Documentation vs Aspen Implementation

| Feature | iroh Documentation | Aspen Status | Notes |
|---------|-------------------|--------------|-------|
| DNS Discovery | Enabled by default | Implemented (disabled by default) | Can use n0's dns.iroh.link or custom |
| Pkarr via DNS | Enabled by default | Implemented | Part of DhtDiscovery |
| DHT Discovery | Disabled by default | Implemented (disabled by default) | Full DhtDiscovery with publish+resolve |
| Local/mDNS | Disabled by default | Implemented (enabled by default) | Good for dev/testing |
| Custom DNS Server | Mentioned as TODO | **Not needed** | Pkarr relay serves same purpose |
| Custom Relay URL | Documented | Implemented | `pkarr_relay_url` config option |
| Private Infrastructure | Mentioned | Implemented | Can run custom pkarr relay |
| Rate Limiting | Not mentioned | Implemented | Per-peer and global limits |
| Signed Messages | Built into Pkarr | Implemented | Both Pkarr and Gossip signed |
| Security Defaults | Not mentioned | Implemented | Auto-enable Raft auth with Pkarr |

## What iroh-dns-server Provides

Based on research, `iroh-dns-server` is:
1. A pkarr relay that receives signed packets
2. A DNS server that serves those records
3. DNS-over-HTTPS (DoH) support (RFC 8484)

**Key insight**: This is what n0 runs at `dns.iroh.link`. Aspen can already use this via `DhtDiscovery::n0_dns_pkarr_relay()` or configure a custom relay URL.

## Recommendations

### What Aspen Does NOT Need to Implement

1. **iroh-dns-server binary** - Aspen doesn't need to run its own DNS server. The existing Pkarr relay integration (via `DhtDiscovery`) provides equivalent functionality.

2. **DNS zone management** - Not needed for peer discovery; Pkarr handles record signing and publishing.

3. **Custom Discovery trait** - Aspen uses iroh's built-in discovery implementations which are well-tested.

### Potential Enhancements (Low Priority)

#### 1. Private iroh-dns-server Deployment Guide
For users who want completely isolated infrastructure:
- Document how to deploy `iroh-dns-server` from iroh repo
- Configure Aspen to use custom `pkarr_relay_url` pointing to it
- This is already supported, just needs documentation

#### 2. DNS-over-HTTPS Support
Currently Aspen uses standard DNS. Could add DoH for:
- Bypassing DNS filtering
- Better privacy
- Already supported by iroh-dns-server

#### 3. Bootstrap Node List
Consider adding a default bootstrap node list for first-time discovery:
```toml
[iroh]
bootstrap_nodes = ["node1.aspen.example.com", "node2.aspen.example.com"]
```
This would help new nodes join without tickets or manual peer configuration.

### Recommended Actions

1. **No code changes required** - Current implementation is complete

2. **Documentation update** - Add a discovery configuration guide explaining:
   - When to use each discovery mechanism
   - Private infrastructure setup
   - Security implications of each option

3. **Integration tests** - Add tests for:
   - DNS discovery with custom URL
   - Pkarr with custom relay
   - Combined discovery (mDNS + DHT + Gossip)

## Architecture Diagram

```
                     Aspen Node Discovery Architecture
    ┌─────────────────────────────────────────────────────────────────┐
    │                        IrohEndpointManager                       │
    │  ┌──────────────────────────────────────────────────────────┐   │
    │  │                  Iroh Endpoint Builder                    │   │
    │  │  ┌─────────┐ ┌─────────┐ ┌─────────────┐ ┌────────────┐ │   │
    │  │  │  mDNS   │ │   DNS   │ │ DhtDiscovery │ │   Gossip   │ │   │
    │  │  │(default)│ │(opt-in) │ │   (opt-in)   │ │  (default) │ │   │
    │  │  └────┬────┘ └────┬────┘ └──────┬───────┘ └─────┬──────┘ │   │
    │  └───────┼───────────┼─────────────┼───────────────┼────────┘   │
    │          │           │             │               │            │
    └──────────┼───────────┼─────────────┼───────────────┼────────────┘
               │           │             │               │
               ▼           ▼             ▼               ▼
          Local LAN    dns.iroh.link  BitTorrent DHT   Cluster Topic
          (multicast)  or custom URL  + Pkarr Relay   (Blake3 hash)
```

## Configuration Examples

### Development (Default)
```toml
[iroh]
enable_mdns = true
enable_gossip = true
# DNS and Pkarr disabled - LAN discovery sufficient
```

### Production (Public Internet)
```toml
[iroh]
enable_mdns = false  # Disable LAN-only discovery
enable_dns_discovery = true
enable_pkarr = true
enable_pkarr_dht = true
enable_pkarr_relay = true
enable_raft_auth = true  # Auto-enabled with pkarr anyway
```

### Private Infrastructure
```toml
[iroh]
enable_mdns = false
enable_pkarr = true
enable_pkarr_dht = false  # Don't use public DHT
enable_pkarr_relay = true
pkarr_relay_url = "https://dns.internal.company.com"
relay_mode = "custom"
relay_urls = ["https://relay-us.internal.company.com", "https://relay-eu.internal.company.com"]
enable_raft_auth = true
```

## Conclusion

Aspen's discovery implementation is **already comprehensive and production-ready**. The codebase implements all mechanisms described in the iroh documentation, with additional security features (signed gossip, rate limiting, auto-auth) that exceed the basic requirements.

The only gap is documentation - users would benefit from a guide explaining when to use each discovery mechanism and how to configure private infrastructure. No code changes are required.

---

# Appendix: Embedding iroh-dns-server into Aspen

**Added**: 2025-12-26T17:00:00-05:00

## Overview

The `iroh-dns-server` crate (now part of the main iroh repo) **does expose a library API** that can be embedded into Aspen. This would allow Aspen nodes to serve as their own discovery infrastructure.

## iroh-dns-server Library API

### Server Spawning
```rust
use iroh_dns_server::{Config, Server, ZoneStore, Metrics};

// Create storage
let store = ZoneStore::in_memory(metrics.clone());  // or ZoneStore::persistent(path, opts, metrics)

// Spawn server
let server = Server::spawn(config, store, Arc::new(metrics)).await?;

// Later: graceful shutdown
server.shutdown().await?;
```

### Configuration Structure
```rust
pub struct Config {
    pub http: Option<HttpConfig>,      // HTTP server (port 8080 default)
    pub https: Option<HttpsConfig>,    // HTTPS server (port 8443 default)
    pub dns: DnsConfig,                // DNS server (port 5300 default)
    pub metrics_addr: Option<SocketAddr>,
    pub mainline: Option<MainlineConfig>,  // DHT fallback
    pub pkarr_put_rate_limit: Option<RateLimitConfig>,
    // ...
}
```

### ZoneStore Options
```rust
// In-memory (testing/ephemeral)
let store = ZoneStore::in_memory(metrics);

// Persistent (production)
let store = ZoneStore::persistent(data_dir, options, metrics)?;

// With DHT fallback
let store = store.with_mainline_fallback(bootstrap_option);
```

## Integration Architecture

### Option A: Dedicated DNS Node Role (Recommended)

Add a new node role where designated nodes run the DNS server:

```
┌─────────────────────────────────────────────────────────────┐
│                    Aspen Cluster                             │
│                                                             │
│  ┌───────────┐  ┌───────────┐  ┌───────────────────────┐   │
│  │  Node 1   │  │  Node 2   │  │  Node 3 (DNS Role)    │   │
│  │  (Voter)  │  │  (Voter)  │  │  (Voter + DNS Server) │   │
│  │           │  │           │  │  ┌─────────────────┐  │   │
│  │  Raft     │  │  Raft     │  │  │ iroh-dns-server │  │   │
│  │  KV Store │  │  KV Store │  │  │ - DNS :5300     │  │   │
│  │           │  │           │  │  │ - HTTP :8080    │  │   │
│  │           │  │           │  │  │ - HTTPS :8443   │  │   │
│  └───────────┘  └───────────┘  │  └─────────────────┘  │   │
│                                └───────────────────────────┘   │
└─────────────────────────────────────────────────────────────┘
```

### Option B: All Nodes Run DNS (Simple but Resource Heavy)

Every node runs an embedded DNS server. Simple but wasteful for large clusters.

### Option C: Sidecar Deployment (No Code Changes)

Deploy `iroh-dns-server` binary alongside Aspen nodes. Configure via:
```toml
[iroh]
pkarr_relay_url = "http://localhost:8080"
```

## Implementation Plan

### Phase 1: Add Dependency (Feature-Gated)

```toml
# Cargo.toml
[dependencies]
iroh-dns-server = { version = "0.x", optional = true }

[features]
default = []
dns-server = ["iroh-dns-server"]
```

### Phase 2: Create DNS Server Module

```rust
// src/dns_server.rs (new file)
use iroh_dns_server::{Config, Server, ZoneStore, Metrics};

pub struct EmbeddedDnsServer {
    server: Server,
    config: DnsServerConfig,
}

impl EmbeddedDnsServer {
    pub async fn spawn(config: DnsServerConfig) -> Result<Self> {
        let metrics = Metrics::new();
        let store = if let Some(path) = &config.data_dir {
            ZoneStore::persistent(path, Default::default(), Arc::new(metrics.clone()))?
        } else {
            ZoneStore::in_memory(Arc::new(metrics.clone()))
        };

        let server_config = Config {
            http: config.http_addr.map(|addr| HttpConfig { addr, ..Default::default() }),
            https: config.https_config.clone(),
            dns: DnsConfig {
                port: config.dns_port.unwrap_or(5300),
                origin: config.origin.clone(),
                ..Default::default()
            },
            ..Default::default()
        };

        let server = Server::spawn(server_config, store, Arc::new(metrics)).await?;
        Ok(Self { server, config })
    }

    pub async fn shutdown(self) -> Result<()> {
        self.server.shutdown().await
    }
}
```

### Phase 3: Add Configuration

```rust
// In src/cluster/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsServerConfig {
    /// Enable embedded DNS server
    #[serde(default)]
    pub enable: bool,

    /// DNS server port (default: 5300)
    pub dns_port: Option<u16>,

    /// HTTP server address for pkarr relay
    pub http_addr: Option<SocketAddr>,

    /// HTTPS configuration (optional)
    pub https_config: Option<HttpsConfig>,

    /// DNS origin domain (e.g., "aspen.local")
    pub origin: String,

    /// Data directory for persistent storage
    pub data_dir: Option<PathBuf>,

    /// Enable DHT fallback for resolution
    #[serde(default)]
    pub enable_dht_fallback: bool,
}
```

### Phase 4: Integration with Bootstrap

```rust
// In bootstrap.rs
if config.dns_server.enable {
    let dns_server = EmbeddedDnsServer::spawn(config.dns_server.clone()).await?;

    // Auto-configure nodes to use this DNS server
    let local_pkarr_url = format!("http://{}:{}",
        config.dns_server.http_addr.unwrap_or(([127, 0, 0, 1], 8080).into()));

    // Update IrohConfig to use local DNS
    iroh_config.pkarr_relay_url = Some(local_pkarr_url);
    iroh_config.dns_discovery_url = Some(format!("dns://localhost:{}",
        config.dns_server.dns_port.unwrap_or(5300)));
}
```

## Configuration Examples

### Single DNS Node
```toml
# dns-node.toml
[dns_server]
enable = true
dns_port = 5300
http_addr = "0.0.0.0:8080"
origin = "cluster.aspen.local"
data_dir = "/var/lib/aspen/dns"
enable_dht_fallback = true

[iroh]
enable_pkarr = true
pkarr_relay_url = "http://dns-node.internal:8080"
```

### Client Nodes
```toml
# client-node.toml
[dns_server]
enable = false

[iroh]
enable_pkarr = true
pkarr_relay_url = "http://dns-node.internal:8080"
enable_dns_discovery = true
dns_discovery_url = "dns://dns-node.internal:5300"
```

## Dependencies Added

The `iroh-dns-server` crate brings:
- `hickory-server` (DNS implementation)
- `axum` 0.8 (HTTP server - may conflict with existing axum 0.7)
- `governor` (rate limiting)
- `tokio-rustls-acme` (ACME certificates)
- `redb` (already used by Aspen)

**Potential Issue**: Axum version conflict (Aspen uses 0.7, iroh-dns-server uses 0.8). May need to upgrade Aspen's axum or wait for compatibility.

## Benefits

1. **Self-contained clusters**: No external DNS/pkarr relay needed
2. **Private infrastructure**: Full control over discovery
3. **Reduced latency**: Local DNS resolution
4. **Offline operation**: Clusters work without internet

## Tradeoffs

1. **Increased complexity**: Another service to manage
2. **Resource usage**: DNS server adds memory/CPU overhead
3. **Axum conflict**: May require dependency updates
4. **Operational burden**: Need to manage DNS zones

## Recommendation

**Start with Option C (Sidecar)** to validate the use case:
1. Deploy `iroh-dns-server` binary alongside Aspen
2. Configure Aspen to use it via `pkarr_relay_url`
3. If valuable, implement Option A (embedded with DNS role)

This avoids code changes while proving the value proposition.

---

# Appendix B: Raft-Integrated DNS Server (Deep Analysis)

**Added**: 2025-12-26T17:15:00-05:00

## The Question

> "What if we ran iroh-dns-server as part of Raft?"

This would make DNS records part of the **replicated state machine**, providing:
- Fault-tolerant discovery infrastructure
- Linearizable consistency for DNS records
- Automatic failover without external dependencies

## How It Would Work

### Architecture: Consul-Style DNS over Raft

```
┌─────────────────────────────────────────────────────────────────────────┐
│                       Aspen Cluster (3+ nodes)                          │
│                                                                         │
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐         │
│  │   Node 1        │  │   Node 2        │  │   Node 3        │         │
│  │   (Leader)      │  │   (Follower)    │  │   (Follower)    │         │
│  │                 │  │                 │  │                 │         │
│  │ ┌─────────────┐ │  │ ┌─────────────┐ │  │ ┌─────────────┐ │         │
│  │ │ Raft State  │ │  │ │ Raft State  │ │  │ │ Raft State  │ │         │
│  │ │ Machine     │◄├──┼─┤ Machine     │◄├──┼─┤ Machine     │ │         │
│  │ │             │ │  │ │             │ │  │ │             │ │         │
│  │ │ ┌─────────┐ │ │  │ │ ┌─────────┐ │ │  │ │ ┌─────────┐ │ │         │
│  │ │ │ KV Data │ │ │  │ │ │ KV Data │ │ │  │ │ │ KV Data │ │ │         │
│  │ │ └─────────┘ │ │  │ │ └─────────┘ │ │  │ │ └─────────┘ │ │         │
│  │ │ ┌─────────┐ │ │  │ │ ┌─────────┐ │ │  │ │ ┌─────────┐ │ │         │
│  │ │ │DNS Recs │ │ │  │ │ │DNS Recs │ │ │  │ │ │DNS Recs │ │ │         │
│  │ │ └─────────┘ │ │  │ │ └─────────┘ │ │  │ │ └─────────┘ │ │         │
│  │ └─────────────┘ │  │ └─────────────┘ │  │ └─────────────┘ │         │
│  │                 │  │                 │  │                 │         │
│  │ ┌─────────────┐ │  │ ┌─────────────┐ │  │ ┌─────────────┐ │         │
│  │ │DNS Server   │ │  │ │DNS Server   │ │  │ │DNS Server   │ │         │
│  │ │:5300 (UDP)  │ │  │ │:5300 (UDP)  │ │  │ │:5300 (UDP)  │ │         │
│  │ │             │ │  │ │             │ │  │ │             │ │         │
│  │ │Reads from   │ │  │ │Reads from   │ │  │ │Reads from   │ │         │
│  │ │local state  │ │  │ │local state  │ │  │ │local state  │ │         │
│  │ └─────────────┘ │  │ └─────────────┘ │  │ └─────────────┘ │         │
│  │                 │  │                 │  │                 │         │
│  │ ┌─────────────┐ │  │ ┌─────────────┐ │  │ ┌─────────────┐ │         │
│  │ │Pkarr HTTP   │ │  │ │Pkarr HTTP   │ │  │ │Pkarr HTTP   │ │         │
│  │ │:8080        │ │  │ │:8080        │ │  │ │:8080        │ │         │
│  │ │             │ │  │ │             │ │  │ │             │ │         │
│  │ │PUT → Raft   │ │  │ │PUT → Raft   │ │  │ │PUT → Raft   │ │         │
│  │ │GET → local  │ │  │ │GET → local  │ │  │ │GET → local  │ │         │
│  │ └─────────────┘ │  │ └─────────────┘ │  │ └─────────────┘ │         │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘         │
└─────────────────────────────────────────────────────────────────────────┘
                              │
                              ▼ Raft Consensus (writes)
                              │
                        ┌─────┴─────┐
                        │  Quorum   │
                        │  Commit   │
                        │  (2 of 3) │
                        └───────────┘
```

### Data Flow

**Write Path (Pkarr PUT)**:
1. Client PUTs signed packet to any node's HTTP endpoint
2. Node validates signature (Ed25519)
3. Node proposes `DnsSetRecord` to Raft
4. Leader replicates to quorum
5. All nodes apply to local state machine
6. Response returned to client

**Read Path (DNS Query)**:
1. Client queries any node's DNS server (UDP :5300)
2. Node reads from **local replicated state** (no consensus needed)
3. Returns DNS response immediately

This is the **Consul pattern**: writes go through Raft, reads serve from local replica.

## Implementation Approach

### Option 1: Store Pkarr Packets in Raft State Machine

Add new AppRequest variants to store raw Pkarr signed packets:

```rust
// In src/raft/types.rs
pub enum AppRequest {
    // Existing variants...

    /// Store a Pkarr signed packet (DNS record announcement)
    PkarrStore {
        /// z32-encoded public key (endpoint ID)
        public_key_z32: String,
        /// The raw signed packet bytes
        packet_bytes: Vec<u8>,
        /// Timestamp for freshness checking
        timestamp: u64,
    },

    /// Delete a Pkarr record
    PkarrDelete {
        /// z32-encoded public key to remove
        public_key_z32: String,
    },
}
```

Add a new table in SharedRedbStorage:

```rust
// In src/raft/storage_shared.rs
const SM_PKARR_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_pkarr");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredPkarrPacket {
    /// The raw signed packet (includes signature)
    pub packet_bytes: Vec<u8>,
    /// Timestamp from the packet
    pub timestamp: u64,
    /// When we received it (for TTL)
    pub received_at_ms: u64,
}
```

### Option 2: Use Existing KV Store with Prefix Convention

Simpler approach - no schema changes:

```rust
// Key: "pkarr:{z32_public_key}"
// Value: JSON-serialized StoredPkarrPacket

let key = format!("pkarr:{}", public_key.to_z32());
let value = serde_json::to_string(&StoredPkarrPacket {
    packet_bytes: signed_packet.to_relay_payload(),
    timestamp: signed_packet.timestamp(),
    received_at_ms: now_unix_ms(),
})?;

// Store via Raft
raft_node.write(WriteRequest::Set { key, value }).await?;
```

### Option 3: Full DNS Zone Replication (Advanced)

For comprehensive DNS, store parsed records:

```rust
// Table: SM_DNS_ZONES
const SM_DNS_ZONES: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_dns_zones");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsZone {
    pub origin: String,           // "cluster.aspen.local"
    pub soa: SoaRecord,
    pub ns_records: Vec<String>,
    pub default_ttl: u32,
}

// Table: SM_DNS_RECORDS
const SM_DNS_RECORDS: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_dns_records");

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsRecord {
    pub name: String,             // FQDN
    pub record_type: DnsRecordType,
    pub ttl: u32,
    pub data: DnsRecordData,      // A, AAAA, TXT, MX, etc.
    pub created_revision: u64,
    pub mod_revision: u64,
}
```

## Consistency Modes (Following Consul's Pattern)

Like Consul, Aspen should support multiple consistency modes for DNS reads:

### 1. Stale (Default for DNS) - Recommended
```rust
// Read directly from local replica
// ~10µs latency, may be slightly stale
let record = state_machine.get_pkarr(&public_key)?;
```

**Rationale** (from [Consul docs](https://developer.hashicorp.com/consul/docs/concept/consistency)):
> "HashiCorp strongly recommends using stale consistency mode for DNS lookups to optimize for performance over consistency when operating at scale."

Staleness is bounded by Raft replication lag (~10-50ms typically).

### 2. Consistent (Optional)
```rust
// ReadIndex protocol - confirm leader, wait for apply
let linearizer = raft.get_read_linearizer(ReadPolicy::ReadIndex).await?;
linearizer.await_ready(&raft).await?;
let record = state_machine.get_pkarr(&public_key)?;
```

**Use case**: Security-critical lookups where stale data could cause issues.

### 3. Leader-Only (Strongest)
```rust
// Only leader can respond, always fresh
if !raft.is_leader() {
    return Err(DnsError::NotLeader);
}
let record = state_machine.get_pkarr(&public_key)?;
```

**Use case**: Testing, debugging, or when you need absolute freshness.

## Performance Analysis

### Write Performance (Pkarr PUT)
| Path | Latency | Throughput |
|------|---------|------------|
| Single node (Redb) | ~2-3ms | 350+ ops/sec |
| 3-node cluster | ~8-10ms | 100+ ops/sec |
| 5-node cluster | ~12-15ms | 70+ ops/sec |

Writes are I/O bound by fsync. Aspen's single-fsync design helps significantly.

### Read Performance (DNS Query)
| Mode | Latency | Notes |
|------|---------|-------|
| Stale (local) | ~10-50µs | In-memory BTreeMap lookup |
| ReadIndex | ~100-500µs | Network round-trip to confirm leader |
| Leader-only | ~50-100µs | Fast but single point of failure |

DNS queries at stale mode can achieve **20,000+ QPS per node**.

### Comparison with Standalone iroh-dns-server
| Metric | Standalone | Raft-Integrated |
|--------|------------|-----------------|
| Write durability | Single node | Replicated (f+1 copies) |
| Read availability | Single node | Any node |
| Failover | Manual | Automatic |
| Consistency | N/A | Linearizable writes, tunable reads |
| Latency (write) | ~1-2ms | ~8-10ms (3-node) |
| Latency (read) | ~1ms | ~10-50µs (stale) |

## Failure Modes and Mitigations

### 1. Leader Failure
- **Behavior**: New leader elected in ~150-300ms
- **Impact**: Writes blocked during election
- **Mitigation**: Reads from followers continue (stale mode)

### 2. Network Partition (Minority)
- **Behavior**: Minority partition cannot commit writes
- **Impact**: DNS writes fail, reads continue from stale data
- **Mitigation**: Clients retry to majority partition

### 3. Network Partition (Majority)
- **Behavior**: Majority elects new leader, continues
- **Impact**: Old leader becomes stale, then steps down
- **Mitigation**: Raft leader lease prevents stale reads from old leader

### 4. Split Brain
- **Behavior**: Impossible with Raft (requires majority)
- **Mitigation**: Built into consensus protocol

### 5. Full Cluster Failure
- **Behavior**: All nodes down, no DNS service
- **Impact**: Complete outage
- **Mitigation**: External monitoring, fast restart, or fallback to DHT

## Security Considerations

### 1. Pkarr Signature Verification
All stored packets must be cryptographically verified:
```rust
// Before proposing to Raft
let signed_packet = SignedPacket::from_relay_payload(&public_key, &payload)?;
// from_relay_payload() verifies Ed25519 signature internally
```

### 2. Rate Limiting (Existing in Aspen)
Gossip discovery already has rate limiting that can be reused:
- Per-peer: 12/min
- Global: 300/min

### 3. Key Validation
Only accept records for keys that match the signer:
```rust
if signed_packet.public_key() != expected_public_key {
    return Err(DnsError::KeyMismatch);
}
```

### 4. TTL Enforcement
Expire old records automatically:
```rust
if now_unix_ms() > record.received_at_ms + (record.ttl as u64 * 1000) {
    // Record expired, don't serve
    return None;
}
```

## Implementation Roadmap

### Phase 1: Pkarr Storage in Raft (2-3 days)
1. Add `PkarrStore`/`PkarrDelete` to `AppRequest`
2. Add `SM_PKARR_TABLE` to SharedRedbStorage
3. Implement `apply_pkarr_in_txn()` helper
4. Add read methods: `get_pkarr()`, `scan_pkarr()`

### Phase 2: HTTP Pkarr Relay Endpoint (1-2 days)
1. Add `/pkarr/{z32}` endpoints to existing HTTP server
2. PUT: Validate signature, propose to Raft
3. GET: Read from local state machine (stale mode)

### Phase 3: DNS Server Integration (2-3 days)
1. Add `hickory-server` dependency (feature-gated)
2. Implement DNS query handler reading from state machine
3. Bind UDP :5300 on all nodes
4. Support `_iroh.<z32>.<origin>` TXT queries

### Phase 4: Configuration and Testing (1-2 days)
1. Add `DnsServerConfig` to cluster config
2. Integration tests with real DNS queries
3. Chaos testing (node failures during writes)

**Total estimate**: 6-10 days

## Tradeoffs Summary

### Advantages of Raft-Integrated DNS
1. **Fault tolerance**: Survives node failures automatically
2. **No external dependencies**: Self-contained cluster
3. **Consistent writes**: Linearizable via Raft
4. **Unified management**: Single system to operate
5. **Efficient reads**: Local state machine, no network hop

### Disadvantages
1. **Write latency**: ~8-10ms vs ~1-2ms standalone
2. **Complexity**: More code to maintain
3. **Resource usage**: DNS runs on all nodes
4. **Coupling**: DNS tied to Raft health

### When to Use Raft-Integrated DNS
- **Yes**: Self-contained clusters, private networks, high availability required
- **No**: High write throughput needed, public DNS, existing DNS infrastructure

## Recommendation

**Implement Option 1 (Pkarr packets in Raft state machine)** with these specifics:

1. Use **dedicated table** (`SM_PKARR_TABLE`) not KV prefix
2. Default to **stale reads** for DNS queries
3. Make DNS server **feature-gated** (`--features dns-server`)
4. Run DNS on **all nodes** (not designated role)

This provides the simplest path to fault-tolerant discovery while maintaining Aspen's single-binary deployment model.

---

# Appendix C: Final Recommendation - DNS Strategy for Aspen

**Added**: 2025-12-26T17:19:31-05:00

## Summary of Findings

After deep analysis of iroh's discovery mechanisms, iroh-experiments, Consul/etcd patterns, and Aspen's existing implementation, here's what we learned:

### What Iroh Provides
| Component | Function | Peer-to-Peer? |
|-----------|----------|---------------|
| DhtDiscovery | Publish/resolve via BitTorrent DHT | No (global DHT) |
| PkarrPublisher | Publish to relay (dns.iroh.link) | No (central relay) |
| DnsDiscovery | Resolve via DNS queries | No (DNS servers) |
| MdnsDiscovery | Local network broadcast | Yes (LAN only) |
| content-discovery (experiment) | Blob tracker | No (tracker server) |

**Key insight**: Iroh does NOT have cluster-internal discovery record sharing.

### What Aspen Already Has
Aspen's `gossip_discovery.rs` provides exactly what iroh lacks:
- Signed peer announcements (Ed25519)
- Cluster-wide gossip topic (derived from cookie)
- Rate limiting (12/min per peer, 300/min global)
- Automatic peer addition to Iroh endpoint
- Works without external infrastructure

## The Decision Matrix

| Use Case | Solution | Already Have? |
|----------|----------|---------------|
| Cluster nodes find each other | Gossip discovery | **Yes** |
| Survive node restarts | Gossip re-announces | **Yes** |
| External clients find cluster | DNS/Pkarr | **Yes** (configurable) |
| Private cluster (no internet) | mDNS + Gossip | **Yes** |
| Persist discovery across full restart | **Gap** | No |
| DNS interface for legacy clients | **Gap** | No |

## Recommendation: Three-Tier Approach

### Tier 1: Keep What Works (No Changes)
Aspen's current discovery is already excellent:
```
┌─────────────────────────────────────────────────────────────┐
│  Current Aspen Discovery (Production-Ready)                 │
│                                                             │
│  ┌─────────┐    gossip    ┌─────────┐    gossip    ┌─────────┐
│  │ Node A  │◄────────────►│ Node B  │◄────────────►│ Node C  │
│  │         │              │         │              │         │
│  │ mDNS    │              │ mDNS    │              │ mDNS    │
│  │ Pkarr?  │              │ Pkarr?  │              │ Pkarr?  │
│  └─────────┘              └─────────┘              └─────────┘
│                                                             │
│  Features:                                                  │
│  ✓ Signed announcements (Ed25519)                          │
│  ✓ Rate limiting                                            │
│  ✓ Works without external infra                             │
│  ✓ 10-second announce interval                              │
└─────────────────────────────────────────────────────────────┘
```

### Tier 2: Optional DNS Interface (Feature-Gated)
For clusters that need DNS queryability:

```toml
# Cargo.toml
[dependencies]
hickory-server = { version = "0.25", optional = true }

[features]
dns-server = ["hickory-server"]
```

Implementation approach:
1. Add thin DNS server that reads from gossip-discovered peers
2. Serve `_iroh.<z32>.<origin>` TXT records
3. No Raft integration needed - just read current peer state

```rust
// Pseudocode for DNS handler
async fn handle_dns_query(query: DnsQuery) -> DnsResponse {
    let node_id = parse_z32_from_query(&query)?;

    // Read from gossip-discovered peers (already in memory)
    if let Some(peer) = known_peers.get(&node_id) {
        return build_txt_response(peer.relay_url, peer.addrs);
    }

    // Fallback to Pkarr/DHT if not in cluster
    if let Some(resolved) = dht_discovery.resolve(node_id).await {
        return build_txt_response(resolved);
    }

    DnsResponse::NxDomain
}
```

### Tier 3: Raft/iroh-docs Persistence (Future, If Needed)
Only implement if you need:
- Discovery to survive complete cluster restart
- Strong consistency for discovery records
- External auditing of discovery state

**Current recommendation: Don't build this yet.** Gossip re-announces within 10 seconds of restart.

## What NOT to Do

1. **Don't embed iroh-dns-server** - It's designed for public relay, not cluster-internal use
2. **Don't replicate Pkarr packets via Raft** - Gossip already does this faster
3. **Don't use iroh-docs for discovery** - It's for documents, not ephemeral peer state
4. **Don't remove existing discovery** - It's well-designed and production-ready

## Implementation Priority

| Priority | Feature | Effort | Value |
|----------|---------|--------|-------|
| **0** | Keep current gossip | 0 days | High |
| **1** | Document discovery config | 1 day | Medium |
| **2** | Add DNS server (optional) | 3-4 days | Medium |
| **3** | Raft persistence | 6-10 days | Low |

## Configuration for Different Scenarios

### Development (Default)
```toml
[iroh]
enable_mdns = true
enable_gossip = true
# Everything else disabled - LAN discovery sufficient
```

### Production (Internet)
```toml
[iroh]
enable_mdns = false
enable_gossip = true
enable_pkarr = true
enable_pkarr_dht = true
enable_pkarr_relay = true
enable_raft_auth = true  # Auto-enabled with pkarr
```

### Air-Gapped/Private
```toml
[iroh]
enable_mdns = true       # If same LAN
enable_gossip = true     # Always
enable_pkarr = false     # No external calls
relay_mode = "disabled"  # Direct only
```

### With DNS Interface (Future)
```toml
[iroh]
enable_gossip = true

[dns_server]
enable = true
port = 5300
origin = "cluster.internal"
```

## Final Answer

**Do nothing for now.** Aspen's discovery is already better than what iroh provides out of the box because:

1. **Gossip works** - Nodes find each other within 10 seconds
2. **Signed & rate-limited** - Security is built in
3. **No external dependencies** - Works air-gapped
4. **Already integrated** - With Raft network factory

If you later need:
- **DNS interface** → Add thin hickory-server wrapper (Tier 2)
- **Persistence** → Consider Raft/iroh-docs (Tier 3)

But don't over-engineer. The current system handles the core use case well.
