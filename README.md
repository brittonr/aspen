# Aspen

A distributed orchestration layer for the Blixard ecosystem, providing fault-tolerant consensus, peer-to-peer networking, and actor-based coordination.

## Overview

Aspen is a foundational distributed systems framework written in Rust, drawing inspiration from Erlang/BEAM (actor supervision, fault tolerance), Plan 9 (distributed computing philosophy), Kubernetes (orchestration patterns), FoundationDB/etcd (distributed consensus), and Antithesis (deterministic testing).

### Key Features

- **Raft Consensus**: Cluster-wide linearizability via vendored OpenRaft v0.10.0
- **P2P Networking**: QUIC-based transport via Iroh with automatic peer discovery
- **Actor Supervision**: Automatic restart with exponential backoff and circuit breaker
- **Hybrid Storage**: redb for Raft log (append-optimized), SQLite for state machine (queryable)
- **Terminal UI**: Full-featured TUI for real-time monitoring and cluster management
- **Deterministic Testing**: Madsim-based simulation for reproducible distributed tests
- **Tiger Style**: Fixed resource limits, fail-fast semantics, explicit error handling

## Architecture

```
                              +-----------------+
                              |   HTTP API      |
                              | /health /write  |
                              | /read /metrics  |
                              +--------+--------+
                                       |
                    +------------------+------------------+
                    |                                     |
           +--------v--------+                   +--------v--------+
           | ClusterController|                   |  KeyValueStore  |
           |     Trait        |                   |     Trait       |
           +--------+---------+                   +--------+--------+
                    |                                      |
                    +------------------+-------------------+
                                       |
                              +--------v--------+
                              |   RaftActor     |
                              | (ractor Actor)  |
                              +--------+--------+
                                       |
              +------------------------+------------------------+
              |                        |                        |
     +--------v--------+      +--------v--------+      +--------v--------+
     |    OpenRaft     |      |   Storage Layer |      |  IRPC Network   |
     |   (Consensus)   |      |   redb + SQLite |      |  (Iroh QUIC)    |
     +--------+--------+      +-----------------+      +--------+--------+
              |                                                 |
              +-------------------------------------------------+
                                       |
                              +--------v--------+
                              |  Iroh Endpoint  |
                              | + Gossip + mDNS |
                              +-----------------+
```

### Data Flow

```
Write Request Flow:
==================

1. Client Request
   POST /write {"command": {"Set": {"key": "foo", "value": "bar"}}}
        |
        v
2. HTTP Handler
   Validates input (key <= 1KB, value <= 1MB)
        |
        v
3. RaftActor (via ractor message)
   raft.client_write(WriteCommand::Set{key, value})
        |
        v
4. OpenRaft Consensus
   - Leader: Appends to log, replicates to followers
   - Follower: Forwards to leader
        |
        v
5. Log Replication (IRPC over Iroh QUIC)
   AppendEntries RPC to each peer
        |
        v
6. Quorum Acknowledgment
   Majority responds -> entry committed
        |
        v
7. State Machine Apply (SQLite)
   INSERT OR REPLACE INTO kv_store(key, value)
        |
        v
8. Response to Client
   {"success": true, "log_id": {"term": 1, "index": 5}}


Read Request Flow:
==================

1. Client Request
   POST /read {"key": "foo"}
        |
        v
2. HTTP Handler -> RaftActor
        |
        v
3. Linearizable Read (ReadIndex)
   get_read_linearizer(ReadIndex).await
        |
        v
4. Leader Confirmation
   Verifies leadership via heartbeat round
        |
        v
5. State Machine Query (SQLite)
   SELECT value FROM kv_store WHERE key = ?
        |
        v
6. Response to Client
   {"value": "bar"}


Peer Discovery Flow:
====================

1. Node Startup
   aspen-node --node-id 1 --cookie my-cluster
        |
        v
2. Iroh Endpoint Created
   - Generates/loads Ed25519 keypair
   - Binds QUIC socket
        |
        v
3. Discovery Services (parallel)
   +-- mDNS (local network, enabled by default)
   +-- Gossip (topic from blake3(cookie))
   +-- DNS Discovery (production, optional)
   +-- Pkarr DHT (production, optional)
        |
        v
4. Peer Announcement (gossip every 10s)
   PeerAnnouncement { node_id, endpoint_addr, timestamp }
        |
        v
5. Automatic Connection
   network_factory.add_peer(node_id, endpoint_addr)
        |
        v
6. Raft Membership
   /add-learner then /change-membership
```

