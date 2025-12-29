# Aspen vs Ceph: Architecture Comparison and Gap Analysis

**Date**: 2025-12-20
**Purpose**: Analyze whether Aspen can work like Ceph and identify gaps

## Executive Summary

**Short Answer**: No, Aspen cannot currently work like Ceph. They solve fundamentally different problems at different scales.

| Aspect | Ceph | Aspen |
|--------|------|-------|
| **Primary Purpose** | Petabyte-scale unified storage (block/object/file) | Distributed coordination and consensus (etcd-like) |
| **Data Model** | Sharded objects across thousands of OSDs | Fully replicated KV store across small cluster |
| **Consistency** | Configurable (RADOS replication factor) | Strong linearizability via Raft |
| **Scale Target** | Thousands of nodes, petabytes of data | Tens of nodes, gigabytes of metadata |
| **Use Case** | VM images, backups, data lakes, media storage | Configuration management, service discovery, coordination |

Aspen is architecturally similar to **etcd** or **FoundationDB**, not Ceph.

## Detailed Architecture Comparison

### 1. Data Distribution Model

#### Ceph: CRUSH Algorithm + Placement Groups

```
Object Write
    |
    v
Pool (named container)
    |
    v
Placement Group (via hash(object_id) mod pg_num)
    |
    v
CRUSH(PG, cluster_map) -> Acting Set (3 OSDs)
    |
    v
Primary OSD receives write
    |
    v
Replication to 2 secondary OSDs
    |
    v
Client ACK
```

Key characteristics:

- **Decentralized placement**: Clients compute object location independently
- **Sharding**: Data partitioned across placement groups
- **No central coordinator**: CRUSH algorithm deterministically maps data
- **Horizontal scalability**: Add OSDs, data redistributes automatically

#### Aspen: Raft Consensus + Full Replication

```
Client Write
    |
    v
RaftNode.write()
    |
    v
OpenRaft Leader
    |
    v
Log replication to ALL followers
    |
    v
Quorum commit (majority)
    |
    v
SqliteStateMachine.apply() on ALL nodes
    |
    v
Client ACK
```

Key characteristics:

- **Full replication**: Every node stores complete dataset
- **Consensus-based**: Single leader, quorum writes
- **No sharding**: No placement groups or hash rings
- **Vertical scalability only**: Adding nodes doesn't increase capacity

### 2. Storage Interfaces

#### Ceph Provides

1. **RADOS** (foundation layer): Object storage with custom attributes
2. **RBD** (block): Striped block devices for VMs
3. **RGW** (object): S3/Swift-compatible API
4. **CephFS** (file): POSIX-compliant distributed filesystem with MDS

#### Aspen Provides

1. **KeyValueStore** trait: Simple key-value operations
2. **BlobStore** trait: Content-addressed blob storage (via iroh-blobs)
3. **FUSE** filesystem: Maps KV keys to file paths (flat namespace)
4. **iroh-docs**: CRDT-based real-time synchronization

### 3. Metadata Handling

#### Ceph

- **MDS (Metadata Server)**: Dedicated servers for CephFS metadata
- **Scalable MDS**: Multiple MDS for different directory subtrees
- **CRUSH map**: Distributed, versioned cluster topology

#### Aspen

- **No separate metadata**: All data is metadata (small values)
- **Raft log**: Single source of truth for all state
- **SQLite state machine**: Queryable metadata storage

### 4. Current Aspen Capabilities That Overlap with Ceph

| Ceph Feature | Aspen Equivalent | Gap |
|--------------|------------------|-----|
| Object storage (RADOS) | KeyValueStore + BlobStore | Limited to 1GB blobs, no attributes |
| Block storage (RBD) | None | Would require striping implementation |
| File storage (CephFS) | aspen-fuse | Virtual directories only, no POSIX semantics |
| S3 API (RGW) | None | Would need HTTP gateway layer |
| Replication | Raft consensus | All nodes get all data (not configurable) |
| Erasure coding | None | Would require significant development |
| CRUSH placement | None | No data placement algorithm |

## What Would Be Required to "Work Like Ceph"

### Critical Missing Components

#### 1. Data Sharding / Placement Algorithm

Aspen would need:

- Consistent hashing or CRUSH-like algorithm
- Placement groups abstraction
- Object-to-node mapping that's deterministic
- Automatic rebalancing on cluster topology changes

**Effort**: Major architectural change (~3-6 months)

#### 2. OSD-like Storage Nodes

Currently Aspen has:

- Raft followers that store complete replicas
- SQLite + redb for state machine and logs

Would need:

- Independent storage daemons per disk/device
- Local object storage with POSIX or raw block backend
- Heartbeat and failure detection independent of Raft
- Data scrubbing and repair

**Effort**: Major new subsystem (~2-4 months)

#### 3. Block Device Layer (RBD equivalent)

Would need:

- Object striping across placement groups
- Thin provisioning
- Snapshot and clone support
- Block device interface (NBD, virtio-blk)

**Effort**: New subsystem (~2-3 months)

#### 4. S3/Object Gateway

Would need:

- HTTP server (axum or similar)
- S3 API implementation (buckets, objects, ACLs)
- Multipart upload support
- Object versioning

**Effort**: Moderate (~1-2 months)

#### 5. Full POSIX Filesystem

Current aspen-fuse limitations:

- Virtual directories (not stored)
- No hard links, symlinks
- No extended attributes
- No locking semantics

Would need:

- Metadata server (MDS) for directory operations
- Distributed locking
- Full POSIX attribute support

**Effort**: Major new subsystem (~3-6 months)

## Alternative: What Aspen IS Good For

Aspen is designed for Ceph's **control plane**, not data plane:

### Ideal Use Cases for Aspen

1. **Cluster coordination**: Leader election, distributed locks
2. **Configuration management**: Storing cluster-wide settings
3. **Service discovery**: Tracking which services run where
4. **Metadata storage**: Indexes, mappings, catalogs
5. **Orchestration state**: Job queues, workflow state

### Comparison with Similar Systems

| System | Purpose | Aspen Similarity |
|--------|---------|------------------|
| etcd | Kubernetes coordination | Very High |
| FoundationDB | Transactional KV | High |
| Consul | Service discovery | High |
| ZooKeeper | Distributed coordination | High |
| Ceph | Large-scale storage | Low |
| MinIO | Object storage | Low |

## Recommendations

### If You Want Ceph-like Storage

1. **Use Ceph**: It's battle-tested for petabyte-scale storage
2. **Use MinIO**: S3-compatible object storage, simpler than Ceph
3. **Use GlusterFS**: Distributed file system

### If You Want to Extend Aspen Toward Storage

#### Phase 1: Enhanced Blob Storage (1-2 months)

- Larger blob support (>1GB via chunking)
- Blob sharding across nodes (via consistent hashing)
- Garbage collection improvements

#### Phase 2: Object Storage API (1-2 months)

- S3-compatible HTTP API
- Bucket and object metadata in KV
- Blob data in iroh-blobs

#### Phase 3: Placement Groups (3-4 months)

- CRUSH-like algorithm or consistent hashing
- Multiple replication strategies (2x, 3x, EC)
- Automatic rebalancing

#### Phase 4: Block Storage (2-3 months)

- Object striping
- NBD or virtio-blk interface
- Snapshot support

**Total estimated effort**: 8-12 months to achieve Ceph-like functionality

