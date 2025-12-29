# Aspen vs TigerBeetle Design Audit

**Created**: 2025-12-14
**Scope**: Comprehensive architectural comparison identifying design gaps

## Executive Summary

This audit compares Aspen's distributed orchestration layer against TigerBeetle's financial database architecture. While Aspen has strong foundations (Tiger Style, deterministic simulation, Raft consensus), several design gaps exist in areas where TigerBeetle has made intentional, production-hardened choices.

**Key Finding**: Aspen is well-designed for its orchestration use case but could benefit from TigerBeetle's lessons in: static allocation, storage layer control, consensus protocol selection, and testing methodology refinements.

---

## 1. Consensus Protocol: VSR vs Raft

### TigerBeetle Approach: Viewstamped Replication (VSR)

TigerBeetle chose VSR over Raft for specific performance and availability benefits:

| Aspect | TigerBeetle (VSR) | Aspen (Raft) |
|--------|-------------------|--------------|
| Leader Election | Deterministic round-robin (no voting) | Vote-based election with candidates |
| Disk I/O | Can run without stable storage; async disk | Requires persistent log before responding |
| Quorum Flexibility | Flexible (3/6 replication, 4/6 view-change) | Traditional majority quorums |
| Storage Fault Recovery | Protocol-Aware Recovery (NACK protocol) | Standard Raft log repair |
| Performance | Higher availability, no sync disk requirement | Lower due to sync requirements |

### Gap Analysis

**GAP 1.1: Synchronous Disk I/O Requirement**

- **TigerBeetle**: VSR doesn't require synchronous disk I/O for consensus
- **Aspen**: Raft requires `fsync` before responding to clients
- **Impact**: Higher latency (5-50ms per write for fsync)
- **Recommendation**: Consider if Aspen's use case requires strict durability per-operation, or if batched durability would suffice

**GAP 1.2: Flexible Quorums**

- **TigerBeetle**: 6-replica cluster with 3/6 replication quorum, 4/6 election quorum
- **Aspen**: Standard Raft majority quorum (ceil(n/2)+1)
- **Impact**: TigerBeetle can tolerate complete cloud provider outage + 1 extra failure
- **Recommendation**: Consider implementing flexible quorums for improved availability

**GAP 1.3: Protocol-Aware Recovery**

- **TigerBeetle**: NACK protocol for safe WAL corruption recovery
- **Aspen**: Relies on log replication for recovery
- **Impact**: TigerBeetle can recover from storage faults that would halt Aspen
- **Recommendation**: Add corruption detection and consensus-coordinated recovery

**Mitigation**: Aspen uses OpenRaft which is well-tested and appropriate for orchestration workloads. VSR's benefits are most pronounced for financial transaction processing where TigerBeetle operates.

---

## 2. Memory Allocation Strategy

### TigerBeetle Approach: Static Allocation

TigerBeetle computes worst-case upper bounds for ALL object types at startup, allocates exact needed amounts once, then enters the event loop with ZERO runtime malloc/free operations.

### Aspen Current State

Aspen uses Tiger Style resource bounds but with **dynamic allocation**:

```rust
// Aspen: Dynamic allocation with bounds
pub const MAX_BATCH_SIZE: u32 = 1000;
pub const MAX_SNAPSHOT_ENTRIES: u32 = 1_000_000;
pub const DEFAULT_CAPACITY: u32 = 1000;

// Allocations happen at runtime, bounded but not static
let mut serialized_entries = Vec::with_capacity(MAX_BATCH_SIZE as usize);
```

### Gap Analysis

**GAP 2.1: Runtime Memory Allocation**

- **TigerBeetle**: Zero malloc after startup
- **Aspen**: Allocates on each operation (Vec, String, etc.)
- **Impact**: Latency variance, potential fragmentation over time
- **Severity**: MEDIUM for orchestration (HIGH for financial)

**GAP 2.2: Memory Pressure Under Load**

- **TigerBeetle**: Guaranteed memory footprint
- **Aspen**: Can grow until bounds hit, then errors
- **Impact**: Less predictable behavior under stress
- **Recommendation**: Consider pre-allocating object pools for hot paths

**Partial Mitigation**: Aspen's bounds prevent runaway growth. Full static allocation would require significant architectural changes.

---

## 3. Storage Architecture

### TigerBeetle Approach

- **Direct I/O**: Bypasses OS page cache entirely
- **io_uring**: Exclusive use for async syscalls
- **Single File**: WAL + Grid + Superblock in one file
- **Hash-Chained Blocks**: 512 KiB blocks with external checksums
- **Deterministic Layout**: Byte-for-byte identical across replicas

