# Aspen vs FoundationDB vs etcd vs CockroachDB: Comprehensive Comparison

*Analysis generated: 2025-12-20*

## Executive Summary

This document provides a deep technical comparison between Aspen and three major distributed systems: FoundationDB, etcd, and CockroachDB. Each system occupies a different niche in the distributed systems landscape:

| System | Primary Use Case | Architecture Style | Consistency Model |
|--------|-----------------|-------------------|-------------------|
| **Aspen** | Distributed orchestration layer | Raft + P2P (Iroh) | Linearizable |
| **FoundationDB** | Multi-model database foundation | OCC + MVCC + Paxos | Strict Serializable |
| **etcd** | Configuration/service discovery | Raft | Linearizable |
| **CockroachDB** | Distributed SQL database | Raft + SQL layer | Serializable/Snapshot |

---

## 1. Architecture Comparison

### 1.1 High-Level Architecture

#### Aspen

```
Client RPC (Iroh QUIC) / Terminal UI (ratatui)
         |
ClusterController + KeyValueStore Traits
         |
RaftNode (Direct Async Implementation)
         |
OpenRaft 0.10.0 (vendored)
    +-- RedbLogStore (append-only log)
    +-- SqliteStateMachine (ACID state machine)
    +-- IrpcRaftNetwork (IRPC over Iroh)
         |
IrohEndpointManager (QUIC + NAT traversal)
    +-- mDNS discovery (local)
    +-- DNS discovery (production)
    +-- Pkarr (DHT fallback)
    +-- Gossip (peer announcements)
    +-- iroh-blobs (content-addressed storage)
    +-- iroh-docs (CRDT replication)
```

**Key architectural decisions:**

- **Direct async APIs** over actors (simplicity, reduced latency)
- **Hybrid storage**: redb for Raft log (append-optimized), SQLite for state machine (queryable)
- **P2P-first networking**: Iroh QUIC with built-in NAT traversal
- **Vendored OpenRaft**: Tight control over consensus layer

#### FoundationDB

```
Client Layer (Language Bindings)
         |
Cluster Controller (Active Disk Paxos)
         |
Transaction System (OCC + MVCC)
    +-- Sequencer (version assignment)
    +-- Proxies (MVCC reads, commit orchestration)
    +-- Resolvers (conflict detection)
    +-- LogServers (distributed persistent queues)
         |
Storage Servers (range-sharded)
```

**Key architectural decisions:**

- **Unbundled architecture**: Control plane, transaction system, and storage independently scalable
- **OCC with MVCC**: Optimistic concurrency with multi-version reads
- **Proactive failure handling**: Entire transaction system shuts down on failure for fast recovery
- **No long-lived transactions**: In-memory linearization prevents long-running txns

#### etcd

```
Client (gRPC)
         |
etcd Server
    +-- Raft Consensus
    +-- WAL (Write-Ahead Log)
    +-- BoltDB/bbolt Storage
    +-- Watch API
         |
Cluster (typically 3-5 nodes)
```

**Key architectural decisions:**

- **Simplicity first**: Single binary, minimal configuration
- **Raft for everything**: Both consensus and data storage
- **Watch API**: Core primitive for configuration change notifications
- **Kubernetes-optimized**: Primary use case is cluster state management

#### CockroachDB

```
SQL Layer
    +-- Parser, Optimizer, Execution Engine
         |
Distribution Layer
    +-- Range-based sharding
    +-- Automatic rebalancing
         |
Replication Layer (Raft per range)
         |
Storage Layer (Pebble/LSM-tree)
```

**Key architectural decisions:**

- **SQL on Raft**: Full SQL semantics over distributed consensus
- **Range-based sharding**: Data split into 512MB ranges, each with its own Raft group
- **Multi-active**: Every node can serve reads and writes
- **PostgreSQL wire protocol**: Drop-in replacement compatibility

---

## 2. Consensus Mechanism Comparison

