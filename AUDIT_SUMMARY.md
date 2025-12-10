# Aspen Codebase Audit - Quick Reference

## Overview

- **Total LOC**: 12,187 (main modules)
- **Modules**: 30+ files across 6 main packages
- **Language**: Rust 2024 Edition
- **Key Dependencies**: openraft (vendored), ractor, iroh, redb, rusqlite

## Module Breakdown

### 1. API Module (src/api/) - 368 LOC

**Core Traits**:

- `ClusterController`: init, add_learner, change_membership, get_metrics, trigger_snapshot
- `KeyValueStore`: write, read, delete

**Key Types**: ClusterNode, ClusterState, WriteCommand, ReadRequest/Result

**Implementations**:

- Real: RaftControlClient, KvClient
- Mock: DeterministicClusterController, DeterministicKeyValueStore

---

### 2. Raft Module (src/raft/) - ~8,500 LOC

| Component | LOC | Purpose |
|-----------|-----|---------|
| mod.rs | 687 | RaftActor, RaftControlClient |
| storage_sqlite.rs | 1,088 | SQLite state machine (prod) |
| supervision.rs | 1,580 | Actor restart + health monitoring |
| storage.rs | 948 | Backend abstraction (redb/SQLite) |
| learner_promotion.rs | 549 | Safe membership changes |
| node_failure_detection.rs | 577 | Crash vs network failure detection |
| bounded_proxy.rs | 716 | Mailbox backpressure via semaphore |
| madsim_network.rs | 496 | Deterministic testing network |
| network.rs | 408 | IRPC Raft network layer |
| storage_validation.rs | 564 | Pre-restart storage checks |
| server.rs | 262 | IRPC server for incoming RPCs |
| constants.rs | 244 | Tiger Style fixed limits |

**Key Features**:

- 3-phase supervision: health checks → auto-restart → circuit breaker
- Hybrid storage: redb (log) + SQLite (state machine)
- IRPC-based P2P networking over Iroh
- Deterministic testing with madsim
- All resource limits fixed at compile time

**Tiger Style Constants**:

- Network: MAX_RPC_MESSAGE_SIZE (10MB), IROH timeouts (5/2/10s), MAX_SNAPSHOT_SIZE (100MB)
- Storage: MAX_BATCH_SIZE (1000), MAX_SETMULTI_KEYS (100), MAX_KEY_SIZE (1KB), MAX_VALUE_SIZE (1MB)
- Concurrency: MAX_CONCURRENT_CONNECTIONS (500), MAX_STREAMS_PER_CONNECTION (100), MAX_PEERS (1000)
- Supervision: MAX_RESTART_HISTORY_SIZE (100), MAX_BACKOFF_SECONDS (16s)
- Cluster: MAX_VOTERS (100), LEARNER_LAG_THRESHOLD (100 entries), MEMBERSHIP_COOLDOWN (300s)

---

### 3. Cluster Module (src/cluster/) - ~2,200 LOC

| Component | LOC | Purpose |
|-----------|-----|---------|
| bootstrap.rs | 876 | Node startup orchestration |
| config.rs | 817 | Configuration types (env/TOML/CLI) |
| mod.rs | 652 | IrohEndpointManager |
| metadata.rs | 492 | Persistent node registry (redb) |
| gossip_discovery.rs | 406 | Gossip-based peer discovery |
| ticket.rs | 281 | Cluster join tickets |

**Configuration Precedence**:
Environment Variables < TOML File < CLI Arguments

**Bootstrap Sequence**:

1. Load configuration
2. Initialize metadata store (redb)
3. Create Iroh P2P endpoint
4. Initialize storage backends
5. Start ractor NodeServer
6. Create RaftActor with supervision
7. Register RPC endpoints
8. Start gossip discovery
9. Return BootstrapHandle

**Peer Discovery Options**:

- mDNS: Local network (LAN only)
- DNS: Production discovery via DNS service
- Pkarr: DHT-based distributed discovery
- Gossip: Broadcasts node_id + EndpointAddr every 10s
- Manual: Explicit --peers flag

---

### 4. Key-Value Service Module (src/kv/) - ~120 LOC

**API**:

- `KvServiceBuilder`: Fluent builder for node bootstrap
- `KvService`: Returned handle with shutdown support
- `KvClient`: Implements KeyValueStore, wraps RaftActor
- `NodeId`: Newtype wrapper preventing u64 confusion

**Example**:

```rust
let service = KvServiceBuilder::new(1, "./data/node-1")
    .with_storage(StorageBackend::Sqlite)
    .with_gossip(true)
    .start()
    .await?;
let client = service.client();
client.write(WriteRequest { command: Set { key, value } }).await?;
```

---

### 5. Binary Entry Point (src/bin/aspen-node.rs)

**HTTP Endpoints**:

- Control Plane: POST /cluster/init, /cluster/add-learner, /cluster/change-membership
- Key-Value: POST /kv/read, /kv/write, /kv/delete
- Monitoring: GET /health, /metrics, /raft-metrics, /cluster-ticket
- Admin: POST /admin/promote-learner

**Configuration**:

- CLI args (clap): --config, --node-id, --data-dir, --storage-backend
- TOML support
- Environment variable fallback

---

## Key Design Patterns