## Actor Model (Ractor)

Aspen uses the Ractor framework for fault-tolerant actor-based concurrency, inspired by Erlang/BEAM.

### RaftActor - Core Consensus Actor

The `RaftActor` owns the OpenRaft instance and processes messages sequentially:

```rust
// Message types handled by RaftActor
enum RaftActorMessage {
    Init { members: Vec<NodeId> },        // Initialize cluster
    AddLearner { node_id: NodeId },       // Add non-voting learner
    ChangeMembership { members: Set },    // Change voting members
    Write { command: WriteCommand },      // Replicated write
    Read { key: String },                 // Linearizable read
    CurrentState,                          // Query cluster state
}
```

**Key Properties:**

- **Isolated**: Each actor has its own mailbox (no shared memory)
- **Sequential**: Messages processed one at a time (no race conditions)
- **Bounded**: Mailbox capacity 1000 msgs (max 10000) for backpressure
- **Supervised**: Automatic restart on failure

### Actors in Aspen

Aspen uses a **hybrid actor-task architecture** where long-living actors manage subsystems and internally spawn lightweight tasks for I/O operations. This provides supervision benefits while maintaining performance.

#### Core Actors (Existing)

##### 1. **RaftActor** (Core Consensus)

- **Purpose**: Owns the OpenRaft instance, processes consensus operations
- **Spawned by**: RaftSupervisor (via `spawn_linked`)
- **Mailbox**: Bounded (1000-10000 messages)
- **Messages**: Init, AddLearner, ChangeMembership, Write, Read, CurrentState
- **Lifetime**: Restarted on failure by supervisor

##### 2. **RaftSupervisor** (Fault Tolerance)

- **Purpose**: Monitors and restarts RaftActor on failures
- **Spawned by**: bootstrap_node() during startup
- **Implements**: OneForOne supervision with exponential backoff
- **Health Check**: Every 5s, expects response within 25ms
- **Circuit Breaker**: Prevents restart storms (>3 restarts/10min)

##### 3. **NodeServer** (Actor System Root)

- **Purpose**: Ractor cluster node, manages actor hierarchy
- **Spawned by**: bootstrap_node() via `NodeServerConfig`
- **Transport**: Can use Iroh P2P or custom transports
- **Role**: Parent of all local actors, enables remoting

#### Long-Living Manager Actors (New Pattern)

These actors manage subsystems and internally spawn tasks for efficient I/O:

##### 4. **RaftRpcServerActor** (RPC Management) - NEW

- **Purpose**: Manages incoming Raft RPC connections with supervision
- **File**: `src/raft/server_actor.rs`
- **Tiger Style**: Bounded connections (MAX_CONCURRENT_CONNECTIONS)
- **Messages**:
  - `GetStats`: Connection statistics
  - `SetMaxConnections`: Update connection limit
  - `HealthCheck`: Server health status
- **Internal Tasks**: Connection handlers (tokio::spawn)

##### 5. **GossipActor** (Peer Discovery) - NEW

- **Purpose**: Manages gossip-based peer discovery with supervision
- **File**: `src/cluster/gossip_actor.rs`
- **Tiger Style**: Bounded peer list (MAX_PEER_COUNT = 1000)
- **Messages**:
  - `PeerDiscovered`: New peer found
  - `GetPeers`: List known peers
  - `GetStats`: Discovery statistics
  - `SetMaxPeers`: Update peer limit