### Aspen Current State

```
Aspen Storage:
├── Redb (append-only B-tree) - Raft log
│   └── Uses mmap, OS page cache
├── SQLite (B-tree) - State machine
│   └── WAL mode, OS page cache
└── No direct I/O or io_uring
```

### Gap Analysis

**GAP 3.1: OS Page Cache Dependency**

- **TigerBeetle**: Direct I/O eliminates page cache
- **Aspen**: Relies on OS page cache for both redb and SQLite
- **Impact**: `fsync` error handling unreliable with page cache
- **Severity**: HIGH for crash consistency

**GAP 3.2: Deterministic Storage Layout**

- **TigerBeetle**: Replicas are byte-for-byte identical when synced
- **Aspen**: SQLite/redb internal layouts may differ across nodes
- **Impact**: Cannot do physical repair (block-level verification)
- **Recommendation**: Add state machine checksum verification across nodes

**GAP 3.3: Per-Entry Transaction Overhead (SQLite)**

- **TigerBeetle**: Batch 8,000 transfers per prepare
- **Aspen**: Each entry = separate SQLite transaction
- **Impact**: 3x transaction overhead (data + metadata + checkpoint)
- **Severity**: HIGH for throughput

```rust
// Current: 1000 entries = 3000 transactions
while let Some((entry, responder)) = entries.try_next().await? {
    let guard = TransactionGuard::new(&conn)?;
    SqliteStateMachine::apply_entry_payload(&conn, &entry.payload)?;
    guard.commit()?;  // Individual commit per entry
}

// Recommended: 1000 entries = 1 transaction
let guard = TransactionGuard::new(&conn)?;
while let Some((entry, responder)) = entries.try_next().await? {
    SqliteStateMachine::apply_entry_payload(&conn, &entry.payload)?;
}
guard.commit()?;  // Batch commit
```

**GAP 3.4: WAL Checkpoint Frequency**

- **TigerBeetle**: Batched superblock updates after "several megabytes"
- **Aspen**: PASSIVE checkpoint after EVERY entry
- **Impact**: Unnecessary I/O during normal operation
- **Recommendation**: Conditional checkpoint based on WAL size threshold

**GAP 3.5: Read Connection Reset**

- **Aspen**: Executes dummy query on every read to ensure WAL visibility
- **TigerBeetle**: No equivalent overhead (custom storage)
- **Impact**: ~100us latency per read operation
- **Recommendation**: Implement connection affinity or lazy reset

---

## 4. Execution Model

### TigerBeetle Approach: Single-Threaded

Despite multi-core availability, TigerBeetle uses a single thread because:

1. Hot accounts serialize transfers anyway (inherent contention)
2. Single core achieves 1M+ TPS through efficiency
3. Simplified programming model
4. Superior testing efficiency

### Aspen Current State

Aspen uses async Rust with multi-threaded Tokio runtime:

- Multiple concurrent operations via semaphore (MAX_CONCURRENT_OPS = 1000)
- Connection pooling for SQLite reads
- Single write connection (serialized)

### Gap Analysis

**GAP 4.1: Concurrency Complexity**

- **TigerBeetle**: Single-threaded = deterministic execution
- **Aspen**: Multi-threaded with locks and semaphores
- **Impact**: Harder to reason about, potential lock contention
- **Mitigation**: Aspen's madsim testing provides deterministic simulation

**GAP 4.2: Cache Efficiency**

- **TigerBeetle**: Cache-line alignment, prefetching, SIMD
- **Aspen**: Standard Rust data structures
- **Impact**: Lower CPU efficiency
- **Severity**: LOW for orchestration (Aspen is I/O bound)

**GAP 4.3: Non-Interactive Transactions**

- **TigerBeetle**: Executes transaction logic within database (no round-trips)
- **Aspen**: Standard request/response pattern
- **Impact**: Higher latency for complex operations
- **Applicability**: Less relevant for KV operations

---

## 5. Testing Methodology

### TigerBeetle Approach: VOPR (Viewstamped Operation Replicator)

**Two-Mode Architecture**:

1. **Safety Mode**: Uniform fault injection on every tick
   - Validates strict serializability under network/process/storage faults

2. **Liveness Mode**: Tests cluster availability with healthy quorum
   - Guarantees: if quorum available, cluster makes progress
   - Catches: asymmetric partition livelocks, resonance bugs

**Key Features**:

- 1 minute VOPR time = days of real-world testing
- Thousands of production-enabled assertions
- Byte-for-byte replica verification
- Targeted deterministic tests for known edge cases