## Conclusion

Aspen cannot work like Ceph today because:

1. **Different scale targets**: Aspen is for metadata (GBs), Ceph is for data (PBs)
2. **Different consistency models**: Aspen provides linearizability, Ceph trades consistency for scale
3. **Different data distribution**: Aspen fully replicates, Ceph shards with configurable replication
4. **Different interfaces**: Aspen is KV-focused, Ceph provides block/object/file

The fundamental architecture would need to change from "all nodes store all data" to "data is sharded across nodes based on placement algorithm."

**For Ceph-like workloads, use Ceph or MinIO.**

**Aspen excels at what etcd/FoundationDB do**: strongly consistent coordination and metadata storage.

---

# PART 2: Implementation Roadmap for Ceph-like Distributed Storage

**Updated**: 2025-12-20
**Scope**: Detailed implementation plan for evolving Aspen toward Ceph-like capabilities

## Strategic Vision

Transform Aspen from an **etcd-like coordination system** into a **unified distributed storage platform** while preserving its core strengths:

- Strong consistency via Raft consensus (for metadata)
- P2P networking via Iroh (for data distribution)
- Tiger Style resource bounds (for reliability)
- Clean trait-based APIs (for extensibility)

## Architecture Evolution

```
CURRENT (Coordination-focused):
┌─────────────────────────────────────────────────────────────┐
│                    RaftNode (monolithic)                     │
│  ClusterController + KeyValueStore + BlobStore (all data)   │
├─────────────────────────────────────────────────────────────┤
│  Storage: SQLite (all KV) + redb (logs) + iroh-blobs        │
├─────────────────────────────────────────────────────────────┤
│  Network: Iroh QUIC (all protocols on single endpoint)      │
└─────────────────────────────────────────────────────────────┘

TARGET (Storage-focused):
┌─────────────────────────────────────────────────────────────┐
│                    Client Layer                              │
│    S3 Gateway  |  FUSE Mount  |  Block Device (NBD)         │
├─────────────────────────────────────────────────────────────┤
│                 Placement Layer (CRUSH-like)                 │
│   PlacementStrategy trait  |  Hash Ring  |  Rebalancer      │
├──────────────────┬──────────────────┬────────────────────────┤
│   Metadata       │    OSD Cluster    │   Monitor Cluster     │
│   Server (MDS)   │  (Object Storage) │   (Health/Topology)   │
│   Raft-based     │   iroh-blobs      │   Raft-based          │
└──────────────────┴──────────────────┴────────────────────────┘
```

---

## Phase 1: Data Sharding / Placement Algorithm (3-6 months)

### 1.1 Core Placement Abstractions

**New file: `src/placement/mod.rs`**

```rust
//! Data placement and distribution algorithms.
//!
//! Provides CRUSH-like placement for distributing data across storage nodes.
//! Tiger Style: All placement decisions are deterministic and reproducible.

pub mod strategy;
pub mod hash_ring;
pub mod crush;
pub mod rebalancer;

pub use strategy::{PlacementStrategy, PlacementResult, ReplicaSet};
pub use hash_ring::ConsistentHashRing;
pub use crush::{CrushMap, CrushBucket, CrushRule};
```

**New file: `src/placement/strategy.rs`**

```rust
use async_trait::async_trait;
use snafu::Snafu;

/// Identifies a storage node in the cluster.
pub type OsdId = u64;

/// A set of nodes that should store replicas of an object.
#[derive(Debug, Clone)]
pub struct ReplicaSet {
    /// Primary node (receives writes first).
    pub primary: OsdId,
    /// Secondary replicas for redundancy.
    pub secondaries: Vec<OsdId>,
}

/// Result of a placement calculation.
#[derive(Debug, Clone)]
pub struct PlacementResult {
    /// Placement group ID (hash of object key mod pg_num).
    pub pg_id: u32,
    /// Nodes that should store this object.
    pub replicas: ReplicaSet,
    /// Version of the placement map used.
    pub map_epoch: u64,
}

/// Errors during placement operations.
#[derive(Debug, Snafu)]
pub enum PlacementError {
    #[snafu(display("insufficient OSDs for replication factor {required}, have {available}"))]
    InsufficientOsds { required: u32, available: u32 },

    #[snafu(display("placement group {pg_id} has no active OSDs"))]
    NoActiveOsds { pg_id: u32 },

    #[snafu(display("OSD {osd_id} not found in cluster map"))]
    OsdNotFound { osd_id: OsdId },
}

/// Tiger Style limits for placement.
pub const MAX_PLACEMENT_GROUPS: u32 = 32768;
pub const MAX_REPLICATION_FACTOR: u32 = 7;
pub const MAX_OSDS_PER_CLUSTER: u32 = 10000;
pub const DEFAULT_REPLICATION_FACTOR: u32 = 3;

/// Strategy for mapping objects to storage nodes.
#[async_trait]
pub trait PlacementStrategy: Send + Sync {
    /// Calculate which nodes should store an object.
    ///
    /// # Arguments
    /// * `object_key` - The key/name of the object
    /// * `pool_id` - The storage pool (determines replication policy)
    ///
    /// # Returns
    /// PlacementResult with primary and replica nodes.
    async fn get_placement(
        &self,
        object_key: &str,
        pool_id: u32,
    ) -> Result<PlacementResult, PlacementError>;

    /// Get placement for a specific placement group.
    async fn get_pg_placement(
        &self,
        pg_id: u32,
        pool_id: u32,
    ) -> Result<ReplicaSet, PlacementError>;

    /// Hash an object key to a placement group.
    fn hash_to_pg(&self, object_key: &str, pg_num: u32) -> u32 {
        use blake3::hash;
        let hash = hash(object_key.as_bytes());
        let hash_u32 = u32::from_le_bytes(hash.as_bytes()[0..4].try_into().unwrap());
        hash_u32 % pg_num
    }

    /// Get the current map epoch (version).
    fn map_epoch(&self) -> u64;

    /// Check if a rebalance is needed.
    async fn needs_rebalance(&self) -> bool;
}
```

### 1.2 Consistent Hash Ring Implementation

**New file: `src/placement/hash_ring.rs`**