- **Internal Tasks**: Announcer/receiver (tokio::spawn)

##### 6. **HealthMonitorActor** - PLANNED

- **Purpose**: Monitor Raft health with proper supervision
- **Tiger Style**: Bounded retries (MAX_HEALTH_RETRIES)
- **Messages**: `CheckHealth`, `GetStatus`

#### Architecture Pattern

```
RaftSupervisor (Actor)
├── RaftActor (Actor)
├── RaftRpcServerActor (Actor)
│   └── connection_tasks[] (tokio::spawn)
├── GossipActor (Actor)
│   ├── announcer_task (tokio::spawn)
│   └── receiver_task (tokio::spawn)
└── HealthMonitorActor (Actor)
```

**Pattern Benefits:**

- **Supervision**: Automatic restart on failure
- **Message APIs**: Clean, testable interfaces
- **State Management**: Centralized state ownership
- **Tiger Style**: All resources bounded
- **Performance**: Lightweight tasks for I/O

### Supervision with Circuit Breaker

The `RaftSupervisor` implements sophisticated fault tolerance:

```
Supervision States:
==================

Healthy (Normal Operation)
    ↓ (3 failed health checks)
Restart with Exponential Backoff
    [1s → 2s → 4s → 8s → 16s]
    ↓ (>3 restarts in 10 min)
Circuit OPEN (Meltdown)
    [No restarts for 5 min]
    ↓ (After cooldown)
Circuit HALF-OPEN
    [Try ONE restart]
    ↓
Success → Healthy
Failure → Circuit OPEN
```

**Health Monitoring:**

- Health check every 5 seconds
- 25ms timeout for actor response
- Status exposed via `/health` endpoint

## P2P Networking (Iroh)

Aspen uses Iroh for QUIC-based peer-to-peer networking with automatic discovery.

### Discovery Mechanisms

Nodes find each other automatically through multiple methods:

| Method | Default | Use Case | Description |
|--------|---------|----------|-------------|
| **mDNS** | ON | Local dev | Zero-config LAN discovery |
| **Gossip** | ON | All | Topic from blake3(cookie), announces every 10s |
| **DNS** | OFF | Production | Internet-scale via iroh.link |
| **Pkarr** | OFF | Production | DHT-based, decentralized |
| **Manual** | Always | Fallback | `--peers "2@endpoint_id:relay"` |

### IRPC Network Transport

Raft consensus messages flow over Iroh QUIC streams:

```
Raft RPC Flow:
=============

RaftActor → append_entries()
    ↓
IrpcRaftNetwork → get peer EndpointAddr
    ↓
Iroh Endpoint → QUIC connection
    ↓
IRPC Protocol → serialize & send
    ↓
Peer receives → deserialize
    ↓
Peer RaftActor → process & reply
```

**Connection Management:**

- Connection caching (reuses QUIC connections)
- Auto-reconnect on failure
- MAX_PEERS: 1000, MAX_STREAMS: 100 per connection
- 10-second RPC timeout

### Gossip Auto-Discovery

The gossip protocol enables automatic mesh formation:

```rust
// Every 10 seconds, announce presence:
PeerAnnouncement {
    node_id: u64,
    endpoint_addr: EndpointAddr,  // QUIC address + relay URL
    timestamp: u64,
}

// On receiving announcement:
if peer.node_id != self.node_id {
    network_factory.add_peer(peer.node_id, peer.endpoint_addr);
    // Automatic connection established!
}
```

### Integration Flow