### Tiger Style Compliance

- **Explicit Types**: u64 for node IDs (not usize), NodeId newtype
- **Fixed Limits**: All constants at compile time
- **Fail Fast**: Disk checks before writes, config validation before startup
- **Bounded Resources**: Connection pools, queue capacities, peer map limits
- **Bounded Memory**: MAX_SNAPSHOT_SIZE (100MB), MAX_BATCH_SIZE (1000 entries)

### Supervision Pattern

```
Health Check (5s interval)
    ↓
RaftActor Ping (25ms timeout)
    ↓
Consecutive Failures (3+ consecutive)
    ↓
Exponential Backoff (1s → 2s → 4s → 8s → 16s)
    ↓
Storage Validation (before restart)
    ↓
Circuit Breaker (prevent restart loops)
```

### Error Handling

- **Library Layer**: snafu for explicit, contextual errors
- **Application Layer**: anyhow for general errors
- **HTTP Layer**: Semantic status codes + JSON error bodies

### Storage Architecture

```
Raft Log: redb (append-only optimized)
    ↓
State Machine: SQLite (queryable, ACID transactions)
    ├─ Connection Pool: r2d2 with 10 read connections
    └─ WAL Mode: Enabled for concurrency
```

---

## Observable Components

### Health Endpoint (GET /health)

```json
{
  "node_id": 1,
  "state": "Leader|Follower|Learner|Candidate",
  "current_leader": 1,
  "term": 42,
  "supervision": {
    "status": "healthy|degraded|unhealthy",
    "consecutive_failures": 0,
    "circuit_state": "Closed|Open|HalfOpen",
    "enabled": true
  }
}
```

### Metrics

- RaftActor: messages sent/rejected, operation latency, responsiveness
- Supervision: restart count, meltdown detection, circuit state
- Storage: write latency, read latency, snapshot size
- Network: RPC latency by peer, connection establishment, failure detection

---

## Error Types Summary

### ControlPlaneError

- `InvalidRequest { reason }`: Validation failed
- `NotInitialized`: Cluster not initialized
- `Failed { reason }`: Operation failed

### KeyValueStoreError

- `NotFound { key }`: Key missing (idempotent)
- `KeyTooLarge { size, max }`: Exceeds MAX_KEY_SIZE (1KB)
- `ValueTooLarge { size, max }`: Exceeds MAX_VALUE_SIZE (1MB)
- `BatchTooLarge { size, max }`: Exceeds MAX_SETMULTI_KEYS (100)
- `Timeout { duration_ms }`: Operation exceeded timeout

### SqliteStorageError (snafu)

- `OpenDatabase`, `Execute`, `Query`, `Serialize`, `Deserialize`
- `JsonSerialize`, `JsonDeserialize`, `IoError`
- `DiskSpaceInsufficient`: >95% disk usage

---

## Module Dependencies

```
lib.rs
├─ api/                (Traits: ClusterController, KeyValueStore)
├─ raft/               (Consensus: RaftActor, RaftControlClient)
├─ cluster/            (Coordination: bootstrap, config, IrohManager)
├─ kv/                 (High-level: KvServiceBuilder, KvClient)
├─ testing/            (AspenRouter for in-memory tests)
└─ utils.rs            (Disk space checks)

bin/aspen-node.rs
├─ api::*              (Traits + types)
├─ cluster::bootstrap  (Node initialization)
├─ raft::*             (Control and data plane)
└─ Axum                (HTTP server)
```

---

## Entry Points

| Entry Point | Usage | Configuration |
|-------------|-------|---|
| `aspen-node` binary | Production cluster node with HTTP API | TOML + CLI args |
| `KvServiceBuilder` | Programmatic node bootstrap | Rust builder API |
| `bootstrap_node()` | Low-level bootstrap | ClusterBootstrapConfig struct |
| `DeterministicController` | Mock control plane (tests) | In-memory Arc<Mutex> |
| `AspenRouter` | In-memory multi-node test harness | Simulated networking |

---

## Strengths & Achievements

✓ **Type Safety**: Explicit types prevent bugs at compile time
✓ **Predictability**: Fixed resource bounds ensure deterministic behavior
✓ **Resilience**: Multi-layer health monitoring with automatic recovery
✓ **Flexibility**: Narrow trait interfaces enable testing and pluggability
✓ **Observability**: Health endpoints, metrics, supervision tracking
✓ **Testability**: madsim for deterministic simulation, in-memory mocks

---

## Key Technical Achievements

1. **Full Raft Implementation**: Consensus engine from scratch on openraft v0.10.0
2. **Multi-level Supervision**: Health checks → auto-restart → circuit breaker
3. **Hybrid Storage**: Separate log (redb) and state machine (SQLite) backends
4. **P2P Networking**: IRPC-based Raft communication over Iroh QUIC
5. **Deterministic Testing**: madsim integration for reproducible distributed tests
6. **Configuration Management**: Multi-layer precedence (env < TOML < CLI)
7. **Node Discovery**: Gossip protocol + manual peer configuration
8. **Cluster Tickets**: Easy joining mechanism for new nodes

---

## Files Location

Complete audit report: `/home/brittonr/git/aspen/ASPEN_AUDIT_REPORT.txt`
