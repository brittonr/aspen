# Aspen

Aspen is a hybrid-consensus distributed systems framework in Rust, built on top of [Iroh](https://github.com/n0-computer/iroh). It combines **local-first Raft consensus** for strong coordination within a cluster with **eventual, peer-to-peer convergence** (via CRDTs / iroh-docs / Automerge) for global state across clusters. It also supports **federated cluster-to-cluster replication**.

Built on Iroh (QUIC-based P2P networking with NAT traversal), a vendored OpenRaft for consensus, and a FoundationDB-inspired "unbundled database" philosophy where higher-level features (Git forge, CI/CD, secrets, DNS) are stateless layers over core KV + blob primitives.

~519k lines of Rust across 91 workspace crates (+65k vendored OpenRaft).

---

## Table of Contents

- [Architecture](#architecture)
- [Build](#build)
- [Run](#run)
- [Design Decisions](#design-decisions)
- [Consensus: Raft + Eventual Convergence](#consensus-raft--eventual-convergence)
- [Storage Architecture](#storage-architecture)
- [Networking & Transport (Iroh)](#networking--transport-iroh)
- [Core KV](#core-kv)
- [Cluster Management & Bootstrap](#cluster-management--bootstrap)
- [Coordination Primitives](#coordination-primitives)
- [Plugins](#plugins)
- [Federation](#federation)
- [Sharding](#sharding)
- [Forge (Git Hosting)](#forge-git-hosting)
- [CI/CD Pipeline](#cicd-pipeline)
- [Secrets Management](#secrets-management)
- [Auth: Capability Tokens (UCAN)](#auth-capability-tokens-ucan)
- [Job Queue System](#job-queue-system)
- [Additional Subsystems](#additional-subsystems)
- [RPC Handler Architecture](#rpc-handler-architecture)
- [Feature Flags](#feature-flags)
- [Testing](#testing)
- [Design Philosophy](#design-philosophy)
- [Crate Map](#crate-map)
- [License](#license)

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                      APPLICATION LAYER                               │
│  Forge (Git)  │  CI/CD  │  Secrets  │  DNS  │  Automerge  │  FUSE  │
├─────────────────────────────────────────────────────────────────────┤
│                    PLUGIN / HANDLER LAYER                            │
│  WASM Plugins  │  Native Handlers  │  HandlerRegistry (ArcSwap)     │
├─────────────────────────────────────────────────────────────────────┤
│                   COORDINATION LAYER                                 │
│  Locks  │  Elections  │  Counters  │  Queues  │  Barriers  │  Leases│
├─────────────────────────────────────────────────────────────────────┤
│                    CORE PRIMITIVES                                    │
│  KV Store (Raft)  │  Blob Store (iroh-blobs)  │  Docs (iroh-docs)  │
├─────────────────────────────────────────────────────────────────────┤
│                    CONSENSUS & STORAGE                               │
│  OpenRaft (vendored)  │  Redb (single-fsync)  │  Write Batcher     │
├─────────────────────────────────────────────────────────────────────┤
│                    TRANSPORT & DISCOVERY                              │
│  Iroh (QUIC/P2P)  │  Gossip  │  mDNS  │  Pkarr DHT  │  Federation │
└─────────────────────────────────────────────────────────────────────┘
```

Everything above the "Core Primitives" line is a **stateless layer** — it stores its data in the KV store and blob store. This is the FoundationDB "layer" philosophy: the database provides ordered KV + transactions, and everything else (SQL, documents, queues) is built on top without modifying the core.

---

## Build

```bash
# Optional: enter the Nix dev shell
nix develop

cargo build
```

## Run

```bash
# Node binary (requires features)
cargo run -p aspen --bin aspen-node --features "jobs,docs,blob,hooks" -- --node-id 1 --cookie dev

# CLI and TUI
cargo run -p aspen-cli -- --help
cargo run -p aspen-tui -- --help
```

---

## Design Decisions

### Hybrid Consensus Model

- **Within a cluster**: Strong consistency via Raft (linearizable reads/writes)
- **Across clusters**: Eventual consistency via pull-based federation with signature verification
- Most operations need strong consistency (KV writes, coordination), but cross-org federation can tolerate eventual consistency. This avoids the impossibility of strongly consistent global consensus while still enabling multi-cluster collaboration.

### Iroh as Exclusive Transport

- All inter-node communication uses Iroh (QUIC-based P2P)
- NAT traversal, relay servers, hole-punching built-in
- **No HTTP/DNS required** for cluster operation — fully P2P
- ALPN-based protocol multiplexing on a single QUIC endpoint
- A single QUIC endpoint multiplexes Raft, client RPC, blobs, gossip, and federation — no port-per-service complexity
- **Trade-off**: Tightly couples to Iroh's API. Mitigated by vendoring and abstraction layers.

### FoundationDB-Inspired Layers

- **Tuple encoding**: Order-preserving serialization of composite keys (FDB Tuple Layer spec)
- **Subspace isolation**: Namespace partitioning for multi-tenant workloads
- **Directory layer**: High-contention allocator for subspace management
- Proven pattern from FoundationDB for building complex data models on ordered KV. Every higher-level feature (forge, CI, secrets, coordination) is a "layer" that only uses KV + CAS + scan — never modifying the consensus engine. This enables independent development, testing, and plugin-based extensibility.

### Vendored OpenRaft

- Full copy of OpenRaft in `openraft/` directory (not a git submodule)
- Allows patching consensus behavior without upstream changes
- Workspace dependencies aligned between Aspen and OpenRaft
- Custom Raft behavior (single-fsync storage, custom type configs). The type transmute between `aspen-transport` and `aspen-raft` requires structurally identical type configs verified at compile time.
- **Trade-off**: Carries maintenance burden of vendored code. Worth it for control over consensus internals.

### Single-Fsync Storage (Redb)

- Traditional Raft: 2 fsyncs (log append + state machine apply)
- Aspen: 1 fsync via `SharedRedbStorage` — both in a single Redb transaction
- Halves write latency. Redb's ACID transactions guarantee atomicity across log + state machine in one commit. This is the single biggest performance optimization in the write path.
- **Trade-off**: Couples log and state machine into one database. Acceptable because Redb supports ordered keys (needed for both) and the coupling simplifies crash recovery.

### Write Batching (Group Commit)

- Multiple concurrent writes batched into a single Raft proposal + single fsync
- Configurable batch window (1ms ≈ 10× throughput, 5ms ≈ 30× throughput)
- Without batching, each write pays the full fsync cost (~3.2ms). With batching, N writes amortize to one fsync.
- **Trade-off**: Adds latency equal to the batch window. Configurable per-deployment.

### Raft Type Transmute

- `aspen-transport` and `aspen-raft` define structurally identical Raft type configs
- `transmute_raft_for_transport()` safely transmutes between them
- Safety verified at compile time via `static_assertions`
- Breaks a circular dependency — `aspen-transport` needs Raft types but can't depend on `aspen-raft` (which depends on `aspen-transport`). The transmute is centralized, documented, and would fail to compile if types ever diverged.

### Content-Addressed Blob Storage

- All blobs (git objects, CI artifacts, WASM binaries) stored via iroh-blobs with BLAKE3 content hashing
- Automatic deduplication across all subsystems
- P2P transfer with garbage collection protection tags
- The same blob store serves git objects, CI artifacts, plugin binaries, and Nix store paths.

### Handler Self-Registration (`inventory` crate)

- Handler crates use the `inventory` crate for compile-time self-registration
- `HandlerFactory` implementations are collected automatically at link time
- `ArcSwap` enables lock-free handler hot-reload without node restart
- Adding a new handler crate requires zero changes to the dispatcher — just implement `HandlerFactory`, register via `inventory::submit!`, and it's discoverable.

### Configuration-Driven Bootstrap

- Nodes bootstrap via `bootstrap_node(config: NodeConfig)` — a configuration struct, not a builder
- `NodeConfig` merges CLI args, env vars, and config files with explicit priority
- Configuration structs are serializable (for persistence/debugging), testable (for unit tests), and declarative (for NixOS integration).

### Resource Group Pattern (NodeHandle)

- `NodeHandle` composes resources into typed groups: `StorageResources`, `NetworkResources`, `DiscoveryResources`, `SyncResources`, `WorkerResources`, `BlobReplicationResources`, `HookResources`, `ShutdownCoordinator`
- 10-phase ordered shutdown: signal → workers → hooks → blob replication → discovery → sync → storage → shutdown coordinator → network → metadata

---

## Consensus: Raft + Eventual Convergence

### Raft Implementation

- **Based on**: OpenRaft (vendored, v0.10.0)
- **RaftNode**: Direct async wrapper around OpenRaft (no actor message passing)
- **Implements**: Both `ClusterController` and `KeyValueStore` traits

### Key Raft Configuration

| Parameter | Default | Purpose |
|-----------|---------|---------|
| Heartbeat interval | 500ms | Leader liveness signal |
| Election timeout min | 1500ms | Follower wait before candidacy |
| Election timeout max | 3000ms | Randomized upper bound |
| Max batch size | 1000 | Bounded Raft proposals |
| Max snapshot size | 100MB | Bounded state transfer |
| Max RPC message size | 10MB | Bounded message payloads |
| Iroh connect timeout | 5s | Peer connection establishment |
| Iroh stream open timeout | 2s | Bidirectional stream setup |
| Iroh read timeout | 10s | RPC response read |
| Max peers | 1000 | Peer map bounds |

### Storage: Single-Fsync Redb

- **Redb** (embedded key-value store) for both Raft log AND state machine
- Single fsync covers both log append + state machine apply
- `SharedRedbStorage` struct manages both in a single database
- **Alternative**: `InMemory` backend for testing (no persistence)
- Storage backend selected via `StorageBackend` enum: `Redb` (default) or `InMemory`

### Network Layer

- `IrpcRaftNetwork`: Raft network over IRPC (Iroh RPC) over QUIC
- Connection pooling for efficient peer communication
- Node failure detection distinguishes transient errors from node-level failures
- Clock drift detection monitors time synchronization

### Authenticated Raft

- Two ALPN protocols: `raft-rpc` (legacy, deprecated) and `raft-auth` (authenticated)
- `TrustedPeersRegistry` synced with Raft membership via `MembershipWatcher`
- Only members' PublicKeys (from Iroh identity) can send Raft RPCs

---

## Storage Architecture

### Redb (Primary)

- Embedded, ACID, zero-copy key-value store
- Used for: Raft log, state machine, metadata
- Single database file per node
- Supports range scans (ordered keys)

### Blob Store (iroh-blobs)

- Content-addressed storage (BLAKE3 hashing)
- P2P transfer via Iroh
- Used by: Forge (git objects), CI (artifacts), plugins (WASM binaries), Nix (NAR archives)
- Garbage collection with protection tags
- Replication across cluster nodes

### Iroh-Docs (CRDTs)

- Document-level CRDT synchronization
- Used for: eventually-consistent replicated documents
- Integrates with Automerge for collaborative editing

### Hybrid Logical Clocks (HLC)

- `aspen-hlc` crate wraps the `uhlc` library
- Deterministic node ID from blake3 hash of node identifier
- Total ordering even when wall clocks are equal
- Used for: conflict resolution, causal consistency

---

## Networking & Transport (Iroh)

### Protocol Multiplexing

All protocols share one QUIC endpoint via ALPN:

| ALPN | Protocol | Purpose |
|------|----------|---------|
| `raft-rpc` | Raft (legacy) | Unauthenticated Raft RPCs (deprecated) |
| `raft-auth` | Raft (authenticated) | Membership-verified Raft RPCs |
| `aspen-client` | Client RPC | CLI/app connections |
| `iroh-blobs/0` | Blobs | P2P blob transfer |
| `iroh-gossip/0` | Gossip | Peer discovery/announcements |
| `/aspen/federation/1` | Federation | Cross-cluster communication |
| `raft-shard` | Sharded Raft | Multi-shard Raft RPCs |
| `aspen-logs` | Log subscription | Real-time Raft log streaming |
| `iroh+h3` | Nix cache | HTTP/3 gateway for NAR archives |

### Peer Discovery Stack

1. **mDNS** (default): LAN discovery
2. **DNS Discovery** (opt-in): Production bootstrap
3. **Pkarr DHT** (opt-in): BitTorrent Mainline DHT for decentralized discovery
4. **Gossip** (default): Broadcasts node_id + EndpointAddr periodically
5. **Manual peers** (fallback): Explicit `node_id@endpoint_id` configuration
6. **Cluster Tickets**: Compact bootstrap info (`aspen{...}` format)

### Connection Management

- `ConnectionManager`: Bounded concurrent connections with permits
- `StreamManager`: Bounded concurrent streams with permits
- `RaftConnectionPool`: Reuses connections for Raft RPCs

---

## Core KV

Aspen ships a core key-value store backed by Raft for strongly consistent reads/writes within a cluster, then converges via peer-to-peer replication across clusters.

### Data Model

- Keys: strings with hierarchical namespacing (e.g., `forge:repos:myrepo:refs/heads/main`)
- Values: strings (JSON, base64-encoded binary, plain text)
- Versioned entries: `KeyValueWithRevision` tracks version, create_revision, mod_revision

### Operations

| Operation | Consistency | Description |
|-----------|-------------|-------------|
| Get | Linearizable (via Raft) | Read single key |
| Set | Linearizable | Write single key |
| Delete | Linearizable | Remove key |
| CAS | Linearizable | Compare-and-swap (optimistic concurrency) |
| CAD | Linearizable | Compare-and-delete |
| Scan | Linearizable | Prefix scan with pagination |
| BatchRead | Linearizable | Multi-key read |
| BatchWrite | Linearizable | Atomic multi-key write |
| Transactions | Linearizable | etcd-style If/Then/Else transactions |

### Scan Pagination

- Continuation tokens (base64-encoded last key)
- Default scan limit: 1,000 / Maximum: 10,000
- `normalize_scan_limit()` clamps to bounds (verified in Verus)
- Bounded batch operations: `MAX_BATCH_SIZE` = 1,000

### Leases (TTL)

- `grant_lease(ttl_seconds)` → lease_id
- Keys attached to leases auto-expire
- `keepalive_lease` / `revoke_lease`
- Background `ttl_cleanup` and `lease_cleanup` tasks

### SQL Layer (Optional)

- Apache DataFusion for read-only SQL over Redb KV data
- Feature-gated behind `sql`
- Virtual table over KV entries with `key`, `value` columns
- Supports WHERE, ORDER BY, LIMIT, COUNT, JOINs

---

## Cluster Management & Bootstrap

### Node Bootstrap Sequence

`bootstrap_node(config: NodeConfig)` executes 8 phases:

1. Initialize metadata store (node registry in Redb)
2. Create Iroh P2P endpoint
3. Start Raft consensus with configured storage backend
4. Register protocol handlers (Raft, Client, Gossip, Federation...)
5. Start gossip discovery
6. Join/form cluster via tickets or manual peers
7. Register metadata and capabilities
8. Assemble `NodeHandle` from initialized resources

### Example

```rust
let config = NodeConfig {
    node_id: NodeId(1),
    data_dir: "./data/node-1".into(),
    storage_backend: StorageBackend::Redb, // or InMemory for tests
    heartbeat_interval_ms: 500,
    election_timeout_min_ms: 1500,
    election_timeout_max_ms: 3000,
    cookie: Some("my-cluster".into()),
    gossip_enabled: true,
    ..NodeConfig::default()
};
let handle = bootstrap_node(config).await?;
handle.spawn_router(); // Registers ALPN handlers
```

### NodeHandle Resource Groups

| Resource Group | Contents |
|---------------|----------|
| `storage` | `raft_node`, state machine variant |
| `network` | `iroh_manager`, optional `blob_store` |
| `discovery` | Peer/content discovery handles |
| `sync` | Document synchronization |
| `worker` | Job execution resources |
| `blob_replication` | Blob replication manager |
| `hooks` | Event hook service |
| `shutdown` | Shutdown coordinator |

### Graceful Shutdown (10 phases)

1. Signal shutdown to all components
2. Stop workers (no new jobs)
3. Stop hooks (event bridge)
4. Stop blob replication
5. Stop discovery
6. Stop document sync
7. Stop storage TTL cleanup
8. Stop shutdown supervisor
9. Close network connections (last)
10. Update metadata status

---

## Coordination Primitives

Built on CAS operations over the KV store, providing linearizable semantics through Raft:

| Primitive | Description |
|-----------|-------------|
| `DistributedLock` | Mutual exclusion with fencing tokens |
| `LeaderElection` | Automatic lease renewal, fencing tokens |
| `AtomicCounter` | Race-free increment/decrement |
| `SequenceGenerator` | Monotonically increasing unique IDs |
| `DistributedRateLimiter` | Token bucket rate limiting |
| `QueueManager` | FIFO with visibility timeout, DLQ, dedup |
| `ServiceRegistry` | Service discovery with health checks |
| `BarrierManager` | Distributed barrier synchronization |
| `DistributedSemaphore` | Bounded concurrent access |
| `DistributedRWLock` | Multiple readers / exclusive writer |
| `BufferedCounter` | Local buffering for high-throughput counting |

All primitives are "userspace" — they don't modify the Raft engine. They use `write` + `cas` + `scan` operations on specially-prefixed keys. A distributed lock is just a KV entry with a CAS-guarded owner field and a lease. Leader election is a periodic CAS loop that refreshes a TTL key.

---

## Plugins

Aspen includes a WASM plugin system for extending clusters with custom request handlers.
Plugins run sandboxed in hyperlight-wasm with capability-based permissions, KV namespace
isolation, and Ed25519 signing.

See [Plugin Development Guide](docs/PLUGIN_DEVELOPMENT.md) for building your own plugins.

### Three-Tier Architecture

1. **Native Handlers**: Compiled Rust, linked at build time (`inventory` crate for self-registration)
2. **WASM Plugins**: Hyperlight-wasm sandboxes, hot-reloadable, least-privilege
3. **VM Plugins**: Hyperlight micro-VMs for native binaries (highest isolation)

### WASM Plugin Lifecycle

1. `PluginRegistry::load_all` scans KV for plugin manifests (`plugins/handlers/` prefix)
2. WASM bytes fetched from blob store via manifest's `wasm_hash`
3. Hyperlight-wasm sandbox created with host functions
4. Guest `plugin_info` export called to validate manifest
5. `WasmPluginHandler` wraps sandbox as `RequestHandler`

### Plugin Permissions (Least Privilege)

```rust
struct PluginPermissions {
    kv_read: bool,
    kv_write: bool,
    blob_read: bool,
    blob_write: bool,
    cluster_info: bool,
    randomness: bool,
    signing: bool,
    timers: bool,
    hooks: bool,
}
```

### KV Namespace Isolation

- Each plugin gets a key prefix (e.g., `forge:`, `__hooks:`, `__secrets:`)
- Host functions validate all KV operations against `allowed_kv_prefixes`
- Empty manifest prefixes → auto-scoped to `__plugin:{name}:`

### Hot Reload

- `HandlerRegistry` uses `ArcSwap` for lock-free handler updates
- `PluginReload` request triggers re-scan + re-load without node restart

### Existing WASM Plugins

| Plugin | Prefix | Crate |
|--------|--------|-------|
| `forge` | `forge:` | `aspen-forge-plugin` |
| `hooks` | `__hooks:` | `aspen-hooks-plugin` |
| `service-registry` | `__service:` | `aspen-service-registry-plugin` |
| `secrets` | `__secrets:` | `aspen-secrets-plugin` |
| `automerge` | `automerge:` | `aspen-automerge-plugin` |

---

## Federation

Independent Aspen clusters can discover each other, share content, and synchronize
resources across organizational boundaries — without HTTP, DNS, or any central
authority. Federation is built on Ed25519 cluster identities, DHT-based discovery
(BitTorrent Mainline BEP-44), rate-limited gossip, and a QUIC-based sync protocol
with three-layer cryptographic verification.

See [Federation Guide](docs/FEDERATION.md) for the full architecture and API reference.

### Core Concepts

1. **Cluster Identity**: Ed25519 keypair per cluster (persists across node changes)
2. **Federated IDs**: `origin_cluster_key:local_id` for global uniqueness
3. **Pull-Based Sync**: Eventual consistency with signature verification

### Trust Model

- **Public**: Anyone can discover and sync
- **AllowList**: Only explicitly trusted clusters
- All data verified: cluster signatures, delegate signatures, content hashes

### Discovery

- BitTorrent Mainline DHT (BEP-44) for decentralized cluster discovery
- Gossip for real-time announcements
- No central relay needed

### Resource Bounds

| Resource | Limit | Constant |
|----------|-------|----------|
| Apps per cluster | 32 | `MAX_APPS_PER_CLUSTER` |
| Capabilities per app | 16 | `MAX_CAPABILITIES_PER_APP` |
| Tracked clusters (discovery) | 1,024 | `discovery::MAX_TRACKED_CLUSTERS` |
| Tracked clusters (gossip) | 512 | `gossip::MAX_TRACKED_CLUSTERS` |
| Gossip rate per cluster | 12/min | `gossip::CLUSTER_RATE_PER_MINUTE` |
| Global gossip rate | 600/min | `gossip::GLOBAL_RATE_PER_MINUTE` |

---

## Sharding

- **Jump Consistent Hash**: Uniform key distribution across shards
- **Max shards**: 256 (`MAX_SHARDS`, Tiger Style bound)
- Each shard = independent Raft cluster

### Components

- `ShardRouter`: Routes keys to shards via jump consistent hash
- `ShardedKeyValueStore`: Wraps multiple `KeyValueStore` implementations
- `ShardTopology`: Manages shard state and range assignments
- `ShardMetricsCollector`: Per-shard metrics
- `ShardAutomationManager`: Background split/merge automation

### Architecture

```
Client Request (key: "user:123")
       ↓
ShardRouter.get_shard_for_key("user:123")
       ↓
Returns ShardId = 2 (Jump consistent hash)
       ↓
ShardedKeyValueStore.shards[2].write(request)
       ↓
Individual RaftNode handles the operation
       ↓
Communication via "raft-shard" ALPN
```

---

## Forge (Git Hosting)

### Architecture: Three Layers

```
IMMUTABLE LAYER (iroh-blobs)
  Git Objects │ COB Changes │ Signed Attestations
  → BLAKE3 content-addressed hashes

MUTABLE LAYER (Raft KV)
  refs/heads/main → Hash
  cobs/issue/{id}:heads → [Hash]

DISCOVERY LAYER
  iroh-gossip (announcements) + DHT (find seeders)
```

### Key Components

- **Git Objects**: Commits, trees, blobs stored in iroh-blobs
- **Collaborative Objects (COBs)**: Issues, patches, reviews as immutable DAGs
- **Refs**: Branch/tag storage via Raft (strongly consistent)
- **Git Bridge**: Bidirectional sync with GitHub/GitLab/Gitea via `git-remote-aspen` helper
- **Pijul**: Alternative patch-based VCS (via libpijul, GPL-2.0-or-later)

### Design: Radicle-Inspired

- Decentralized, no central server
- Delegates sign canonical refs
- Content flows via P2P (iroh-blobs + gossip)
- Identities are Ed25519 keys (reuses Iroh identity)

---

## CI/CD Pipeline

### Configuration

- **Nickel** (.ncl) for type-safe pipeline definitions with contracts
- Stored in `.aspen/ci.ncl` per repository

### Execution Backends

| Executor | Crate | Isolation |
|----------|-------|-----------|
| Shell | `aspen-ci-executor-shell` | Process-level |
| Nix | `aspen-ci-executor-nix` | Nix sandbox |
| VM | `aspen-ci-executor-vm` | Cloud Hypervisor |

### Integration Points

- **Gossip Triggers**: Automatic builds on ref updates
- **Distributed Execution**: Jobs run across cluster via `aspen-jobs`
- **Artifact Storage**: Build outputs in iroh-blobs (P2P)
- **Nix Binary Cache**: SNIX integration with HTTP/3 gateway (`iroh+h3` ALPN) for NAR archives

---

## Secrets Management

### Two Modes

1. **Bootstrap (SOPS)**: Encrypted secrets loaded at startup via age encryption
2. **Runtime (Vault-like)**: Dynamic secrets engines

### Secrets Engines

| Engine | Description |
|--------|-------------|
| **KV** | Versioned key-value with soft/hard delete |
| **Transit** | Encryption as a service (encrypt, decrypt, sign, verify) |
| **PKI** | Certificate authority with role-based policies |

### Mount Registry

- Dynamic multi-mount support (like HashiCorp Vault)
- Each mount has isolated storage with prefixed keys
- `MountRegistry` creates stores on-demand

---

## Auth: Capability Tokens (UCAN)

### Design Principles

1. **Reuses Iroh identity**: `NodeId` = Ed25519 public key
2. **Self-contained tokens**: No database lookup for authorization
3. **Delegation**: Tokens can create child tokens with fewer permissions
4. **Offline verification**: Works without contacting the cluster

### Capabilities

```rust
enum Capability {
    Full { prefix: String },   // Full access to key prefix
    Read { prefix: String },   // Read-only access
    Write { prefix: String },  // Write-only access
    Delete { prefix: String }, // Delete access
    Delegate,                  // Can create child tokens
}
```

- `MAX_CAPABILITIES_PER_TOKEN` = 32 (Tiger Style bound, verified in Verus)

### Token Verification

```rust
let verifier = TokenVerifier::new();
verifier.authorize(
    &token,
    &Operation::Write { key: "myapp:data".into(), value: vec![] },
    None,
)?;
```

### HMAC Auth

- Separate `hmac_auth` module for Raft RPC authentication
- Cluster cookie used as shared secret for HMAC-SHA256

---

## Job Queue System

### Features

- Priority queues (High, Normal, Low)
- Retry with exponential backoff
- Dead letter queue (DLQ)
- Job dependencies and workflows (DAGs)
- Saga pattern for distributed transactions
- Cron-based scheduling
- Worker affinity

### Worker Types

| Worker | Crate | Purpose |
|--------|-------|---------|
| `BlobWorker` | `aspen-jobs-worker-blob` | Blob operations |
| `ReplicationWorker` | `aspen-jobs-worker-replication` | Data replication |
| `SqlWorker` | `aspen-jobs-worker-sql` | SQL query execution |
| `MaintenanceWorker` | `aspen-jobs-worker-maintenance` | Cluster maintenance |
| `ShellWorker` | `aspen-jobs-worker-shell` | System command execution |

### Architecture

- `JobManager`: Submits/tracks jobs via KV store
- `WorkerPool`: Routes jobs to workers by `job_types()`
- `WorkerCoordinator`: Distributes workers across cluster
- `DurableTimer`: Persistent timers for delayed/cron jobs
- `EventStore`: Job event history for replay
- `SagaOrchestrator`: Compensating transactions

---

## Additional Subsystems

### DNS (`aspen-dns`)

- DNS record management with CRUD operations
- Wildcard resolution, zone management
- Hickory-based DNS protocol server

### Automerge CRDTs (`aspen-automerge`)

- Document CRUD with automatic merge of concurrent changes
- Full change history preserved
- Sync protocol with capability-based auth (`AutomergeSyncTicket`)

### FUSE Filesystem (`aspen-fuse`)

- Mount Aspen KV store as a POSIX filesystem
- Read cache (data: 5s TTL, metadata: 2s, scans: 1s)
- Connection pooling for QUIC connections
- VirtioFS integration for VM guests

### Nix Integration (`aspen-snix`)

- Nix binary cache store using SNIX (Rust Nix implementation)
- HTTP/3 cache gateway (`aspen-nix-cache-gateway`) serving NAR archives via `iroh+h3` ALPN
- Ed25519 Narinfo signing

### DHT Discovery (`aspen-dht-discovery`)

- BitTorrent Mainline DHT for content and peer discovery
- Used by federation and global content discovery

---

## RPC Handler Architecture

### Dispatch Pipeline

```
Client → QUIC/Iroh → ClientProtocolHandler
  → deserialize ClientRpcRequest
  → HandlerRegistry::dispatch(request, ctx, proxy_hops)
  → Route to specific handler
  → serialize ClientRpcResponse
  → return to client
```

### Handler Registry

- `HandlerRegistry`: Maps request types to `RequestHandler` implementations
- `HandlerFactory` + `inventory` crate: Self-registration at link time
- `ArcSwap`: Lock-free handler hot-reload
- `collect_handler_factories()`: Collects all registered factories

### Handler Crates (One per Domain)

| Crate | Handles |
|-------|---------|
| `aspen-kv-handler` | KV CRUD, batch, transactions |

| `aspen-blob-handler` | Blob CRUD, replication |
| `aspen-forge-handler` | All forge operations |
| `aspen-cluster-handler` | Init, membership, metrics |
| `aspen-core-essentials-handler` | Ping, health, node info |
| `aspen-docs-handler` | CRDT document operations |
| `aspen-job-handler` | Job submission, status |
| `aspen-hooks-handler` | Hook management |
| `aspen-secrets-handler` | PKI + Nix cache signing (native crypto only) |
| `aspen-ci-handler` | CI pipeline operations |
| `aspen-query-handler` | SQL queries |
| `aspen-pijul-handler` | Pijul VCS operations |
| `aspen-nix-handler` | Nix store operations |

### Cross-Cluster Proxying

- `ProxyConfig` + `ProxyService` for federated dispatch
- `proxy_hops` counter prevents infinite forwarding
- Tries local dispatch → proxy to capable cluster → `CapabilityUnavailable`

### ClientProtocolContext

Context struct passed to all handlers with optional fields per feature:

- `raft_node`, `kv_store`, `controller` (always present)
- `blob_store`, `docs_sync`, `forge_node` (feature-gated)
- `job_manager`, `hook_service`, `secrets_service` (feature-gated)
- `federation_identity`, `federation_trust_manager` (feature-gated)
- `app_registry`, `proxy_config` (always present)

---

## Feature Flags

Most functionality is behind Cargo features. See `Cargo.toml` for all available flags.

### Presets

| Preset | Features | Use Case |
|--------|----------|----------|
| `default` | Empty (Raft + KV + Coordination) | Minimal node |
| `full` | Everything (18 features) | Development |
| `forge-full` | forge + git-bridge + blob + discovery + docs | Git hosting |
| `ci-full` | ci + forge + blob + shell-worker + discovery + jobs | CI/CD |
| `ci-basic-full` | ci-basic + forge + blob + shell-worker + discovery + jobs | CI/CD (no snix) |

### Key Individual Flags

| Flag | What It Enables |
|------|----------------|
| `sql` | DataFusion SQL engine |
| `dns` | Hickory DNS server |
| `blob` | iroh-blobs storage |
| `forge` | Decentralized Git |
| `git-bridge` | GitHub/GitLab sync |
| `pijul` | Pijul VCS (GPL-2.0-or-later) |
| `plugins` | All plugin backends (umbrella) |
| `plugins-wasm` | WASM plugin host (Hyperlight) |
| `plugins-vm` | Hyperlight micro-VMs |
| `plugins-rpc` | Dynamic RPC handlers |
| `secrets` | Secrets engine |
| `automerge` | CRDT documents |
| `ci` | Full CI/CD with Nix cache (snix) |
| `ci-basic` | CI/CD without Nix cache |
| `ci-vm` | CI/CD with VM executor |
| `hooks` | Event-driven hooks |
| `federation` | Cross-cluster comms |
| `global-discovery` | DHT discovery |
| `snix` | Nix binary cache |
| `shell-worker` | Shell command execution |
| `nix-executor` | Nix build executor |
| `jobs` | Job queue system |
| `docs` | CRDT documents |
| `testing` | Test router utilities |
| `simulation` | madsim testing |
| `fuzzing` | Fuzz testing internals |
| `bolero` | Property-based testing |

---

## Testing

### Testing Pyramid

| Level | Framework | Count | What It Tests |
|-------|-----------|-------|---------------|
| Unit tests | `#[cfg(test)]` | Per-crate | Individual functions/types |
| Integration tests | `tests/*.rs` | 100 files | Multi-component interactions |
| Simulation tests | `madsim` | 16 tests | Deterministic distributed scenarios |
| Chaos tests | Custom | 5 tests | Leader crash, partitions, slow networks |
| Property tests | `proptest` + `bolero` | 16 files | Invariant verification |
| NixOS VM tests | `nixosTest` | 15 tests | Full E2E with real kernel networking |
| Benchmarks | `criterion` | 6 suites | KV, SQL, concurrency, batching |

### Simulation Testing (madsim)

- `madsim` = deterministic async runtime for distributed system testing
- `AspenRaftTester`: High-level test harness with buggify support
- `BuggifyConfig`: FoundationDB-style probabilistic fault injection
- Tests include: single-node, multi-node, replication, clock drift, heartbeat, snapshot, membership failure, failure injection, CI pipeline, crash recovery, SQL cluster, advanced scenarios, property-based
- Deterministic replay of failures

### Chaos Tests

- `chaos_leader_crash`: Leader dies during operations
- `chaos_membership_change`: Dynamic cluster reconfiguration
- `chaos_message_drops`: Partial message loss
- `chaos_network_partition`: Full network split
- `chaos_slow_network`: High latency scenarios

### NixOS VM Tests

- Real QEMU VMs with NixOS
- Real kernel networking (not simulated)
- Tests: forge cluster, multi-node consensus, KV operations, coordination, blob ops, hooks, secrets, automerge+SQL, rate limiting, plugins, and more
- Multi-node tests verify: replication, failover, cross-node operations

### Deterministic Testing Infrastructure

- `AspenRaftTester`: Madsim-based test harness (in `aspen-testing-madsim`)
- `BuggifyConfig`: Probabilistic fault injection (FoundationDB-style)
- `LivenessConfig` / `LivenessMetrics`: Liveness checking during simulation
- `aspen-testing-fixtures`: Pre-built test scenarios
- `aspen-testing-network`: Simulated network with fault injection
- `aspen-testing-core`: Shared test utilities

---

## Design Philosophy

### Tiger Style

Aspen follows "Tiger Style" — a coding philosophy influenced by TigerBeetle:

1. **Fixed resource bounds**: `MAX_BATCH_SIZE=1000`, `MAX_SHARDS=256`, `MAX_APPS_PER_CLUSTER=32`, `MAX_SCAN_RESULTS=10000`, `MAX_SNAPSHOT_SIZE=100MB`, `MAX_CAPABILITIES_PER_TOKEN=32`
2. **Explicit error types**: `snafu`-based errors with context
3. **No unbounded collections**: All vectors/maps have capacity limits
4. **Assertions in production**: `debug_assert!` and runtime checks
5. **Compile-time assertions**: `const _: () = assert!(...)` validates constant relationships
6. **Fail-fast semantics**: `ensure_disk_space_available()` panics on low disk
7. **Deterministic**: HLC for ordering, BLAKE3 for content addressing
8. **Bounded retries**: All retry loops have explicit limits
9. **Sanitized errors**: `error_sanitization` module strips internal details from client responses
10. **Centralized constants**: All bounds in `aspen-constants` crate with cross-validated assertions

### Formal Verification (Verus)

Follows the **Functional Core, Imperative Shell** pattern:

```
Production Code                    Verification
─────────────                      ─────────────
verified/scan.rs (pure functions)  verus/scan.rs (Verus specs)
verified/validation.rs             verus/validation.rs
```

- **`verified/`** dirs (12 crates): Production-compiled pure functions (no I/O, no system calls)
- **`verus/`** dirs (15 crates): Standalone Verus specifications with `ensures`/`requires`
- Pure functions take time as explicit parameter (no `SystemTime::now()`)

#### Crates with Verified Modules

`aspen-core`, `aspen-raft`, `aspen-coordination`, `aspen-auth`, `aspen-transport`, `aspen-cluster`, `aspen-forge`, `aspen-jobs`, `aspen-redb-storage`, `aspen-rpc-handlers`, `aspen-ci` / `aspen-ci-core`, `aspen-raft-network`

#### Crates with Verus Specs

`aspen-core`, `aspen-raft`, `aspen-coordination`, `aspen-auth`, `aspen-transport`, `aspen-cluster`, `aspen-forge`, `aspen-jobs`, `aspen-sharding`, `aspen-blob`, `aspen-client-api`, `aspen-ticket`, `aspen-snix`, `aspen-automerge`, `aspen-cache`

Verification coverage is tracked via the `aspen-verus-metrics` crate with terminal, GitHub, and markdown output formats.

---

## Crate Map

### Core (11 crates)

| Crate | Purpose |
|-------|---------|
| `aspen` | Main crate, wires everything together |
| `aspen-core` | Traits, types, constants, verified functions |
| `aspen-constants` | Centralized Tiger Style bounds with compile-time assertions |
| `aspen-raft` | Raft consensus (OpenRaft wrapper) |
| `aspen-raft-types` | Raft type definitions |
| `aspen-raft-network` | Raft network layer |
| `aspen-redb-storage` | Redb storage backend |
| `aspen-cluster` | Cluster coordination, bootstrap |
| `aspen-cluster-types` | Cluster type definitions |
| `aspen-cluster-bridges` | Cross-crate bridges |
| `aspen-transport` | ALPN protocol handlers |

### Storage & Crypto (9 crates)

| Crate | Purpose |
|-------|---------|
| `aspen-kv-types` | KV type definitions |
| `aspen-storage-types` | Storage abstractions |
| `aspen-disk` | Disk storage utilities |
| `aspen-hlc` | Hybrid Logical Clocks |
| `aspen-time` | Time abstractions |
| `aspen-auth` | UCAN capability tokens |
| `aspen-crypto-types` | Crypto type definitions |
| `aspen-vault` | System key validation |
| `aspen-cache` | Caching layer |

### Features (16+ crates)

| Crate | Purpose |
|-------|---------|
| `aspen-forge` | Decentralized Git |
| `aspen-forge-protocol` | Forge wire protocol |
| `aspen-ci` | CI/CD pipelines |
| `aspen-ci-core` | CI core types |
| `aspen-secrets` | Secrets management |
| `aspen-blob` | Blob storage |
| `aspen-docs` | CRDT documents |
| `aspen-automerge` | Automerge integration |
| `aspen-dns` | DNS management |
| `aspen-fuse` | FUSE filesystem |
| `aspen-hooks` | Event-driven hooks |
| `aspen-hooks-types` | Hook type definitions |
| `aspen-jobs` | Distributed job queue |
| `aspen-jobs-protocol` | Job wire protocol |
| `aspen-coordination` | Distributed primitives |
| `aspen-coordination-protocol` | Coordination wire protocol |
| `aspen-sharding` | Horizontal scaling |
| `aspen-federation` | Cross-cluster comms |
| `aspen-dht-discovery` | DHT discovery |
| `aspen-snix` | Nix binary cache |
| `aspen-nickel` | Nickel config integration |

### Client & RPC (7 crates)

| Crate | Purpose |
|-------|---------|
| `aspen-client` | Client library |
| `aspen-client-api` | Wire protocol types |
| `aspen-cli` | Command-line interface |
| `aspen-tui` | Terminal UI |
| `aspen-rpc-core` | RPC infrastructure |
| `aspen-rpc-handlers` | Handler registry |
| `aspen-ticket` | Cluster ticket encoding |

### Plugins (9 crates)

| Crate | Purpose |
|-------|---------|
| `aspen-plugin-api` | Plugin manifest + types |
| `aspen-wasm-plugin` | WASM plugin host |
| `aspen-wasm-guest-sdk` | Guest-side SDK |
| `aspen-forge-plugin` | Forge WASM plugin |
| `aspen-hooks-plugin` | Hooks WASM plugin |
| `aspen-secrets-plugin` | Secrets WASM plugin |
| `aspen-service-registry-plugin` | Service registry plugin |
| `aspen-automerge-plugin` | Automerge WASM plugin |
| `aspen-jobs-guest` | Job guest SDK |

### CI Executors (3 crates)

| Crate | Purpose |
|-------|---------|
| `aspen-ci-executor-shell` | Shell-based CI execution |
| `aspen-ci-executor-nix` | Nix sandbox execution |
| `aspen-ci-executor-vm` | VM-isolated execution |

### Job Workers (5 crates)

| Crate | Purpose |
|-------|---------|
| `aspen-jobs-worker-blob` | Blob operations |
| `aspen-jobs-worker-replication` | Data replication |
| `aspen-jobs-worker-sql` | SQL query execution |
| `aspen-jobs-worker-maintenance` | Cluster maintenance |
| `aspen-jobs-worker-shell` | Shell command execution |

### Testing (5 crates)

| Crate | Purpose |
|-------|---------|
| `aspen-testing` | Main test utilities + re-exports |
| `aspen-testing-core` | Shared test primitives |
| `aspen-testing-fixtures` | Pre-built test scenarios |
| `aspen-testing-madsim` | Madsim simulation harness |
| `aspen-testing-network` | Simulated network + fault injection |

### Other

| Crate | Purpose |
|-------|---------|
| `aspen-verus-metrics` | Verus verification coverage tracking |
| `aspen-layer` | FoundationDB-style layer primitives |
| `aspen-traits` | Shared trait definitions |
| `aspen-nix-cache-gateway` | HTTP/3 Nix cache proxy |
| `aspen-pijul` | Pijul VCS integration |
| `aspen-pijul-handler` | Pijul RPC handler |
| `aspen-sql` | SQL query engine |

---

## License

- Aspen crates: **AGPL-3.0-or-later**
- Vendored OpenRaft: **MIT OR Apache-2.0**