```
Bootstrap Sequence:
==================

1. Start Node (aspen-node binary)
   ↓
2. Create Iroh Endpoint
   - Generate/load Ed25519 keypair
   - Bind QUIC socket (port auto-assigned)
   - Start discovery services (mDNS, DNS, Pkarr)
   ↓
3. Spawn NodeServer (Actor System Root)
   - ractor_cluster::NodeServer
   - Manages actor hierarchy
   - Enables actor remoting
   ↓
4. Spawn RaftSupervisor Actor
   - Monitors RaftActor health
   - Implements circuit breaker
   ↓
5. RaftSupervisor spawns RaftActor
   - Via spawn_linked (supervised)
   - Owns OpenRaft instance
   - Bounded mailbox (1000 msgs)
   ↓
6. Start Background Tasks
   - Health Monitor (tokio::spawn)
   - Gossip Announcer (10s interval)
   - Gossip Receiver (continuous)
   - IRPC Server (connection acceptor)
   ↓
7. HTTP API Ready
   - Routes to RaftActor via ractor messages
   - /health includes supervision status
```

Actor Hierarchy:

```
NodeServer (root)
    └── RaftSupervisor
            └── RaftActor (spawn_linked)

Background Tasks (tokio::spawn):
- Health Monitor
- Gossip Announcer
- Gossip Receiver
- IRPC Server
```

## Module Structure

```
src/
+-- api/               # Trait definitions
|   +-- mod.rs         # ClusterController, KeyValueStore traits
|   +-- inmemory.rs    # Deterministic in-memory implementations (testing)
|
+-- cluster/           # Cluster coordination
|   +-- bootstrap.rs   # Node startup orchestration
|   +-- config.rs      # Multi-layer configuration (env < TOML < CLI)
|   +-- metadata.rs    # Persistent node registry (redb-backed)
|   +-- ticket.rs      # Cluster join tickets (base32-encoded)
|   +-- gossip_discovery.rs  # Automatic peer discovery via iroh-gossip
|
+-- kv/                # Key-value service
|   +-- mod.rs         # KvServiceBuilder (fluent API)
|   +-- client.rs      # KvClient (delegates to RaftActor)
|   +-- types.rs       # NodeId newtype wrapper
|
+-- raft/              # Consensus engine
|   +-- mod.rs         # RaftActor, RaftControlClient
|   +-- constants.rs   # Tiger Style fixed limits
|   +-- storage.rs     # Hybrid storage (redb log + SQLite state machine)
|   +-- storage_sqlite.rs  # SQLite state machine implementation
|   +-- network.rs     # IRPC-based Raft RPC (vote, append, snapshot)
|   +-- server.rs      # IRPC server accepting Raft RPCs
|   +-- supervision.rs # Actor supervision with circuit breaker
|   +-- bounded_proxy.rs   # Backpressure via semaphore-bounded mailbox
|   +-- learner_promotion.rs  # Safe membership changes
|   +-- node_failure_detection.rs  # Distinguish crash vs network failure
|
+-- testing/           # Test infrastructure
|   +-- router.rs      # AspenRouter for deterministic multi-node tests
|
+-- simulation.rs      # Madsim artifact capture and persistence
+-- utils.rs           # Disk space checking (Tiger Style)
+-- lib.rs             # Public API exports
```

## Configuration

### Default Values

| Setting | Default | Description |
|---------|---------|-------------|
| `node_id` | (required) | Unique Raft node identifier (non-zero u64) |
| `data_dir` | `./data/node-{id}` | Persistent storage directory |
| `storage_backend` | `Sqlite` | Storage: `Sqlite`, `InMemory`, or `Redb` (deprecated) |
| `http_addr` | `127.0.0.1:8080` | HTTP control API address |
| `ractor_port` | `26000` | Ractor cluster port |
| `cookie` | `aspen-cookie` | Cluster authentication token |
| `heartbeat_interval_ms` | `500` | Raft heartbeat interval |
| `election_timeout_min_ms` | `1500` | Minimum election timeout |
| `election_timeout_max_ms` | `3000` | Maximum election timeout |
| `raft_mailbox_capacity` | `1000` | Bounded mailbox size (max: 10000) |

### Iroh Discovery Defaults