```rust
//! Consistent hash ring for simple placement scenarios.
//!
//! Provides O(log n) lookup and minimal data movement on topology changes.
//! Tiger Style: Bounded virtual nodes per OSD.

use std::collections::BTreeMap;
use std::sync::RwLock;

use super::{OsdId, PlacementError, ReplicaSet};

/// Maximum virtual nodes per physical OSD.
/// Tiger Style: Prevents unbounded memory for large clusters.
pub const MAX_VNODES_PER_OSD: u32 = 256;

/// A point on the hash ring.
type RingPosition = u64;

/// Consistent hash ring for data placement.
pub struct ConsistentHashRing {
    /// Ring position -> OSD mapping.
    ring: RwLock<BTreeMap<RingPosition, OsdId>>,
    /// OSD -> weight mapping.
    weights: RwLock<BTreeMap<OsdId, u32>>,
    /// Number of virtual nodes per weight unit.
    vnodes_per_weight: u32,
    /// Current epoch (incremented on topology change).
    epoch: std::sync::atomic::AtomicU64,
}

impl ConsistentHashRing {
    pub fn new(vnodes_per_weight: u32) -> Self {
        Self {
            ring: RwLock::new(BTreeMap::new()),
            weights: RwLock::new(BTreeMap::new()),
            vnodes_per_weight: vnodes_per_weight.min(MAX_VNODES_PER_OSD),
            epoch: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Add an OSD to the ring.
    pub fn add_osd(&self, osd_id: OsdId, weight: u32) {
        let num_vnodes = (weight * self.vnodes_per_weight).min(MAX_VNODES_PER_OSD);

        let mut ring = self.ring.write().unwrap();
        let mut weights = self.weights.write().unwrap();

        for i in 0..num_vnodes {
            let position = self.hash_vnode(osd_id, i);
            ring.insert(position, osd_id);
        }
        weights.insert(osd_id, weight);

        self.epoch.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Remove an OSD from the ring.
    pub fn remove_osd(&self, osd_id: OsdId) {
        let mut ring = self.ring.write().unwrap();
        let mut weights = self.weights.write().unwrap();

        ring.retain(|_, &mut v| v != osd_id);
        weights.remove(&osd_id);

        self.epoch.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    }

    /// Find n OSDs for a given key.
    pub fn get_replicas(&self, key: &str, n: u32) -> Result<ReplicaSet, PlacementError> {
        let ring = self.ring.read().unwrap();

        if ring.is_empty() {
            return Err(PlacementError::InsufficientOsds {
                required: n,
                available: 0,
            });
        }

        let position = self.hash_key(key);
        let mut replicas = Vec::with_capacity(n as usize);
        let mut seen = std::collections::HashSet::new();

        // Walk clockwise from position
        for (&_, &osd_id) in ring.range(position..).chain(ring.iter()) {
            if seen.insert(osd_id) {
                replicas.push(osd_id);
                if replicas.len() >= n as usize {
                    break;
                }
            }
        }

        if replicas.is_empty() {
            return Err(PlacementError::InsufficientOsds {
                required: n,
                available: 0,
            });
        }

        let primary = replicas.remove(0);
        Ok(ReplicaSet {
            primary,
            secondaries: replicas,
        })
    }

    fn hash_key(&self, key: &str) -> RingPosition {
        let hash = blake3::hash(key.as_bytes());
        u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap())
    }

    fn hash_vnode(&self, osd_id: OsdId, vnode: u32) -> RingPosition {
        let input = format!("{}:{}", osd_id, vnode);
        let hash = blake3::hash(input.as_bytes());
        u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap())
    }
}
```

### 1.3 CRUSH-like Algorithm

**New file: `src/placement/crush.rs`**

```rust
//! CRUSH (Controlled Replication Under Scalable Hashing) implementation.
//!
//! Hierarchical placement algorithm for failure domain awareness.
//! Based on the Ceph CRUSH algorithm paper (Weil et al., SC'06).

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::{OsdId, PlacementError, ReplicaSet, MAX_REPLICATION_FACTOR};

/// Maximum depth of CRUSH hierarchy.
/// Tiger Style: Prevents stack overflow in recursive descent.
pub const MAX_HIERARCHY_DEPTH: u32 = 10;

/// Maximum children per bucket.
pub const MAX_BUCKET_CHILDREN: u32 = 1000;

/// Types of CRUSH buckets (failure domains).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BucketType {
    /// Root of the hierarchy.
    Root,
    /// Datacenter.
    Datacenter,
    /// Rack within a datacenter.
    Rack,
    /// Host/server within a rack.
    Host,
    /// Individual OSD/disk.
    Osd,
}

/// A bucket in the CRUSH hierarchy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrushBucket {
    pub id: i64,  // Negative for buckets, positive for OSDs
    pub name: String,
    pub bucket_type: BucketType,
    pub weight: u32,
    pub children: Vec<i64>,
    pub algorithm: BucketAlgorithm,
}

/// Bucket selection algorithm.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BucketAlgorithm {
    /// Uniform distribution (all equal weight).
    Uniform,
    /// List bucket (linear search).
    List,
    /// Tree bucket (binary tree).
    Tree,
    /// Straw bucket (optimal but slower).
    Straw,
    /// Straw2 (improved straw).
    Straw2,
}

/// A CRUSH placement rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrushRule {
    pub name: String,
    pub steps: Vec<CrushStep>,
}

/// A step in a CRUSH rule.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CrushStep {
    /// Start from a bucket.
    Take { bucket_id: i64 },
    /// Choose n items of type from current bucket.
    Choose { num: u32, bucket_type: BucketType },
    /// Choose n items, first-n semantics.
    ChooseFirstN { num: u32, bucket_type: BucketType },
    /// Emit selected items as replicas.
    Emit,
}

/// The CRUSH map (cluster topology).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrushMap {
    /// All buckets indexed by ID.
    pub buckets: HashMap<i64, CrushBucket>,
    /// All rules indexed by name.
    pub rules: HashMap<String, CrushRule>,
    /// Map epoch (version).
    pub epoch: u64,
    /// Number of placement groups.
    pub pg_num: u32,
}

impl CrushMap {
    /// Create a new empty CRUSH map.
    pub fn new(pg_num: u32) -> Self {
        Self {
            buckets: HashMap::new(),
            rules: HashMap::new(),
            epoch: 0,
            pg_num,
        }
    }

    /// Execute CRUSH placement for an object.
    pub fn crush(
        &self,
        pg_id: u32,
        rule_name: &str,
        replica_count: u32,
    ) -> Result<ReplicaSet, PlacementError> {
        let rule = self.rules.get(rule_name).ok_or(PlacementError::OsdNotFound {
            osd_id: 0, // Rule not found
        })?;

        let replica_count = replica_count.min(MAX_REPLICATION_FACTOR);
        let mut selected: Vec<OsdId> = Vec::with_capacity(replica_count as usize);
        let mut current_bucket: Option<i64> = None;

        for step in &rule.steps {
            match step {
                CrushStep::Take { bucket_id } => {
                    current_bucket = Some(*bucket_id);
                }
                CrushStep::Choose { num, bucket_type } |
                CrushStep::ChooseFirstN { num, bucket_type } => {
                    if let Some(bucket_id) = current_bucket {
                        let items = self.choose_from_bucket(
                            bucket_id,
                            pg_id,
                            *num,
                            *bucket_type,
                            &selected,
                        )?;
                        selected.extend(items);
                    }
                }
                CrushStep::Emit => {
                    // Emit current selection
                }
            }
        }

        if selected.is_empty() {
            return Err(PlacementError::NoActiveOsds { pg_id });
        }

        let primary = selected.remove(0);
        Ok(ReplicaSet {
            primary,
            secondaries: selected,
        })
    }

    /// Choose items from a bucket using straw2 algorithm.
    fn choose_from_bucket(
        &self,
        bucket_id: i64,
        pg_id: u32,
        num: u32,
        target_type: BucketType,
        already_selected: &[OsdId],
    ) -> Result<Vec<OsdId>, PlacementError> {
        let bucket = self.buckets.get(&bucket_id)
            .ok_or(PlacementError::OsdNotFound { osd_id: bucket_id as u64 })?;

        let mut result = Vec::with_capacity(num as usize);
        let mut attempts = 0;
        const MAX_ATTEMPTS: u32 = 100;

        while result.len() < num as usize && attempts < MAX_ATTEMPTS {
            for &child_id in &bucket.children {
                if let Some(child) = self.buckets.get(&child_id) {
                    if child.bucket_type == target_type || child.bucket_type == BucketType::Osd {
                        let osd_id = if child.bucket_type == BucketType::Osd {
                            child_id as OsdId
                        } else {
                            // Recursively descend
                            continue;
                        };

                        if !already_selected.contains(&osd_id) && !result.contains(&osd_id) {
                            // Straw2 selection
                            let straw = self.straw2_weight(child_id, pg_id, child.weight);
                            // Simplified: just add if we need more
                            result.push(osd_id);
                            if result.len() >= num as usize {
                                break;
                            }
                        }
                    }
                }
            }
            attempts += 1;
        }

        Ok(result)
    }

    /// Calculate straw2 weight for selection.
    fn straw2_weight(&self, item_id: i64, pg_id: u32, weight: u32) -> u64 {
        let input = format!("{}:{}", item_id, pg_id);
        let hash = blake3::hash(input.as_bytes());
        let hash_u64 = u64::from_le_bytes(hash.as_bytes()[0..8].try_into().unwrap());

        // Weight-adjusted hash (simplified straw2)
        if weight == 0 {
            return 0;
        }
        hash_u64 / (u64::MAX / weight as u64)
    }
}
```