### 2.1 Consensus Protocols

| Aspect | Aspen | FoundationDB | etcd | CockroachDB |
|--------|-------|--------------|------|-------------|
| **Protocol** | Raft (OpenRaft) | Active Disk Paxos + OCC | Raft (etcd/raft) | Raft (per range) |
| **Implementation** | Vendored, Rust | Custom C++ | Go (etcd/raft) | Go (etcd/raft fork) |
| **Quorum** | Majority | Majority | Majority | Majority |
| **Leader Election** | Randomized timeout | Coordinator election | Randomized timeout | Randomized timeout |
| **Joint Consensus** | Yes (OpenRaft) | Custom reconfiguration | Yes | Yes |

### 2.2 Transaction Model

| Aspect | Aspen | FoundationDB | etcd | CockroachDB |
|--------|-------|--------------|------|-------------|
| **Transaction Type** | Single-key linearizable | ACID multi-key | Single-key linearizable | ACID multi-key SQL |
| **Isolation** | Linearizable reads/writes | Strict Serializability | Linearizable | Serializable + Snapshot |
| **Conflict Resolution** | Leader serialization | OCC with conflict detection | Leader serialization | Write intents + locking |
| **Long Transactions** | N/A (single-key) | Not supported | N/A | Supported |

### 2.3 Read Path

| System | Default Read | Linearizable Read | Stale Read |
|--------|-------------|-------------------|------------|
| **Aspen** | ReadIndex protocol | Yes (via leader confirmation) | Yes (local SQLite) |
| **FoundationDB** | MVCC snapshot | Yes (with read version) | Yes (past versions) |
| **etcd** | Serializable (leader) | Yes | Yes (local reads) |
| **CockroachDB** | Serializable | Yes | Yes (follower reads) |

### 2.4 Aspen's Unique Consensus Features

1. **Chain hashing for integrity**: Blake3 hash chains detect hardware corruption and Byzantine modification

   ```
   entry_hash = blake3(prev_hash || log_index || term || entry_data)
   ```