| Setting | Default | Description |
|---------|---------|-------------|
| `enable_gossip` | `true` | Automatic peer discovery via gossip |
| `enable_mdns` | `true` | Local network discovery (dev/testing) |
| `enable_dns_discovery` | `false` | DNS-based discovery (production) |
| `enable_pkarr` | `false` | DHT-based publishing (production) |

### Supervision Defaults

| Setting | Default | Description |
|---------|---------|-------------|
| `max_restart_count` | `3` | Max restarts per window |
| `restart_window_secs` | `600` | 10-minute window |
| `actor_stability_duration_secs` | `300` | 5 minutes uptime required |
| `circuit_open_duration_secs` | `300` | 5 minutes before retry |
| `half_open_stability_duration_secs` | `120` | 2 minutes for recovery |

### Tiger Style Resource Limits

All limits fixed at compile time in `src/raft/constants.rs`:

**Network:**

- `MAX_RPC_MESSAGE_SIZE`: 10 MB
- `MAX_SNAPSHOT_SIZE`: 100 MB
- `IROH_CONNECT_TIMEOUT`: 5 seconds
- `IROH_STREAM_OPEN_TIMEOUT`: 2 seconds
- `IROH_READ_TIMEOUT`: 10 seconds
- `MAX_CONCURRENT_CONNECTIONS`: 500
- `MAX_STREAMS_PER_CONNECTION`: 100
- `MAX_PEERS`: 1000

**Storage:**

- `MAX_KEY_SIZE`: 1 KB
- `MAX_VALUE_SIZE`: 1 MB
- `MAX_BATCH_SIZE`: 1000 entries
- `MAX_SETMULTI_KEYS`: 100 keys
- `MAX_SNAPSHOT_ENTRIES`: 1,000,000
- `DEFAULT_READ_POOL_SIZE`: 10 connections

**Cluster:**

- `MAX_VOTERS`: 100 nodes
- `LEARNER_LAG_THRESHOLD`: 100 entries
- `MEMBERSHIP_COOLDOWN`: 300 seconds

**Actor:**

- `DEFAULT_CAPACITY`: 1000 messages
- `MAX_CAPACITY`: 10000 messages
- `MAX_BACKOFF_SECONDS`: 16 seconds
- `MAX_RESTART_HISTORY_SIZE`: 100 entries

## Terminal UI (TUI)

Aspen includes a full-featured Terminal User Interface for cluster monitoring and management.

### Features

- **Real-time Monitoring**: Live cluster status, node health, and metrics
- **Multi-View Interface**: Tab between Cluster, Metrics, Key-Value, Logs, and Help views
- **Interactive Operations**: Initialize clusters, add learners, change membership
- **Key-Value Management**: Read and write operations with visual feedback
- **Multi-Node Support**: Connect to multiple nodes via HTTP or Iroh P2P
- **Log Integration**: View cluster logs with tracing support

### Starting the TUI

```bash
# Connect to local node (default: http://127.0.0.1:8080)
nix run .#aspen-tui

# Connect to multiple nodes
nix run .#aspen-tui -- --nodes http://127.0.0.1:8081 --nodes http://127.0.0.1:8082

# Connect via Iroh P2P with cluster ticket
nix run .#aspen-tui -- --ticket "aspen{base32-encoded-data}"

# Custom refresh interval (default: 1000ms)
nix run .#aspen-tui -- --refresh 500 --debug
```

### TUI Navigation

| Key | Action |
|-----|--------|
| `Tab` / `Shift+Tab` | Next/Previous view |
| `↑` / `↓` | Navigate node list |
| `Enter` | Select/Confirm |
| `i` | Initialize cluster (Cluster view) |
| `a` | Add learner (Cluster view) |
| `m` | Change membership (Cluster view) |
| `w` | Write key-value (KV view) |
| `r` | Read key (KV view) |
| `q` | Quit |
| `?` | Help |

### TUI Views

**Cluster View:**