### 1.4 Placement Groups

**New file: `src/placement/pg.rs`**

```rust
//! Placement Group management.
//!
//! PGs provide a layer of indirection between objects and OSDs,
//! enabling efficient rebalancing and peering.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

use super::{OsdId, ReplicaSet};

/// Placement Group state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PgState {
    /// Initial state, waiting for OSDs.
    Creating,
    /// Peering with replicas to synchronize.
    Peering,
    /// Active and serving requests.
    Active,
    /// Recovering from OSD failure.
    Recovering,
    /// Data degraded but available.
    Degraded,
    /// Backfilling to new OSD.
    Backfilling,
    /// Inconsistent replicas detected.
    Inconsistent,
    /// All replicas down.
    Down,
}

/// Placement Group information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlacementGroup {
    /// PG identifier (pool_id.pg_id).
    pub id: PgId,
    /// Current state.
    pub state: PgState,
    /// Acting set (OSDs currently serving this PG).
    pub acting: ReplicaSet,
    /// Up set (OSDs that should be serving according to CRUSH).
    pub up: ReplicaSet,
    /// Last epoch where acting set changed.
    pub acting_epoch: u64,
    /// Object count in this PG.
    pub object_count: u64,
    /// Total bytes in this PG.
    pub bytes: u64,
}

/// Placement Group identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PgId {
    pub pool_id: u32,
    pub pg_num: u32,
}

impl std::fmt::Display for PgId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{:x}", self.pool_id, self.pg_num)
    }
}

/// Manages placement group state.
pub struct PgManager {
    pgs: RwLock<HashMap<PgId, PlacementGroup>>,
    epoch: std::sync::atomic::AtomicU64,
}

impl PgManager {
    pub fn new() -> Self {
        Self {
            pgs: RwLock::new(HashMap::new()),
            epoch: std::sync::atomic::AtomicU64::new(0),
        }
    }

    /// Get PG for an object.
    pub fn get_pg(&self, object_key: &str, pool_id: u32, pg_num: u32) -> PgId {
        let hash = blake3::hash(object_key.as_bytes());
        let hash_u32 = u32::from_le_bytes(hash.as_bytes()[0..4].try_into().unwrap());
        PgId {
            pool_id,
            pg_num: hash_u32 % pg_num,
        }
    }

    /// Update PG state.
    pub fn update_pg(&self, id: PgId, state: PgState, acting: ReplicaSet) {
        let mut pgs = self.pgs.write().unwrap();
        let epoch = self.epoch.fetch_add(1, std::sync::atomic::Ordering::SeqCst);

        let pg = pgs.entry(id).or_insert_with(|| PlacementGroup {
            id,
            state: PgState::Creating,
            acting: acting.clone(),
            up: acting.clone(),
            acting_epoch: epoch,
            object_count: 0,
            bytes: 0,
        });

        pg.state = state;
        pg.acting = acting;
        pg.acting_epoch = epoch;
    }
}
```

### 1.5 Integration with Existing KeyValueStore

**Modifications to `src/api/mod.rs`**:

```rust
// Add new trait for sharded storage
#[async_trait]
pub trait ShardedKeyValueStore: Send + Sync {
    /// Write to the appropriate shard based on key placement.
    async fn sharded_write(&self, request: WriteRequest) -> Result<WriteResult, KeyValueError>;

    /// Read from the appropriate shard.
    async fn sharded_read(&self, request: ReadRequest) -> Result<ReadResult, KeyValueError>;

    /// Get placement information for a key.
    async fn get_placement(&self, key: &str) -> Result<PlacementResult, KeyValueError>;
}
```

### 1.6 Deliverables - Phase 1

| Deliverable | Description | Files |
|-------------|-------------|-------|
| PlacementStrategy trait | Core abstraction for placement algorithms | `src/placement/strategy.rs` |
| ConsistentHashRing | Simple hash ring for basic placement | `src/placement/hash_ring.rs` |
| CrushMap + CrushRule | CRUSH algorithm implementation | `src/placement/crush.rs` |
| PgManager | Placement group state management | `src/placement/pg.rs` |
| Rebalancer | Automatic data migration on topology changes | `src/placement/rebalancer.rs` |
| Integration tests | Property-based tests for placement correctness | `tests/placement_proptest.rs` |
| Madsim tests | Deterministic simulation of rebalancing | `tests/madsim_placement_test.rs` |

### 1.7 Testing Strategy - Phase 1

1. **Property-based tests** (proptest):
   - Placement is deterministic (same input = same output)
   - Minimum movement on OSD add/remove
   - Replicas are on different failure domains
   - All PGs have required replica count

2. **Simulation tests** (madsim):
   - Rebalancing under network partitions
   - OSD failure during rebalance
   - Cascading failures with recovery

3. **Benchmarks**:
   - CRUSH calculation latency (target: < 1ms for 1000 OSDs)
   - Hash ring lookup (target: O(log n))
   - Rebalance data movement minimization

---

## Phase 2: OSD-like Independent Storage Daemons (2-4 months)

### 2.1 OSD Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      aspen-osd daemon                        │
├─────────────────────────────────────────────────────────────┤
│  Protocol Handlers (ALPN-based)                             │
│  ├─ OSD_ALPN: Object read/write/delete                     │
│  ├─ RECOVERY_ALPN: PG recovery and backfill                │
│  └─ HEARTBEAT_ALPN: Health reporting to monitor            │
├─────────────────────────────────────────────────────────────┤
│  Object Storage Engine                                       │
│  ├─ IrohBlobStore (content-addressed, P2P)                 │
│  ├─ Local SQLite (object metadata)                          │
│  └─ Write-ahead log (recovery)                              │
├─────────────────────────────────────────────────────────────┤
│  PG Management                                               │
│  ├─ Primary handling (client writes)                        │
│  ├─ Replica handling (replication from primary)             │
│  └─ Recovery handling (data repair)                         │
├─────────────────────────────────────────────────────────────┤
│  Local Storage Backend                                       │
│  └─ Filesystem / Raw block device                           │
└─────────────────────────────────────────────────────────────┘
```

### 2.2 OSD Trait Definition

**New file: `src/osd/mod.rs`**

```rust
//! Object Storage Daemon (OSD) implementation.
//!
//! Independent storage nodes that store and serve object data.
//! Similar to Ceph OSDs but using Iroh for networking.

