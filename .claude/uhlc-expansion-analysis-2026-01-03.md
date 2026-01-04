# UHLC (Hybrid Logical Clock) Expansion Analysis

**Date**: 2026-01-03
**Analyst**: Claude (Ultra Mode Analysis)

## Executive Summary

The Aspen codebase currently uses the `uhlc` crate (Unique Hybrid Logical Clock) in one location:
**`crates/aspen-jobs/src/progress.rs`** for CRDT-based job progress tracking. This analysis identifies
10+ additional locations where HLC would provide significant value for deterministic ordering,
conflict resolution, and causal tracking.

## Current UHLC Usage

### aspen-jobs/src/progress.rs (Lines 29-32, 54-89, 469-516)

**Pattern**: CRDT-based job progress tracking with `LwwRegister<T>` using HLC timestamps.

```rust
use uhlc::HLC;
use uhlc::HLCBuilder;
use uhlc::ID;
use uhlc::Timestamp as HlcTimestamp;
```

**Key Components**:

- `CrdtProgressTracker` with shared `Arc<HLC>` instance
- `LwwRegister<T>` using HLC for deterministic last-write-wins conflict resolution
- Blake3 hash of node_id creates unique 16-byte HLC ID
- Total ordering via (time, node_id) tuple ensures commutative merges

This is an **excellent reference implementation** that should be replicated elsewhere.

---

## Priority 1: HIGH VALUE Opportunities

### 1. Multi-Cluster Federation Sync

**Files**: `crates/aspen-cluster/src/federation/sync.rs`, `types.rs`

**Current State**: Cross-cluster sync exchanges cluster identities, refs, blobs, COBs with
no explicit causality tracking between clusters.

**Why HLC**:

- Different clusters operate independently with no global clock
- HLC enables detecting which cluster-local events happened before others
- Merging cluster states without losing causality
- Resolving conflicts in cross-cluster updates deterministically

**Implementation**:

```rust
// In federation/types.rs
pub struct FederatedUpdate {
    pub cluster_id: String,
    pub hlc_timestamp: uhlc::Timestamp,  // ADD THIS
    pub payload: UpdatePayload,
}
```

---

### 2. COB (Collaborative Objects) Conflict Resolution

**Files**: `crates/aspen-forge/src/cob/change.rs`, `store.rs`

**Current State**: `MergeStrategy::LastWriteWins` uses wall-clock `timestamp_ms: u64`
(line 109: "the change with the higher timestamp wins").

**Problem**:

- Vulnerable to clock skew/drift between nodes
- Non-deterministic when wall-clock values are equal
- Already have clock drift detection (`aspen-raft/src/clock_drift_detection.rs`)

**Why HLC**:

- Deterministic total ordering even with equal timestamps
- Node ID tiebreaker ensures consistent merge results
- Self-synchronizing (no NTP dependency)

**Implementation**:

```rust
// In cob/store.rs - Replace timestamp_ms with HLC
pub struct ConflictingValue {
    pub change_hash: [u8; 32],
    pub value: String,
    pub hlc_timestamp: uhlc::Timestamp,  // REPLACE timestamp_ms
    pub author: iroh::PublicKey,
}
```

---

### 3. Event Sourcing / Log Subscribers

**File**: `crates/aspen-raft/src/log_subscriber.rs`

**Current State**: `LogEntryPayload` events carry Raft log index but no HLC.
Used by DocsExporter, monitoring, tracing.

**Why HLC**:

- Track cause-effect chains: "which write led to this read?"
- Enable "replay events as of HLC timestamp X" functionality
- Time-travel debugging across distributed nodes

**Implementation**:

```rust
// In log_subscriber.rs
pub struct LogEntryPayload {
    pub index: u64,
    pub term: u64,
    pub hlc_timestamp: Option<uhlc::Timestamp>,  // ADD THIS
    pub operation: KvOperation,
}
```

---

### 4. iroh-docs CRDT Export

**Files**: `crates/aspen-docs/src/exporter.rs`, `origin.rs`

**Current State**: `KeyOrigin` uses priority-based conflict resolution with `timestamp_secs: u64`
(wall-clock). Same-priority entries have no tiebreaker.

**Why HLC**:

- Deterministic same-priority conflict resolution
- Causal ordering for CRDT replication
- Track which export led to which subsequent state

**Implementation**:

```rust
// In origin.rs
pub struct KeyOrigin {
    pub cluster_id: String,
    pub priority: u32,
    pub timestamp_secs: u64,
    pub hlc_timestamp: Option<uhlc::Timestamp>,  // ADD FOR TIEBREAKER
    pub log_index: u64,
}
```

---

### 5. Deterministic Job Replay

**File**: `crates/aspen-jobs/src/replay.rs`

**Current State**: `JobEvent` enum uses `timestamp: u64` (wall-clock).
`DeterministicJobExecutor.current_time()` returns `SystemTime::now()`.

**Problem**:

- Replay not truly deterministic if clock granularity matters
- Two simultaneous events could lose causal ordering
- Determinism only in seed, not timestamp ordering

**Implementation**:

```rust
// In replay.rs - Replace timestamp: u64 with HLC
pub enum JobEvent {
    Submitted { job: JobSpec, hlc_timestamp: uhlc::Timestamp },
    Started { job_id: JobId, worker_id: String, hlc_timestamp: uhlc::Timestamp },
    Completed { job_id: JobId, result: JobResult, duration_ms: u64, hlc_timestamp: uhlc::Timestamp },
    // ... etc
}
```

---

## Priority 2: MEDIUM VALUE Opportunities

### 6. Gossip Protocol Ordering

**Files**: `crates/aspen-gossip/src/types.rs`, `discovery.rs`

**Current State**: `PeerAnnouncement` has `timestamp_micros` but no HLC.

**Why HLC**:

- Detect stale gossip messages
- Prevent rollbacks from out-of-order announcements
- Track "which discovery led to which config change"

### 7. Snapshot Versioning

**File**: `crates/aspen-raft/src/storage_shared.rs` (lines 160-168)

**Current State**: Snapshot ID format `"snapshot-{index}-{unix_ms}"`.

**Why HLC**:

- Deterministic snapshot ordering across multiple clusters
- Enable "merge these snapshots based on causality"
- Support time-travel queries

### 8. Distributed Tracing Spans

**File**: `crates/aspen-jobs/src/tracing.rs`

**Current State**: `DistributedSpan.start_time` uses `DateTime<Utc>` (wall-clock).

**Why HLC**:

- Causal span ordering across nodes with clock skew
- "happened-before" relationships beyond wall-clock

### 9. Distributed Coordination Primitives

**Files**: `crates/aspen-coordination/src/lock.rs`, `election.rs`, `barrier.rs`

**Current State**: `LockEntry.acquired_at_ms` and heartbeats use unix timestamps.

**Why HLC**:

- Track which lock acquisition preceded which
- Causality of election decisions
- Barrier checkpoint ordering

---

## Priority 3: LOW VALUE (Keep As-Is)

### Raft Clock Drift Detection

- Uses NTP-style offset estimation
- Purely observational, doesn't affect consensus
- Wall-clock sufficient

### Redb Storage Layer

- ACID transactions don't require HLC
- Single-fsync design ensures atomicity
- Chain hashing sufficient for integrity

---

## Implementation Recommendations

### Quick Wins (Low Effort, High Value)

1. **Add HLC to `ConflictingValue`** in `crates/aspen-forge/src/cob/store.rs`
2. **Add HLC to `LogEntryPayload`** in `crates/aspen-raft/src/log_subscriber.rs`
3. **Add HLC tiebreaker to `KeyOrigin`** in `crates/aspen-docs/src/origin.rs`

### Medium Effort, High Value

1. **Federation sync HLC** - Add to all `FederatedUpdate` messages
2. **Job replay HLC** - Replace `timestamp: u64` in `JobEvent` variants
3. **Expand aspen-jobs HLC** - Already started, extend to full lifecycle

### Shared Module Pattern

Create a workspace-level HLC utilities module:

```rust
// crates/aspen-core/src/hlc.rs (or new crate: aspen-hlc)
use uhlc::{HLC, HLCBuilder, ID, Timestamp as HlcTimestamp};

/// Create an HLC instance from a node identifier.
pub fn create_hlc(node_id: &str) -> HLC {
    let hash = blake3::hash(node_id.as_bytes());
    let id_bytes: [u8; 16] = hash.as_bytes()[..16].try_into()
        .expect("16 bytes from blake3");
    let id = ID::try_from(id_bytes).expect("16 bytes always valid for ID");
    HLCBuilder::new().with_id(id).build()
}

/// Generate a new HLC timestamp.
pub fn new_timestamp(hlc: &HLC) -> HlcTimestamp {
    hlc.new_timestamp()
}

/// Update HLC from received timestamp (for causal ordering).
pub fn update_from_timestamp(hlc: &HLC, received: HlcTimestamp) -> Result<(), uhlc::Error> {
    hlc.update_with_timestamp(&received)
}
```

---

## Priority Matrix

| Component | Value | Effort | Priority |
|-----------|-------|--------|----------|
| Multi-cluster Federation | Very High | Medium | **1** |
| COB Conflict Resolution | High | Low | **2** |
| Event Sourcing (Log Subscriber) | High | Low | **3** |
| iroh-docs Export | High | Low | **4** |
| Job Replay | High | Medium | **5** |
| Gossip Protocol | Medium | Low | **6** |
| Snapshot Versioning | Medium | Low | **7** |
| Distributed Tracing | Medium | Medium | **8** |
| Coordination Primitives | Medium | Medium | **9** |

---

## Key Insight

The existing HLC implementation in `aspen-jobs/src/progress.rs` is **production-quality**
and follows all Tiger Style principles. The same pattern should be systematically applied
to all "last-writer-wins" scenarios across the codebase. The infrastructure is proven;
this is about adoption breadth, not new capability development.

## References

- [UHLC Crate](https://github.com/atolab/uhlc-rs) - Rust implementation
- [Hybrid Logical Clocks](https://martinfowler.com/articles/patterns-of-distributed-systems/hybrid-clock.html) - Martin Fowler's pattern description
- [Original HLC Paper](https://cse.buffalo.edu/tech-reports/2014-04.pdf) - "Logical Physical Clocks and Consistent Snapshots in Globally Distributed Databases"