```
┌─────────────── Nodes ────────────────┐ ┌──────── Node Details ─────────┐
│ [1] ● Leader    127.0.0.1:8081      │ │ Node ID: 1                    │
│ [2] ○ Follower  127.0.0.1:8082      │ │ State: Leader                 │
│ [3] ○ Follower  127.0.0.1:8083      │ │ Term: 5                       │
│ [4] ◐ Learner   127.0.0.1:8084      │ │ Applied: 1234                 │
│ [5] ✗ Offline   127.0.0.1:8085      │ │ Last Contact: 0.5s ago        │
└──────────────────────────────────────┘ └────────────────────────────────┘
```

**Metrics View:**

- Raft metrics: term, commit index, applied index
- Network stats: RPC latencies, message counts
- Storage metrics: log size, snapshot count
- Performance: operations/sec, latencies

**Key-Value View:**

- Interactive write/read operations
- Result display with syntax highlighting
- Operation history

**Logs View:**

- Scrollable log output
- Severity-based coloring
- Timestamp and module info

## Quick Start

### Building

```bash
# Build without S3 support (default)
nix develop -c cargo build --release

# Build with S3 support
nix develop -c cargo build --release --features s3

# Using Nix
nix build .#aspen          # Without S3
nix build .#aspen-full     # With S3 support
nix build .#aspen-s3       # Just the S3 server binary
```

### Optional Features

| Feature | Description               | Dependencies              |
|---------|---------------------------|---------------------------|
| `s3`    | S3-compatible API server  | s3s, s3s-aws, md5, hyper  |

When the `s3` feature is enabled, you can run an S3-compatible server:

```bash
# Build and run S3 server
nix develop -c cargo build --bin aspen-s3 --features s3
./target/release/aspen-s3 --node-id 1 --s3-addr 127.0.0.1:9000

# Or using Nix
nix run .#aspen-s3 -- --node-id 1 --s3-addr 127.0.0.1:9000
```

### Single Node

```bash
# Build
nix develop -c cargo build --release

# Start node
./target/release/aspen-node --node-id 1 --cookie dev-cluster

# Or start TUI for monitoring (in another terminal)
./target/release/aspen-tui

# Initialize cluster (in another terminal)
curl -X POST http://127.0.0.1:8080/init \
  -H "Content-Type: application/json" \
  -d '{"members":[1]}'

# Write data
curl -X POST http://127.0.0.1:8080/write \
  -H "Content-Type: application/json" \
  -d '{"command":{"Set":{"key":"hello","value":"world"}}}'

# Read data
curl -X POST http://127.0.0.1:8080/read \
  -H "Content-Type: application/json" \
  -d '{"key":"hello"}'
```

### Three-Node Cluster

```bash
# Terminal 1: Start node 1
./target/release/aspen-node --node-id 1 --http-addr 127.0.0.1:8081 --ractor-port 26001

# Terminal 2: Start node 2
./target/release/aspen-node --node-id 2 --http-addr 127.0.0.1:8082 --ractor-port 26002

# Terminal 3: Start node 3
./target/release/aspen-node --node-id 3 --http-addr 127.0.0.1:8083 --ractor-port 26003

# Initialize with node 1 as sole voter
curl -X POST http://127.0.0.1:8081/init -H "Content-Type: application/json" -d '{"members":[1]}'

# Get cluster ticket from node 1 for gossip
curl http://127.0.0.1:8081/cluster-ticket

# Add nodes 2 and 3 as learners
curl -X POST http://127.0.0.1:8081/add-learner -H "Content-Type: application/json" -d '{"node_id":2}'
curl -X POST http://127.0.0.1:8081/add-learner -H "Content-Type: application/json" -d '{"node_id":3}'

# Promote to voters
curl -X POST http://127.0.0.1:8081/change-membership -H "Content-Type: application/json" -d '{"members":[1,2,3]}'
```

### Using the Cluster Script