pub mod daemon;
pub mod object_store;
pub mod pg_handler;
pub mod recovery;
pub mod heartbeat;

use async_trait::async_trait;
use snafu::Snafu;

use crate::placement::{OsdId, PgId};

/// OSD configuration.
#[derive(Debug, Clone)]
pub struct OsdConfig {
    /// Unique OSD identifier.
    pub id: OsdId,
    /// Data directory for object storage.
    pub data_dir: std::path::PathBuf,
    /// Journal/WAL directory (optional, can be separate SSD).
    pub journal_dir: Option<std::path::PathBuf>,
    /// Maximum object size (Tiger Style limit).
    pub max_object_size: u64,
    /// Heartbeat interval to monitor.
    pub heartbeat_interval_secs: u64,
    /// Number of recovery threads.
    pub recovery_threads: u32,
}

/// Tiger Style limits for OSD.
pub const MAX_OBJECT_SIZE: u64 = 128 * 1024 * 1024; // 128 MB
pub const MAX_OBJECTS_PER_PG: u64 = 10_000_000;
pub const MAX_CONCURRENT_RECOVERIES: u32 = 10;
pub const HEARTBEAT_INTERVAL_SECS: u64 = 5;
pub const HEARTBEAT_TIMEOUT_SECS: u64 = 20;

/// OSD errors.
#[derive(Debug, Snafu)]
pub enum OsdError {
    #[snafu(display("object not found: {key}"))]
    ObjectNotFound { key: String },

    #[snafu(display("object too large: {size} > {max}"))]
    ObjectTooLarge { size: u64, max: u64 },

    #[snafu(display("PG {pg_id} not hosted on this OSD"))]
    PgNotHosted { pg_id: PgId },

    #[snafu(display("OSD is not primary for PG {pg_id}"))]
    NotPrimary { pg_id: PgId },

    #[snafu(display("storage error: {source}"))]
    Storage { source: anyhow::Error },

    #[snafu(display("replication failed to {replica_count} replicas"))]
    ReplicationFailed { replica_count: u32 },
}

/// Object metadata.
#[derive(Debug, Clone)]
pub struct ObjectMeta {
    pub key: String,
    pub size: u64,
    pub hash: blake3::Hash,
    pub created_at: u64,
    pub modified_at: u64,
    pub pg_id: PgId,
    pub version: u64,
}

/// Core OSD operations.
#[async_trait]
pub trait ObjectStoreDaemon: Send + Sync {
    /// Write an object (primary operation).
    async fn write_object(
        &self,
        pg_id: PgId,
        key: &str,
        data: bytes::Bytes,
    ) -> Result<ObjectMeta, OsdError>;

    /// Read an object.
    async fn read_object(
        &self,
        pg_id: PgId,
        key: &str,
    ) -> Result<(ObjectMeta, bytes::Bytes), OsdError>;

    /// Delete an object.
    async fn delete_object(
        &self,
        pg_id: PgId,
        key: &str,
    ) -> Result<(), OsdError>;

    /// List objects in a PG.
    async fn list_objects(
        &self,
        pg_id: PgId,
        prefix: Option<&str>,
        limit: u32,
    ) -> Result<Vec<ObjectMeta>, OsdError>;

    /// Replicate an object to this OSD (secondary operation).
    async fn replicate_object(
        &self,
        pg_id: PgId,
        key: &str,
        data: bytes::Bytes,
        version: u64,
    ) -> Result<(), OsdError>;

    /// Get PG statistics.
    async fn get_pg_stats(&self, pg_id: PgId) -> Result<PgStats, OsdError>;
}

/// PG statistics for monitoring.
#[derive(Debug, Clone)]
pub struct PgStats {
    pub pg_id: PgId,
    pub object_count: u64,
    pub bytes_total: u64,
    pub bytes_used: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub read_bytes: u64,
    pub write_bytes: u64,
}
```

### 2.3 OSD Daemon Implementation

**New file: `src/osd/daemon.rs`**

```rust
//! OSD daemon main loop and lifecycle.

use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn, error};

use iroh::{Endpoint, SecretKey};
use iroh_router::Router;

use super::{OsdConfig, OsdError, ObjectStoreDaemon, OsdId, PgStats};
use crate::placement::PgId;

/// ALPN protocols for OSD.
pub const OSD_ALPN: &[u8] = b"aspen-osd/1";
pub const RECOVERY_ALPN: &[u8] = b"aspen-recovery/1";
pub const HEARTBEAT_ALPN: &[u8] = b"aspen-heartbeat/1";

/// OSD daemon state.
pub struct OsdDaemon {
    config: OsdConfig,
    endpoint: Endpoint,
    store: Arc<dyn ObjectStoreDaemon>,
    pg_assignments: RwLock<std::collections::HashSet<PgId>>,
    shutdown: tokio::sync::watch::Sender<bool>,
}

impl OsdDaemon {
    /// Create a new OSD daemon.
    pub async fn new(config: OsdConfig) -> Result<Self, OsdError> {
        // Generate or load OSD secret key
        let secret_key = SecretKey::generate(&mut rand::rngs::OsRng);

        let endpoint = Endpoint::builder()
            .secret_key(secret_key)
            .bind()
            .await
            .map_err(|e| OsdError::Storage { source: e.into() })?;

        let store = Arc::new(LocalObjectStore::new(&config)?);
        let (shutdown_tx, _) = tokio::sync::watch::channel(false);

        Ok(Self {
            config,
            endpoint,
            store,
            pg_assignments: RwLock::new(std::collections::HashSet::new()),
            shutdown: shutdown_tx,
        })
    }

    /// Run the OSD daemon.
    pub async fn run(&self) -> Result<(), OsdError> {
        info!(osd_id = %self.config.id, "Starting OSD daemon");

        // Set up protocol handlers
        let router = Router::builder(self.endpoint.clone())
            .accept(OSD_ALPN.to_vec(), self.osd_handler())
            .accept(RECOVERY_ALPN.to_vec(), self.recovery_handler())
            .accept(HEARTBEAT_ALPN.to_vec(), self.heartbeat_handler())
            .build()
            .await
            .map_err(|e| OsdError::Storage { source: e.into() })?;

        // Start heartbeat task
        let heartbeat_handle = tokio::spawn(self.heartbeat_loop());

        // Start recovery task
        let recovery_handle = tokio::spawn(self.recovery_loop());

        // Wait for shutdown
        let mut shutdown_rx = self.shutdown.subscribe();
        shutdown_rx.changed().await.ok();

        info!(osd_id = %self.config.id, "Shutting down OSD daemon");

        heartbeat_handle.abort();
        recovery_handle.abort();

        Ok(())
    }

    /// Heartbeat loop - report status to monitor.
    async fn heartbeat_loop(&self) {
        let interval = std::time::Duration::from_secs(self.config.heartbeat_interval_secs);
        let mut ticker = tokio::time::interval(interval);

        loop {
            ticker.tick().await;

            // Collect stats and send heartbeat
            let stats = self.collect_stats().await;
            if let Err(e) = self.send_heartbeat(&stats).await {
                warn!(osd_id = %self.config.id, error = %e, "Failed to send heartbeat");
            }
        }
    }