2. **Clock drift detection**: Observational monitoring (doesn't affect consensus, helps operators)
   - Warning threshold: 100ms, Alert: 500ms

3. **Node failure detection**: Distinguishes transient RPC errors from true node crashes
   - Prevents false-positive leader removals

---

## 3. Storage Architecture Comparison

### 3.1 Storage Engines

| System | Log Storage | State Storage | Engine Type |
|--------|------------|---------------|-------------|
| **Aspen** | redb | SQLite | Append-only + B-tree |
| **FoundationDB** | SQLite (persistent queues) | SQLite (storage servers) | B-tree |
| **etcd** | WAL files | bbolt (fork of BoltDB) | B+tree |
| **CockroachDB** | WAL + Raft log | Pebble (RocksDB clone) | LSM-tree |

### 3.2 Aspen's Hybrid Storage Design

**Why two storage engines?**

| Component | Engine | Rationale |
|-----------|--------|-----------|
| **Raft Log** | redb | Append-only access pattern; sequential write optimization |
| **State Machine** | SQLite | Random access reads; ACID transactions; SQL queryability |

This separation allows:

- Raft log optimized for sequential append (redb excels here)
- State machine optimized for point lookups and range scans (SQLite excels here)
- SQL queries directly on state machine without log replay

**Resource bounds (Tiger Style):**

```
MAX_KEY_SIZE: 1 KB
MAX_VALUE_SIZE: 1 MB
MAX_BATCH_SIZE: 1,000 entries
MAX_SNAPSHOT_SIZE: 100 MB
MAX_SNAPSHOT_ENTRIES: 1,000,000
```

### 3.3 Snapshot & Recovery

| System | Snapshot Format | Incremental Snapshots | Recovery Strategy |
|--------|----------------|----------------------|-------------------|
| **Aspen** | SQLite export (streaming) | Partial (via SQLite) | Log replay from snapshot |
| **FoundationDB** | N/A (storage servers are authoritative) | Continuous | Re-replicate from peers |
| **etcd** | Custom binary format | No | Log replay from snapshot |
| **CockroachDB** | RocksDB SST files | Yes (LSM-based) | Range-level recovery |

---

## 4. Networking Comparison

### 4.1 Network Architecture

| Aspect | Aspen | FoundationDB | etcd | CockroachDB |
|--------|-------|--------------|------|-------------|
| **Transport** | QUIC (via Iroh) | TCP | TCP (gRPC) | TCP (gRPC) |
| **Encryption** | TLS 1.3 (QUIC) | TLS (optional) | TLS (optional) | TLS (optional) |
| **NAT Traversal** | Built-in (Iroh) | None | None | None |
| **Service Discovery** | mDNS, DNS, Pkarr, Gossip | Manual | Manual/DNS | Manual/Kubernetes |

### 4.2 Aspen's P2P-First Approach

**Iroh provides:**

1. **QUIC transport**: Multiplexed streams, 0-RTT, built-in encryption
2. **NAT hole punching**: Works in ~85% of network conditions
3. **Relay fallback**: For cases where direct connection fails
4. **Content-addressed storage**: iroh-blobs for large values
5. **CRDT synchronization**: iroh-docs for real-time replication

**Discovery mechanisms:**

| Method | Scope | Default | Use Case |
|--------|-------|---------|----------|
| mDNS | Local LAN | ON | Development/testing |
| Gossip | Cluster-wide | ON | Auto-discovery |
| DNS | Internet | OFF | Production |
| Pkarr | DHT | OFF | Decentralized backup |

### 4.3 Connection Limits (Aspen Tiger Style)

```
MAX_PEERS: 64 nodes
MAX_CONCURRENT_CONNECTIONS: 500
MAX_STREAMS_PER_CONNECTION: 100
IROH_CONNECT_TIMEOUT: 5 seconds
IROH_STREAM_OPEN_TIMEOUT: 2 seconds
IROH_READ_TIMEOUT: 10 seconds
MAX_RPC_MESSAGE_SIZE: 10 MB
```

---

## 5. Testing Methodology Comparison

### 5.1 Testing Philosophy

| System | Primary Testing Approach | Unique Innovation |
|--------|-------------------------|-------------------|
| **Aspen** | Deterministic simulation (madsim) + proptest | Tiger Style bounds everywhere |
| **FoundationDB** | Deterministic simulation (custom) | 18 months on simulator before disk I/O |
| **etcd** | Integration tests + Jepsen | Kubernetes-scale testing |
| **CockroachDB** | Roachtest + Jepsen | Nightly distributed tests |

### 5.2 FoundationDB's Legendary Simulation

FoundationDB pioneered deterministic simulation testing:

> "In the entire history of the company, I think we only ever had one or two bugs reported by a customer. Ever. Kyle Kingsbury (aphyr) didn't even bother testing it with Jepsen, because he didn't think he'd find anything."

**Key advantages:**

1. **Perfect reproducibility**: Same seed = exact same execution
2. **Time compression**: Simulate hours of operation in minutes
3. **Comprehensive failure injection**: Network, disk, machine, datacenter
4. **Finding bugs early**: Catches race conditions before production

### 5.3 Aspen's Testing Approach (Inspired by FDB)

**Madsim integration:**

- Deterministic simulation framework for Rust
- Same seed reproduces exact execution
- Artifact capture for CI debugging

**Test categories:**

```
Unit tests:           ~100+
Integration tests:    ~150+
Property-based tests: ~50+ (proptest/bolero)
Simulation tests:     ~50+ (madsim)
Chaos tests:          ~20+ (failure injection)
Total:                350+ tests (100% pass rate)
```

**Unique testing features:**

1. **SimulationArtifactBuilder**: Captures seed, event trace, metrics
2. **Router tests**: In-memory multi-node clusters
3. **Chaos scenarios**: Rolling failures, Byzantine conditions
4. **Tiger Style bounds**: All resource limits compile-time enforced

### 5.4 Jepsen Testing

| System | Jepsen Tested | Results |
|--------|--------------|---------|
| **Aspen** | Not yet | N/A |
| **FoundationDB** | Declined by Jepsen | "Simulator already more rigorous" |
| **etcd** | Yes (2020) | Several issues found and fixed |
| **CockroachDB** | Yes (multiple) | Issues found and fixed |

---

## 6. Feature Comparison Matrix

### 6.1 Data Model & API

| Feature | Aspen | FoundationDB | etcd | CockroachDB |
|---------|-------|--------------|------|-------------|
| **Data Model** | KV + SQL queries | KV | KV | Relational (SQL) |
| **API Protocol** | Iroh RPC (QUIC) | Native client | gRPC | PostgreSQL wire |
| **Watch/Subscribe** | Log subscriber | Watches | Watch API | CDC (Changefeeds) |
| **Transactions** | Single-key | Multi-key ACID | Single-key | Multi-key ACID |
| **Secondary Indexes** | Via SQL | Via layers | No | Yes (automatic) |
| **Range Scans** | Yes (prefix) | Yes | Yes (prefix) | Yes (SQL WHERE) |

### 6.2 Operational Features

| Feature | Aspen | FoundationDB | etcd | CockroachDB |
|---------|-------|--------------|------|-------------|
| **Auto-sharding** | No (single Raft group) | Yes | No | Yes (ranges) |
| **Online Scaling** | Add/remove nodes | Add/remove nodes | Add/remove nodes | Add/remove nodes |
| **Backup/Restore** | Snapshots | fdbbackup | etcdctl snapshot | SQL BACKUP |
| **Multi-region** | Yes (Iroh) | Yes | No | Yes (geo-partitioning) |
| **Metrics** | Prometheus adapter | Prometheus | Prometheus | Built-in UI + Prometheus |

### 6.3 Unique Aspen Features

1. **Content-addressed blob storage** (iroh-blobs): Values > 1MB automatically offloaded
2. **CRDT synchronization** (iroh-docs): Real-time client replication
3. **FUSE filesystem**: Mount cluster as POSIX filesystem
4. **Terminal UI** (ratatui): Interactive cluster monitoring
5. **Chain hashing**: Cryptographic log integrity verification
6. **Zero-config discovery**: mDNS, gossip, Pkarr DHT

---

## 7. Performance Characteristics

### 7.1 Throughput Expectations

| System | Typical Write Throughput | Typical Read Throughput | Notes |
|--------|-------------------------|------------------------|-------|
| **Aspen** | 10K-50K ops/sec (estimated) | 100K+ ops/sec (pooled SQLite) | Single Raft group, no sharding |
| **FoundationDB** | 100K+ txns/sec | 1M+ reads/sec | Scales with cluster size |
| **etcd** | 10K-50K ops/sec | 100K+ ops/sec | Single Raft group |
| **CockroachDB** | Scales with ranges | Scales with ranges | Linear scaling via sharding |

### 7.2 Latency Expectations

| System | Write Latency (p50) | Write Latency (p99) | Notes |
|--------|--------------------|--------------------|-------|
| **Aspen** | 1-5ms | 10-50ms | QUIC + Raft commit |
| **FoundationDB** | 1-5ms | 10-20ms | OCC commit |
| **etcd** | 1-5ms | 10-50ms | Raft commit |
| **CockroachDB** | 5-20ms | 50-200ms | SQL parsing + Raft |

### 7.3 Resource Limits Comparison

| Limit | Aspen | FoundationDB | etcd | CockroachDB |
|-------|-------|--------------|------|-------------|
| **Max Key Size** | 1 KB | 10 KB | 1.5 MB | 256 MB (practical) |
| **Max Value Size** | 1 MB (blobs for larger) | 100 KB | 1.5 MB | Unlimited |
| **Max Txn Size** | N/A (single-key) | 10 MB | N/A | 256 MB |
| **Max Cluster Size** | 64 nodes | 1000+ nodes | 5-7 nodes | Unlimited |

---

## 8. Use Case Fit Analysis

### 8.1 When to Use Each System

#### Use Aspen When

- **P2P networking required**: NAT traversal, edge deployments
- **Simple KV with SQL queries**: Read-only SQL on key-value data
- **Content-addressed storage needed**: Large values with deduplication
- **Zero-config discovery**: Automatic cluster formation
- **Deterministic testing critical**: Madsim integration
- **Rust ecosystem**: Native Rust with modern async

#### Use FoundationDB When

- **Massive scale required**: 100K+ transactions/sec
- **Multi-model database**: Build custom layers on top
- **Strictest correctness**: Simulation-tested for years
- **Apple/Snowflake-scale workloads**: Proven at extreme scale

#### Use etcd When

- **Kubernetes cluster state**: Primary use case
- **Simple configuration store**: Small data, frequent reads
- **Watch API critical**: Real-time configuration updates
- **Minimal operational overhead**: Single binary, simple setup

#### Use CockroachDB When

- **SQL required**: Full PostgreSQL compatibility
- **Geo-distributed deployments**: Multi-region with partitioning
- **ACID transactions across tables**: Relational data model
- **Drop-in PostgreSQL replacement**: Wire protocol compatible

### 8.2 Not Suitable For

| System | Poor Fit |
|--------|----------|
| **Aspen** | Massive scale (>64 nodes), multi-key transactions |
| **FoundationDB** | Simple deployments, PostgreSQL compatibility |
| **etcd** | Large values, high write throughput |
| **CockroachDB** | Simple KV, sub-millisecond latency |

---

## 9. Architectural Trade-offs Summary

### 9.1 Aspen Trade-offs

| Decision | Pro | Con |
|----------|-----|-----|
| Single Raft group | Simplicity, strong consistency | Limited horizontal scaling |
| Vendored OpenRaft | Control, stability | Maintenance burden |
| Iroh P2P | NAT traversal, zero-config | Less mature than TCP |
| SQLite state machine | SQL queries, ACID | Single-threaded writes |
| Tiger Style bounds | Predictable resources | Less flexibility |

### 9.2 FoundationDB Trade-offs

| Decision | Pro | Con |
|----------|-----|-----|
| Unbundled architecture | Independent scaling | Operational complexity |
| No long transactions | Simple conflict resolution | Application constraints |
| Custom simulation | Exceptional reliability | High development cost |
| KV-only core | Flexibility via layers | No built-in SQL |

### 9.3 etcd Trade-offs

| Decision | Pro | Con |
|----------|-----|-----|
| Simplicity | Easy to operate | Limited scalability |
| Raft for everything | Consistency | Performance ceiling |
| BoltDB storage | Stability | Not optimal for large data |
| Kubernetes focus | Ecosystem integration | Less general-purpose |

### 9.4 CockroachDB Trade-offs

| Decision | Pro | Con |
|----------|-----|-----|
| Full SQL | Familiar interface | Parsing/planning overhead |
| Per-range Raft | Horizontal scaling | Cross-range txn complexity |
| PostgreSQL wire | Compatibility | Not all features supported |
| LSM storage | Write throughput | Read amplification |

---

## 10. Conclusion

### 10.1 Summary

Aspen occupies a unique position in the distributed systems landscape:

1. **Architecture**: Direct async APIs over vendored OpenRaft, hybrid storage (redb + SQLite)
2. **Networking**: P2P-first with Iroh QUIC, built-in NAT traversal, zero-config discovery
3. **Consistency**: Linearizable reads/writes via Raft, optional stale reads
4. **Testing**: Deterministic simulation inspired by FoundationDB
5. **Unique features**: Chain hashing, iroh-blobs, iroh-docs, FUSE, SQL queries

### 10.2 Key Differentiators

| vs FoundationDB | Aspen Advantage | FoundationDB Advantage |
|-----------------|-----------------|----------------------|
| Networking | NAT traversal, P2P | Proven at massive scale |
| Testing | Madsim (Rust) | More mature simulator |
| Operations | Zero-config discovery | More tooling |
| Language | Modern Rust async | Stable C++ |

| vs etcd | Aspen Advantage | etcd Advantage |
|---------|-----------------|----------------|
| Networking | P2P, NAT traversal | Simpler TCP |
| Features | SQL queries, blobs, CRDT | Kubernetes integration |
| Storage | Hybrid (redb + SQLite) | Simpler (bbolt) |
| API | Iroh QUIC | gRPC ecosystem |

| vs CockroachDB | Aspen Advantage | CockroachDB Advantage |
|----------------|-----------------|----------------------|
| Complexity | Simpler (single Raft) | Full SQL, auto-sharding |
| Networking | P2P, NAT traversal | Enterprise features |
| Latency | Lower (QUIC) | Mature optimizations |
| Scale | Predictable limits | Unlimited horizontal |

### 10.3 Bottom Line

**Aspen is ideal for:**

- Edge/P2P deployments requiring NAT traversal
- Rust-native applications needing distributed state
- Systems requiring deterministic testing
- Moderate-scale clusters (up to 64 nodes)
- Read-only SQL queries on KV data

**Consider alternatives when:**

- Massive scale required (>64 nodes) -> FoundationDB/CockroachDB
- Full SQL transactions needed -> CockroachDB
- Kubernetes-native configuration -> etcd
- Proven production track record critical -> Any of the above

---

## Sources

### FoundationDB

- [FoundationDB Official Documentation](https://apple.github.io/foundationdb/)
- [FoundationDB Paper (SIGMOD 2021)](https://www.foundationdb.org/files/fdb-paper.pdf)
- [FoundationDB Testing Approach](https://apple.github.io/foundationdb/testing.html)
- [XenonStack FoundationDB Architecture Guide](https://www.xenonstack.com/blog/foundationdb-architecture)

### etcd

- [etcd Official Documentation](https://etcd.io/)
- [etcd GitHub Repository](https://github.com/etcd-io/etcd)
- [Deep Dive into etcd (Medium)](https://medium.com/@extio/deep-dive-into-etcd-a-distributed-key-value-store-a6a7699d3abc)
- [etcd vs other key-value stores](https://etcd.io/docs/v3.4/learning/why/)

### CockroachDB

- [CockroachDB Architecture Overview](https://www.cockroachlabs.com/docs/stable/architecture/overview)
- [CockroachDB GitHub Repository](https://github.com/cockroachdb/cockroach)
- [CockroachDB Enterprise Architecture](https://www.cockroachlabs.com/blog/cockroachdb-enterprise-architecture/)
- [CockroachDB SIGMOD Paper](https://dl.acm.org/doi/pdf/10.1145/3318464.3386134)

### Deterministic Simulation Testing

- [DST Learning Resources](https://pierrezemb.fr/posts/learn-about-dst/)
- [Antithesis DST Overview](https://antithesis.com/resources/deterministic_simulation_testing/)
- [RisingWave DST Blog Series](https://www.risingwave.com/blog/deterministic-simulation-a-new-era-of-distributed-system-testing/)
- [Phil Eaton on DST](https://notes.eatonphil.com/2024-08-20-deterministic-simulation-testing.html)

### Iroh P2P

- [Iroh vs libp2p Comparison](https://www.iroh.computer/blog/comparing-iroh-and-libp2p)
- [Iroh GitHub Repository](https://github.com/n0-computer/iroh)
- [Iroh Documentation](https://www.iroh.computer/docs/overview)

### OpenRaft

- [OpenRaft GitHub Repository](https://github.com/databendlabs/openraft)
- [Raft Consensus Algorithm](https://raft.github.io/)