```bash
# Default: 3 nodes with in-memory storage
nix run .#cluster

# Custom configuration
ASPEN_NODE_COUNT=5 ASPEN_STORAGE=sqlite nix run .#cluster

# The script will print TUI connection instructions after startup:
# Example output:
# "Connect TUI with: nix run .#aspen-tui -- --nodes ..."
```

## HTTP API Reference

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Node health, leader, supervision status |
| `/metrics` | GET | Prometheus-format metrics |
| `/raft-metrics` | GET | Detailed Raft metrics (JSON) |
| `/leader` | GET | Current leader node ID |
| `/node-info` | GET | Node endpoint and network info |
| `/cluster-ticket` | GET | Generate cluster join ticket |
| `/init` | POST | Initialize cluster `{"members":[1]}` |
| `/add-learner` | POST | Add learner `{"node_id":2}` |
| `/change-membership` | POST | Change voters `{"members":[1,2,3]}` |
| `/add-peer` | POST | Manual peer addition |
| `/write` | POST | Write KV `{"command":{"Set":{"key":"k","value":"v"}}}` |
| `/read` | POST | Read KV `{"key":"k"}` |
| `/trigger-snapshot` | POST | Force Raft snapshot |

## Development

### Prerequisites

- Nix with flakes enabled
- Rust 2024 edition (via nix develop)

### Commands

```bash
# Enter development shell
nix develop

# Build
nix develop -c cargo build

# Run tests
nix develop -c cargo nextest run

# Format code
nix fmt

# Run clippy
nix develop -c cargo clippy --all-targets -- --deny warnings

# Run smoke tests
./scripts/aspen-cluster-smoke.sh       # Deterministic backend
./scripts/aspen-cluster-raft-smoke.sh  # Real Raft consensus
```

### Testing Philosophy

- **Integration over unit tests**: Real services, not mocks
- **Deterministic simulation**: Madsim for reproducible distributed tests
- **Property-based testing**: Proptest for invariant exploration
- **Simulation artifacts**: Captured to `docs/simulations/` for debugging

### Test Categories

| Category | Count | Framework |
|----------|-------|-----------|
| Madsim Integration | 9+ | madsim |
| Property-Based | 6+ | proptest |
| Router Tests | 25 | AspenRouter |
| Chaos/Failure | 6 | madsim |
| Integration | 8+ | tokio::test |

## Project Status

**Assessment Score**: 85/100 - Production-ready consensus layer

**Completed Phases:**

1. Core Building Blocks (modules, actor primitives)
2. Raft Integration (storage, actor, OpenRaft wiring)
3. Network Fabric (IRPC transport, gossip discovery)
4. Cluster Services (bootstrap, client API)
5. Documentation & Hardening (testing, CI, docs)

**Test Status**: 300+ tests passing (100% pass rate)

See `plan.md` for detailed implementation history and roadmap.

## Binaries

Aspen provides two main binaries:

| Binary | Purpose | Usage |
|--------|---------|-------|
| `aspen-node` | Cluster node daemon | `aspen-node --node-id 1 --cookie cluster-name` |
| `aspen-tui` | Terminal UI client | `aspen-tui --nodes http://localhost:8080` |

## Dependencies

**Core:**

- `openraft` v0.10.0 (vendored) - Raft consensus
- `iroh` v0.95.1 - P2P networking (QUIC)
- `iroh-gossip` v0.95 - Peer discovery
- `ractor` v0.15.9 - Actor framework
- `redb` v2.0 - Embedded storage (Raft log)
- `rusqlite` v0.37 - SQLite (state machine)
- `tokio` v1.48 - Async runtime

**TUI:**

- `ratatui` v0.28 - Terminal UI framework
- `crossterm` v0.28 - Terminal input/output
- `color-eyre` v0.6 - Error formatting
- `tui-logger` v0.13 - Log integration

**Testing:**

- `madsim` v0.2.34 - Deterministic simulation
- `proptest` v1.0 - Property-based testing
- `cargo-nextest` - Parallel test runner

## License

Apache-2.0 OR MIT (dual licensed)