    /// Recovery loop - handle PG recovery.
    async fn recovery_loop(&self) {
        // Recovery implementation
    }

    fn osd_handler(&self) -> impl iroh_router::ProtocolHandler {
        // Return protocol handler for OSD operations
        todo!()
    }

    fn recovery_handler(&self) -> impl iroh_router::ProtocolHandler {
        todo!()
    }

    fn heartbeat_handler(&self) -> impl iroh_router::ProtocolHandler {
        todo!()
    }

    async fn collect_stats(&self) -> OsdStats {
        todo!()
    }

    async fn send_heartbeat(&self, _stats: &OsdStats) -> Result<(), OsdError> {
        todo!()
    }
}

/// Local object storage implementation.
struct LocalObjectStore {
    // Implementation using iroh-blobs + SQLite metadata
}

impl LocalObjectStore {
    fn new(_config: &OsdConfig) -> Result<Self, OsdError> {
        todo!()
    }
}

#[async_trait::async_trait]
impl ObjectStoreDaemon for LocalObjectStore {
    async fn write_object(
        &self,
        _pg_id: PgId,
        _key: &str,
        _data: bytes::Bytes,
    ) -> Result<super::ObjectMeta, OsdError> {
        todo!()
    }

    async fn read_object(
        &self,
        _pg_id: PgId,
        _key: &str,
    ) -> Result<(super::ObjectMeta, bytes::Bytes), OsdError> {
        todo!()
    }

    async fn delete_object(
        &self,
        _pg_id: PgId,
        _key: &str,
    ) -> Result<(), OsdError> {
        todo!()
    }

    async fn list_objects(
        &self,
        _pg_id: PgId,
        _prefix: Option<&str>,
        _limit: u32,
    ) -> Result<Vec<super::ObjectMeta>, OsdError> {
        todo!()
    }

    async fn replicate_object(
        &self,
        _pg_id: PgId,
        _key: &str,
        _data: bytes::Bytes,
        _version: u64,
    ) -> Result<(), OsdError> {
        todo!()
    }

    async fn get_pg_stats(&self, _pg_id: PgId) -> Result<PgStats, OsdError> {
        todo!()
    }
}

/// OSD-level statistics.
struct OsdStats {
    osd_id: OsdId,
    bytes_total: u64,
    bytes_used: u64,
    pg_count: u32,
    load_average: f64,
}
```

### 2.4 Deliverables - Phase 2

| Deliverable | Description | Files |
|-------------|-------------|-------|
| OsdDaemon | Main OSD daemon with protocol handlers | `src/osd/daemon.rs` |
| ObjectStoreDaemon trait | Core object operations | `src/osd/mod.rs` |
| LocalObjectStore | iroh-blobs + SQLite backend | `src/osd/object_store.rs` |
| PgHandler | Placement group handling | `src/osd/pg_handler.rs` |
| Recovery | PG recovery and backfill | `src/osd/recovery.rs` |
| Heartbeat | Health reporting to monitor | `src/osd/heartbeat.rs` |
| aspen-osd binary | Standalone OSD daemon | `src/bin/aspen-osd.rs` |

---

## Phase 3: Block Device Layer (2-3 months)

### 3.1 Block Device Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    Block Device Client                       │
│    NBD Server  |  virtio-blk (future)  |  iSCSI (future)   │
├─────────────────────────────────────────────────────────────┤
│                      Striping Layer                          │
│  Object striping across PGs (4MB default stripe size)       │
├─────────────────────────────────────────────────────────────┤
│                    Image Manager                             │
│  Images, snapshots, clones, thin provisioning               │
├─────────────────────────────────────────────────────────────┤
│                    Placement Layer                           │
│  CRUSH placement for stripe objects                          │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Block Device Trait

**New file: `src/rbd/mod.rs`**

```rust
//! RADOS Block Device (RBD) equivalent for Aspen.
//!
//! Provides striped block storage over distributed object storage.

pub mod image;
pub mod nbd;
pub mod striping;

use async_trait::async_trait;
use snafu::Snafu;

/// Block device image configuration.
#[derive(Debug, Clone)]
pub struct ImageConfig {
    /// Image name.
    pub name: String,
    /// Pool for storing image objects.
    pub pool_id: u32,
    /// Total size in bytes.
    pub size: u64,
    /// Stripe size (default 4MB).
    pub stripe_size: u64,
    /// Features enabled.
    pub features: ImageFeatures,
}

/// Image features (bitmask).
#[derive(Debug, Clone, Copy)]
pub struct ImageFeatures {
    /// Layering (clones).
    pub layering: bool,
    /// Exclusive lock.
    pub exclusive_lock: bool,
    /// Object map for fast diff.
    pub object_map: bool,
    /// Fast diff.
    pub fast_diff: bool,
    /// Deep flatten.
    pub deep_flatten: bool,
}

/// Tiger Style limits for block devices.
pub const MAX_IMAGE_SIZE: u64 = 16 * 1024 * 1024 * 1024 * 1024; // 16 TB
pub const DEFAULT_STRIPE_SIZE: u64 = 4 * 1024 * 1024; // 4 MB
pub const MAX_STRIPE_SIZE: u64 = 64 * 1024 * 1024; // 64 MB
pub const MIN_STRIPE_SIZE: u64 = 64 * 1024; // 64 KB
pub const MAX_SNAPSHOTS_PER_IMAGE: u32 = 1000;

/// Block device errors.
#[derive(Debug, Snafu)]
pub enum RbdError {
    #[snafu(display("image not found: {name}"))]
    ImageNotFound { name: String },

    #[snafu(display("image already exists: {name}"))]
    ImageExists { name: String },

    #[snafu(display("invalid size: {size}"))]
    InvalidSize { size: u64 },

    #[snafu(display("snapshot not found: {name}"))]
    SnapshotNotFound { name: String },

    #[snafu(display("I/O error at offset {offset}: {source}"))]
    IoError { offset: u64, source: anyhow::Error },
}

/// Block device operations.
#[async_trait]
pub trait BlockDevice: Send + Sync {
    /// Create a new image.
    async fn create_image(&self, config: ImageConfig) -> Result<(), RbdError>;

    /// Delete an image.
    async fn delete_image(&self, name: &str) -> Result<(), RbdError>;

    /// Resize an image.
    async fn resize_image(&self, name: &str, new_size: u64) -> Result<(), RbdError>;

    /// Read from image at offset.
    async fn read(&self, name: &str, offset: u64, length: u64) -> Result<bytes::Bytes, RbdError>;

    /// Write to image at offset.
    async fn write(&self, name: &str, offset: u64, data: bytes::Bytes) -> Result<(), RbdError>;

    /// Flush pending writes.
    async fn flush(&self, name: &str) -> Result<(), RbdError>;

    /// Create a snapshot.
    async fn create_snapshot(&self, name: &str, snap_name: &str) -> Result<(), RbdError>;

    /// Delete a snapshot.
    async fn delete_snapshot(&self, name: &str, snap_name: &str) -> Result<(), RbdError>;

    /// Clone from a snapshot.
    async fn clone(&self, parent: &str, snap: &str, child: &str) -> Result<(), RbdError>;

    /// Flatten a cloned image (copy parent data).
    async fn flatten(&self, name: &str) -> Result<(), RbdError>;
}
```

### 3.3 NBD Server

**New file: `src/rbd/nbd.rs`**

```rust
//! Network Block Device server for exposing RBD images.
//!
//! Uses the Linux NBD protocol to expose block devices to the kernel.