### Aspen Current State

Aspen has excellent testing infrastructure:

- 350+ tests (madsim, proptest, integration)
- AspenRaftTester fluent API
- SimulationArtifact capture (3,000+ traces)
- Comprehensive fault injection

### Gap Analysis

**GAP 5.1: Liveness Mode Testing**

- **TigerBeetle**: Explicit liveness mode verifies quorum availability
- **Aspen**: Safety mode only (partitions eventually heal)
- **Impact**: May miss permanent liveness issues
- **Recommendation**: Add liveness mode to madsim tests

```rust
// Proposed liveness mode addition
impl AspenRaftTester {
    pub async fn enter_liveness_mode(&mut self, core_nodes: &[NodeId]) {
        // 1. Stop safety mode fault injection
        // 2. Select quorum as "core"
        // 3. Heal all partitions between core nodes
        // 4. Verify cluster makes progress
        // 5. Non-core nodes may remain down/partitioned
    }
}
```

**GAP 5.2: Production Assertions**

- **TigerBeetle**: Assertions enabled in production ("better to crash than continue incorrectly")
- **Aspen**: Standard debug_assert! (disabled in release)
- **Impact**: Production bugs may manifest as corruption rather than crashes
- **Recommendation**: Consider enabling critical invariant assertions in release builds

**GAP 5.3: Byte-for-Byte Verification**

- **TigerBeetle**: Replicas verified byte-identical when caught up
- **Aspen**: No cross-replica state verification
- **Impact**: Silent divergence possible
- **Recommendation**: Add state machine checksum comparison in tests

```rust
// Proposed verification
#[madsim::test]
async fn test_replica_byte_identity() {
    let t = AspenRaftTester::new(3, "byte_identity").await;
    // ... operations ...

    // Verify all nodes have identical state
    let checksums: Vec<_> = t.nodes().map(|n| n.state_machine_checksum()).collect();
    assert!(checksums.windows(2).all(|w| w[0] == w[1]));
}
```

**GAP 5.4: Storage Corruption Testing**

- **TigerBeetle**: Explicit WAL corruption tests at various offsets
- **Aspen**: Limited storage fault injection
- **Recommendation**: Add targeted corruption tests:
  - Redb log corruption at specific indices
  - SQLite WAL corruption
  - Torn writes during snapshot transfer

**GAP 5.5: Resonance Bug Detection**

- **TigerBeetle**: Liveness mode catches algorithm interaction bugs
- **Aspen**: No explicit testing for cross-system resonance
- **Example to test**: Raft log repair + gossip discovery + snapshot transfer interactions

---

## 6. Hash-Chaining and Integrity

### TigerBeetle Approach

Multi-level hash chaining throughout the system:

- **Blocks**: Pointers include address + u128 checksum
- **Prepares**: Sequential hash-chain from genesis
- **Requests**: Each includes previous reply checksum
- **Superblock**: Physically duplicated with versioning

### Aspen Current State

- OpenRaft provides log checksums
- No explicit cross-layer hash chaining
- No request/reply chaining for client deduplication

### Gap Analysis

**GAP 6.1: Misdirected Write Detection**

- **TigerBeetle**: External checksums detect blocks written to wrong address
- **Aspen**: Relies on filesystem integrity
- **Impact**: Potential silent corruption
- **Recommendation**: Add block-level checksums to storage layer

**GAP 6.2: Request/Reply Chaining**

- **TigerBeetle**: Each request includes previous reply checksum
- **Aspen**: Standard client session tracking
- **Impact**: Client-side ordering verification unavailable
- **Severity**: LOW for orchestration use case

---

## 7. Time Handling

### TigerBeetle Approach

- Primary injects specific timestamp when converting requests to prepares
- Cluster aggregates timing from replication quorum
- Guarantees strictly monotonic time observation
- Every object assigned globally unique u64 creation timestamp

### Aspen Current State

- Uses system time without cluster coordination
- No timestamp injection at consensus level
- OpenRaft handles internal timing

### Gap Analysis

**GAP 7.1: Distributed Time Coordination**