use std::os::unix::net::UnixListener;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::{BlockDevice, RbdError};

/// NBD request types.
#[repr(u32)]
#[derive(Debug, Clone, Copy)]
pub enum NbdCmd {
    Read = 0,
    Write = 1,
    Disc = 2,
    Flush = 3,
    Trim = 4,
}

/// NBD request header.
#[derive(Debug)]
pub struct NbdRequest {
    pub magic: u32,
    pub flags: u16,
    pub cmd: NbdCmd,
    pub handle: u64,
    pub offset: u64,
    pub length: u32,
}

/// NBD server configuration.
#[derive(Debug, Clone)]
pub struct NbdConfig {
    /// Unix socket path or TCP address.
    pub listen: String,
    /// Image name to expose.
    pub image: String,
    /// Read-only mode.
    pub readonly: bool,
}

/// NBD server.
pub struct NbdServer<B: BlockDevice> {
    config: NbdConfig,
    block_device: Arc<B>,
}

impl<B: BlockDevice> NbdServer<B> {
    pub fn new(config: NbdConfig, block_device: Arc<B>) -> Self {
        Self { config, block_device }
    }

    /// Run the NBD server.
    pub async fn run(&self) -> Result<(), RbdError> {
        // Implementation using nbd-netlink or traditional NBD protocol
        todo!()
    }

    /// Handle a single NBD request.
    async fn handle_request(&self, req: NbdRequest, data: Option<bytes::Bytes>) -> Result<bytes::Bytes, RbdError> {
        match req.cmd {
            NbdCmd::Read => {
                self.block_device.read(&self.config.image, req.offset, req.length as u64).await
            }
            NbdCmd::Write => {
                if let Some(data) = data {
                    self.block_device.write(&self.config.image, req.offset, data).await?;
                }
                Ok(bytes::Bytes::new())
            }
            NbdCmd::Flush => {
                self.block_device.flush(&self.config.image).await?;
                Ok(bytes::Bytes::new())
            }
            NbdCmd::Disc => {
                // Disconnect
                Ok(bytes::Bytes::new())
            }
            NbdCmd::Trim => {
                // Discard (optional)
                Ok(bytes::Bytes::new())
            }
        }
    }
}
```

### 3.4 Deliverables - Phase 3

| Deliverable | Description | Files |
|-------------|-------------|-------|
| BlockDevice trait | Core block device abstraction | `src/rbd/mod.rs` |
| ImageManager | Image CRUD, snapshots, clones | `src/rbd/image.rs` |
| StripingLayer | Object striping implementation | `src/rbd/striping.rs` |
| NbdServer | NBD protocol server | `src/rbd/nbd.rs` |
| aspen-rbd binary | RBD CLI tool | `src/bin/aspen-rbd.rs` |

---

## Phase 4: S3/Object Gateway (1-2 months)

### 4.1 S3 Gateway Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      HTTP/HTTPS Layer                        │
│                    (axum with Tower)                         │
├─────────────────────────────────────────────────────────────┤
│                    S3 Protocol Handler                       │
│  Using s3s crate for S3 API compatibility                   │
├─────────────────────────────────────────────────────────────┤
│                    Request Router                            │
│  Bucket operations | Object operations | Multipart          │
├─────────────────────────────────────────────────────────────┤
│                    Auth Layer (S3Auth)                       │
│  HMAC-SHA256 signature verification                          │
├─────────────────────────────────────────────────────────────┤
│                    Storage Backend                           │
│  Bucket metadata (Raft KV) | Object data (OSD cluster)      │
└─────────────────────────────────────────────────────────────┘
```

### 4.2 S3 Gateway Implementation

**New file: `src/s3/mod.rs`**

```rust
//! S3-compatible object storage gateway.
//!
//! Exposes Aspen storage via the S3 API using the s3s crate.

pub mod gateway;
pub mod auth;
pub mod bucket;
pub mod object;
pub mod multipart;

use async_trait::async_trait;
use s3s::auth::S3Auth;
use s3s::S3;

/// S3 gateway configuration.
#[derive(Debug, Clone)]
pub struct S3GatewayConfig {
    /// HTTP listen address.
    pub listen_addr: std::net::SocketAddr,
    /// Enable HTTPS.
    pub https: bool,
    /// TLS certificate path.
    pub tls_cert: Option<std::path::PathBuf>,
    /// TLS key path.
    pub tls_key: Option<std::path::PathBuf>,
    /// Default region.
    pub region: String,
}

/// Tiger Style limits for S3.
pub const MAX_BUCKET_NAME_LENGTH: usize = 63;
pub const MAX_OBJECT_KEY_LENGTH: usize = 1024;
pub const MAX_OBJECT_SIZE: u64 = 5 * 1024 * 1024 * 1024 * 1024; // 5 TB
pub const MAX_PART_SIZE: u64 = 5 * 1024 * 1024 * 1024; // 5 GB
pub const MIN_PART_SIZE: u64 = 5 * 1024 * 1024; // 5 MB
pub const MAX_PARTS_PER_UPLOAD: u32 = 10000;
pub const MAX_BUCKETS_PER_ACCOUNT: u32 = 1000;

/// S3 gateway backed by Aspen storage.
pub struct AspenS3Gateway {
    config: S3GatewayConfig,
    // Connections to metadata (Raft) and data (OSD) clusters
}

// Implement s3s::S3 trait for full S3 compatibility
// The s3s crate provides the trait with ~100 methods for S3 API
```

### 4.3 Deliverables - Phase 4

| Deliverable | Description | Files |
|-------------|-------------|-------|
| AspenS3Gateway | S3 API implementation using s3s | `src/s3/gateway.rs` |
| S3Auth | HMAC-SHA256 authentication | `src/s3/auth.rs` |
| BucketManager | Bucket CRUD operations | `src/s3/bucket.rs` |
| ObjectManager | Object CRUD, presigned URLs | `src/s3/object.rs` |
| MultipartManager | Multipart upload handling | `src/s3/multipart.rs` |
| aspen-s3 binary | S3 gateway daemon | `src/bin/aspen-s3.rs` |

---

## Phase 5: Full POSIX Filesystem with MDS (3-6 months)

### 5.1 MDS Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    FUSE / VirtioFS Client                    │
├─────────────────────────────────────────────────────────────┤
│                    MDS Protocol Handler                      │
│  Directory operations | Inode management | Locking          │
├─────────────────────────────────────────────────────────────┤
│                    Metadata Server Cluster                   │
│  Raft-based consensus for metadata operations               │
│  ├─ Namespace tree (directories, files)                     │
│  ├─ Inode table (permissions, timestamps)                   │
│  ├─ Distributed locks (file locking)                        │
│  └─ Capability management                                   │
├─────────────────────────────────────────────────────────────┤
│                    Data Routing                              │
│  File data stored on OSD cluster via CRUSH placement        │
└─────────────────────────────────────────────────────────────┘
```

### 5.2 MDS Implementation

**New file: `src/mds/mod.rs`**

```rust
//! Metadata Server for POSIX filesystem.
//!
//! Provides directory tree management, inode allocation, and locking.

pub mod inode;
pub mod directory;
pub mod locking;
pub mod capabilities;

use async_trait::async_trait;
use snafu::Snafu;

/// Inode number type.
pub type Ino = u64;

/// File types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileType {
    Regular,
    Directory,
    Symlink,
    Socket,
    Fifo,
    BlockDevice,
    CharDevice,
}

/// Inode attributes.
#[derive(Debug, Clone)]
pub struct InodeAttr {
    pub ino: Ino,
    pub size: u64,
    pub blocks: u64,
    pub atime: u64,
    pub mtime: u64,
    pub ctime: u64,
    pub mode: u32,
    pub nlink: u32,
    pub uid: u32,
    pub gid: u32,
    pub rdev: u32,
    pub file_type: FileType,
}

/// Directory entry.
#[derive(Debug, Clone)]
pub struct DirEntry {
    pub ino: Ino,
    pub name: String,
    pub file_type: FileType,
}

/// Tiger Style limits for MDS.
pub const MAX_NAME_LENGTH: usize = 255;
pub const MAX_PATH_LENGTH: usize = 4096;
pub const MAX_SYMLINK_LENGTH: usize = 4096;
pub const MAX_XATTR_SIZE: usize = 65536;
pub const MAX_DIR_ENTRIES: u32 = 1_000_000;
pub const MAX_LOCKS_PER_FILE: u32 = 1000;

/// MDS errors.
#[derive(Debug, Snafu)]
pub enum MdsError {
    #[snafu(display("no such file or directory: {path}"))]
    NotFound { path: String },

    #[snafu(display("file exists: {path}"))]
    Exists { path: String },

    #[snafu(display("not a directory: {path}"))]
    NotDirectory { path: String },

    #[snafu(display("is a directory: {path}"))]
    IsDirectory { path: String },

    #[snafu(display("directory not empty: {path}"))]
    NotEmpty { path: String },

    #[snafu(display("permission denied: {path}"))]
    PermissionDenied { path: String },

    #[snafu(display("name too long: {name}"))]
    NameTooLong { name: String },
}

/// Metadata operations.
#[async_trait]
pub trait MetadataServer: Send + Sync {
    // Inode operations
    async fn lookup(&self, parent: Ino, name: &str) -> Result<InodeAttr, MdsError>;
    async fn getattr(&self, ino: Ino) -> Result<InodeAttr, MdsError>;
    async fn setattr(&self, ino: Ino, attr: InodeAttr) -> Result<InodeAttr, MdsError>;

    // Directory operations
    async fn mkdir(&self, parent: Ino, name: &str, mode: u32) -> Result<InodeAttr, MdsError>;
    async fn rmdir(&self, parent: Ino, name: &str) -> Result<(), MdsError>;
    async fn readdir(&self, ino: Ino, offset: u64, limit: u32) -> Result<Vec<DirEntry>, MdsError>;

    // File operations
    async fn create(&self, parent: Ino, name: &str, mode: u32) -> Result<InodeAttr, MdsError>;
    async fn unlink(&self, parent: Ino, name: &str) -> Result<(), MdsError>;
    async fn rename(&self, parent: Ino, name: &str, new_parent: Ino, new_name: &str) -> Result<(), MdsError>;

    // Link operations
    async fn link(&self, ino: Ino, new_parent: Ino, new_name: &str) -> Result<InodeAttr, MdsError>;
    async fn symlink(&self, parent: Ino, name: &str, target: &str) -> Result<InodeAttr, MdsError>;
    async fn readlink(&self, ino: Ino) -> Result<String, MdsError>;

    // Extended attributes
    async fn getxattr(&self, ino: Ino, name: &str) -> Result<Vec<u8>, MdsError>;
    async fn setxattr(&self, ino: Ino, name: &str, value: &[u8]) -> Result<(), MdsError>;
    async fn listxattr(&self, ino: Ino) -> Result<Vec<String>, MdsError>;
    async fn removexattr(&self, ino: Ino, name: &str) -> Result<(), MdsError>;

    // Locking
    async fn getlk(&self, ino: Ino, owner: u64) -> Result<Option<FileLock>, MdsError>;
    async fn setlk(&self, ino: Ino, lock: FileLock, block: bool) -> Result<(), MdsError>;
}

/// File lock.
#[derive(Debug, Clone)]
pub struct FileLock {
    pub start: u64,
    pub end: u64,
    pub lock_type: LockType,
    pub owner: u64,
    pub pid: u32,
}

/// Lock types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockType {
    Read,
    Write,
    Unlock,
}
```

### 5.3 Deliverables - Phase 5

| Deliverable | Description | Files |
|-------------|-------------|-------|
| MetadataServer trait | Core MDS abstraction | `src/mds/mod.rs` |
| InodeManager | Inode allocation and caching | `src/mds/inode.rs` |
| DirectoryTree | Directory tree operations | `src/mds/directory.rs` |
| LockManager | Distributed file locking | `src/mds/locking.rs` |
| CapabilityManager | Client capability tokens | `src/mds/capabilities.rs` |
| Enhanced aspen-fuse | Full POSIX support | `src/bin/aspen-fuse/` |
| aspen-mds binary | MDS daemon | `src/bin/aspen-mds.rs` |

---

## Cross-Cutting Concerns

### Security Considerations

1. **Authentication**: HMAC-SHA256 for Raft RPC (already implemented)
2. **Authorization**: Per-bucket/object ACLs for S3
3. **Encryption**: TLS for all network traffic (via Iroh QUIC)
4. **Data at rest**: Optional encryption for blob storage

### Monitoring & Observability

1. **Metrics**: Prometheus-compatible metrics for all components
2. **Tracing**: OpenTelemetry integration
3. **Health checks**: Heartbeat protocol for OSD health
4. **Alerting**: Configurable thresholds for degraded state

### Testing Strategy

1. **Unit tests**: Property-based tests for placement algorithms
2. **Integration tests**: Multi-node cluster tests
3. **Simulation tests**: Madsim for deterministic failure injection
4. **Chaos tests**: Random failure scenarios
5. **Performance tests**: Throughput and latency benchmarks

---

## Timeline Summary

| Phase | Duration | Key Deliverables |
|-------|----------|------------------|
| Phase 1: Placement | 3-6 months | CRUSH algorithm, placement groups, rebalancer |
| Phase 2: OSD | 2-4 months | Independent storage daemons, PG handling |
| Phase 3: Block | 2-3 months | RBD-like block storage, NBD server |
| Phase 4: S3 | 1-2 months | S3-compatible API gateway |
| Phase 5: MDS | 3-6 months | Full POSIX filesystem support |

**Total**: 11-21 months for complete Ceph-like functionality

---

## References

- [CRUSH Algorithm Paper](https://ceph.com/assets/pdfs/weil-crush-sc06.pdf)
- [Ceph Architecture](https://docs.ceph.com/en/latest/architecture/)
- [CephFS MDS](https://docs.redhat.com/en/documentation/red_hat_ceph_storage/4/html/file_system_guide/the-ceph-file-system-metadata-server)
- [s3s Rust crate](https://docs.rs/s3s/latest/s3s/)
- [rust-nbd](https://github.com/tchajed/rust-nbd)
- [libch-placement](https://github.com/mochi-hpc/mochi-ch-placement)
- [DRust OSDI 2024](https://github.com/UCLA-SEAL/DRust)
- [rustfs Distributed Storage](https://typevar.dev/articles/rustfs/rustfs)