- **TigerBeetle**: Consensus-coordinated timestamps
- **Aspen**: Node-local timestamps
- **Impact**: Potential time skew across cluster
- **Severity**: LOW (Aspen doesn't require precise ordering like financial transactions)

---

## 8. Batching Strategy

### TigerBeetle Approach: Multi-Level Batching

```
8,000 transfers → 1 prepare
32 prepares → 1 disk write
1,024 prepares → 1 checkpoint
```

### Aspen Current State

```
N entries → N transactions (per-entry)
1 batch → 1 Raft append
Snapshot → manual trigger
```

### Gap Analysis

**GAP 8.1: Prepare Batching**

- **TigerBeetle**: 8,000 operations per prepare
- **Aspen**: Each client operation = 1 Raft log entry
- **Impact**: Higher consensus overhead
- **Recommendation**: Batch multiple client writes into single log entry

**GAP 8.2: Disk Write Batching**

- **TigerBeetle**: 32 prepares before disk write
- **Aspen**: Each Raft append flushes to disk
- **Impact**: More fsync operations
- **Recommendation**: Consider group commit for Raft log

**GAP 8.3: Checkpoint Batching**

- **TigerBeetle**: 1,024 prepares before checkpoint
- **Aspen**: Manual or periodic snapshots
- **Impact**: Less predictable checkpoint timing
- **Status**: Acceptable for orchestration workload

---

## 9. Dependencies and Complexity

### TigerBeetle Approach: Zero External Dependencies

Beyond Linux kernel, Zig compiler, and Zig stdlib:

- No external libraries
- Complete control over architecture
- Eliminates version management burden

### Aspen Current State

```toml
# Major dependencies
openraft = "0.10.0"  # Vendored for control
redb = "2.4.0"
rusqlite = "0.33.0"
iroh = "0.35.0"
tokio = "1.43.0"
# ... 50+ transitive dependencies
```

### Gap Analysis

**GAP 9.1: Dependency Count**

- **TigerBeetle**: ~3 dependencies (kernel, compiler, stdlib)
- **Aspen**: 50+ dependencies
- **Impact**: Larger attack surface, version conflicts, supply chain risk
- **Mitigation**: Aspen vendors openraft; other deps are well-maintained

**GAP 9.2: Control Over Internals**

- **TigerBeetle**: Can modify any layer
- **Aspen**: Limited by SQLite/redb APIs
- **Impact**: Cannot implement direct I/O, custom storage format
- **Status**: Acceptable trade-off for development velocity

---

## 10. Summary: Priority Recommendations

### Critical (Should Fix)

| ID | Gap | Recommendation | Effort |
|----|-----|----------------|--------|
| 3.3 | Per-entry transactions | Batch SQLite commits | Medium |
| 3.4 | WAL checkpoint frequency | Conditional checkpoint | Low |
| 5.1 | No liveness mode | Add liveness testing | Medium |
| 5.3 | No byte verification | Add checksum comparison | Low |

### High (Strongly Consider)

| ID | Gap | Recommendation | Effort |
|----|-----|----------------|--------|
| 3.2 | Non-deterministic storage | Add state machine checksums | Medium |
| 5.2 | Debug-only assertions | Enable critical assertions in release | Low |
| 5.4 | Limited corruption testing | Add targeted corruption tests | Medium |
| 8.1 | No prepare batching | Batch client operations | High |

### Medium (Consider for Future)

| ID | Gap | Recommendation | Effort |
|----|-----|----------------|--------|
| 1.2 | Standard quorums | Evaluate flexible quorums | High |
| 1.3 | No protocol-aware recovery | Add NACK-style corruption recovery | Very High |
| 2.1 | Runtime allocation | Object pooling for hot paths | High |
| 3.1 | OS page cache | Consider direct I/O (major change) | Very High |

### Low Priority (Context-Dependent)

| ID | Gap | Recommendation | Effort |
|----|-----|----------------|--------|
| 4.1 | Multi-threaded | Single-threaded option for testing | High |
| 6.1 | No block checksums | External checksum storage | High |
| 7.1 | No distributed time | Consensus timestamps | Medium |
| 9.1 | Many dependencies | Reduce where feasible | Ongoing |

---

## 11. Conclusion

Aspen is a well-architected distributed orchestration system that follows Tiger Style principles. The gaps identified are primarily relevant to TigerBeetle's specific domain (financial transaction processing) where:

1. **Latency matters**: TigerBeetle optimizes for <1ms writes; Aspen's 5-50ms is acceptable for orchestration
2. **Throughput matters**: TigerBeetle targets 1M+ TPS; Aspen's 1000s TPS is sufficient
3. **Corruption recovery**: Financial data requires extreme fault tolerance; orchestration can rely on quorum

**Recommended Focus Areas**:

1. **Testing**: Add liveness mode and byte-verification (highest ROI)
2. **Storage**: Batch SQLite transactions (immediate performance win)
3. **Integrity**: Add state machine checksums (safety improvement)

The other gaps represent trade-offs appropriate for Aspen's orchestration use case versus TigerBeetle's financial database use case.
