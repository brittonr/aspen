# Should Aspen Add Multi-Key Transactions and Horizontal Scaling?

*Analysis generated: 2025-12-20*

## Executive Summary

This document analyzes whether Aspen should add:

1. **Multi-key transactions** (currently single-key linearizable only)
2. **Multi-raft-group horizontal scaling** (currently single Raft group)

**TL;DR Recommendation:**

- **Multi-key transactions**: **YES, but limited scope** - Add CAS (Compare-And-Swap) semantics and expand SetMulti to support conditional writes. Full ACID transactions are high cost, low benefit for Aspen's use case.
- **Horizontal scaling via multi-raft**: **NO, not now** - The complexity is enormous, the use case is unclear, and there are simpler alternatives. Revisit if/when you hit the ~64 node limit.

---

## Part 1: Multi-Key Transactions

### Current State

Aspen supports `SetMulti` which writes up to 100 keys atomically:

```rust
WriteCommand::SetMulti { pairs: Vec<(String, String)> }  // MAX_SETMULTI_KEYS = 100
```

This is atomic within a single Raft log entry and applied via SQLite transaction. However, there's no:

- Conditional writes (CAS)
- Read-then-write transactions
- Cross-key conflict detection
- Rollback capability

### What "Multi-Key Transactions" Could Mean

| Level | Description | Implementation Complexity | Use Cases |
|-------|-------------|--------------------------|-----------|
| **L1: CAS** | Compare-and-swap on single key | Low (2-4 weeks) | Counters, locks, optimistic updates |
| **L2: Conditional SetMulti** | SetMulti with preconditions | Medium (4-6 weeks) | Batch updates with guards |
| **L3: Read-Modify-Write** | Transaction { read(keys), modify, commit } | High (8-12 weeks) | Complex business logic |
| **L4: Full ACID** | Percolator/Spanner-style distributed txns | Very High (6+ months) | General-purpose OLTP |

### Pros of Adding Multi-Key Transactions

#### Pro 1: Enables New Use Cases

- **Distributed locks**: CAS enables safe leader election, mutex acquisition
- **Counters with contention**: Increment without race conditions
- **Inventory systems**: Reserve items atomically across multiple keys
- **Config management**: Update related settings atomically with version checks

#### Pro 2: Competitive Parity

- **FoundationDB**: Full ACID transactions (their core differentiator)
- **CockroachDB**: Full SQL transactions
- **etcd**: Has Compare-and-swap via transactions
- Adding CAS would match etcd's capabilities

#### Pro 3: Low Incremental Complexity for L1-L2

Adding CAS requires:

```rust
pub enum WriteCommand {
    // Existing
    Set { key: String, value: String },
    SetMulti { pairs: Vec<(String, String)> },

    // New
    CompareAndSwap {
        key: String,
        expected: Option<String>,  // None = key must not exist
        new_value: String
    },
    ConditionalSetMulti {
        preconditions: Vec<Precondition>,
        pairs: Vec<(String, String)>,
    },
}

pub enum Precondition {
    KeyExists(String),
    KeyNotExists(String),
    KeyEquals(String, String),
    KeyVersion(String, u64),  // Requires version tracking
}
```

The state machine would check preconditions before applying. If any fail, the entire operation fails atomically. This fits cleanly within the existing single-Raft-entry model.

#### Pro 4: Better Developer Experience

Without CAS, developers must implement their own conflict detection:

```rust
// Without CAS (race-prone)
let value = kv.read("counter").await?;
let new_value = (value.parse::<i64>()? + 1).to_string();
kv.write("counter", new_value).await?;  // RACE CONDITION!

// With CAS (safe)
let value = kv.read("counter").await?;
let new_value = (value.parse::<i64>()? + 1).to_string();
kv.compare_and_swap("counter", Some(&value), &new_value).await?;
```

### Cons of Adding Multi-Key Transactions

#### Con 1: Complexity Explosion for L3-L4

Full ACID transactions require:

- **Version vectors** per key (storage overhead)
- **Lock management** (deadlock detection)
- **Transaction coordinator** (single point of contention)
- **2-phase commit** for cross-node transactions
- **Conflict resolution** (abort/retry logic)
- **Rollback capability** (undo logs)

This is the path that leads to building FoundationDB from scratch.

#### Con 2: Performance Impact

Even CAS adds overhead:

- Extra read before write (to check precondition)
- Larger log entries (preconditions serialized)
- Retry loops on contention
- Lock holding time increases latency

For Percolator-style transactions, the overhead is significant:
> "Every transaction requires contacting the [Timestamp Oracle] twice, thus making the scalability and availability of this component a significant concern."

#### Con 3: API Complexity

More transaction types = more error cases:

```rust
pub enum CasError {
    PreconditionFailed { key: String, expected: Option<String>, actual: Option<String> },
    KeyNotFound { key: String },
    VersionMismatch { key: String, expected: u64, actual: u64 },
    TransactionConflict { conflicting_key: String },
    TransactionTimeout { elapsed_ms: u64 },
}
```

Clients must handle these appropriately (retry with backoff, escalate, etc.).

#### Con 4: Aspen's Primary Use Case May Not Need It

Aspen is designed for "distributed orchestration" and "cluster coordination," similar to etcd. Most use cases are:

- Configuration storage (write once, read many)
- Service discovery (mostly reads)
- Leader election (CAS is sufficient)
- Metadata storage (atomic updates rarely cross keys)

Full ACID transactions are overkill for these.

### Recommendation: Multi-Key Transactions

**Add L1 (CAS) and L2 (Conditional SetMulti)**. Skip L3-L4.

**Estimated effort**: 4-6 weeks

**Implementation approach**:

1. Add `CompareAndSwap` to `WriteCommand` enum
2. Add `ConditionalSetMulti` with preconditions
3. Modify state machine to evaluate preconditions
4. Return specific error types for precondition failures
5. Document retry patterns for clients

**Do NOT add**:

- Version vectors (too complex, storage overhead)
- Transaction coordinators (scalability bottleneck)
- 2-phase commit (overkill for single Raft group)
- Rollback logs (not needed within single log entry)

---

## Part 2: Horizontal Scaling via Multi-Raft Groups

### Current State

Aspen uses a **single Raft group** for all data:

```
All keys → Single RaftNode → Single SQLite state machine
```

This limits horizontal scaling to ~64 nodes (MAX_PEERS constant) and means all writes go through a single leader.

### How Multi-Raft Works (CockroachDB/TiKV Pattern)

```
Keys sharded by range → Multiple Raft groups
Group 1: keys [a, m)  → Raft1 (leader: node1, followers: node2, node3)
Group 2: keys [m, z)  → Raft2 (leader: node2, followers: node1, node3)

Cross-group write → 2-phase commit across Group1 and Group2
```

**Key components needed**:

1. **Placement Driver (PD)**: Tracks key-to-group mapping
2. **Range Router**: Directs requests to correct group
3. **Split/Merge Logic**: Dynamically reshard as data grows
4. **Cross-Group Coordinator**: 2PC for multi-key writes
5. **Rebalancing**: Move ranges between nodes

### Pros of Adding Multi-Raft Horizontal Scaling

#### Pro 1: Removes Scale Ceiling

Current limit: ~64 nodes (practical), ~1000 peers (theoretical)
With multi-raft: Thousands of nodes, petabytes of data

TiKV and CockroachDB demonstrate this at scale:
> "TiKV divides data into Regions according to the key range. When a Region becomes too large (96 MB), it splits into two new ones."

#### Pro 2: Better Write Throughput

Single Raft group: All writes through one leader
Multi-Raft: Writes distributed across many leaders

> "With a single consensus group, no matter what the algorithm does, it can never achieve twice the throughput of an individual non-replicated server."

#### Pro 3: Geographic Distribution

Different Raft groups can be placed in different regions:

- Group 1 (US): keys with prefix "us/"
- Group 2 (EU): keys with prefix "eu/"
- Group 3 (Asia): keys with prefix "asia/"

This reduces cross-region latency for region-specific data.

#### Pro 4: Failure Isolation

Single group: Leader failure affects all operations
Multi-group: Leader failure in Group 1 only affects Group 1 keys

### Cons of Adding Multi-Raft Horizontal Scaling

#### Con 1: Massive Implementation Complexity

**TiKV required years of development** to get multi-raft working:
> "It is much more complex to manage multiple, dynamically-split Raft groups than a single Raft group. TiKV is currently one of only a few open-source projects that implement multiple Raft groups."

Components needed (estimate: 12-24 months):

| Component | Lines of Code (est.) | Complexity |
|-----------|---------------------|------------|
| Placement Driver | 5,000-10,000 | High |
| Range Router | 2,000-4,000 | Medium |
| Split/Merge | 5,000-8,000 | Very High |
| 2PC Coordinator | 3,000-5,000 | High |
| Rebalancing | 4,000-7,000 | Very High |
| **Total** | **19,000-34,000** | **Extreme** |

This is roughly **2x the current Aspen codebase**.

#### Con 2: Cross-Group Transactions Are Hard

> "The complexity of two-phase commit comes from all the failure scenarios that can arise."

2PC is a "blocking protocol" - if the coordinator dies during commit, participants may hang indefinitely. Solutions exist (Paxos commit, 3PC) but add more complexity.

FoundationDB avoided this by **prohibiting long-lived transactions**:
> "Relaxed latency requirements let us take a lazy approach to cleaning up locks left behind by transactions running on failed machines. This delay would not be acceptable in a DBMS running OLTP tasks."

#### Con 3: Operational Nightmare

Multi-raft introduces many failure modes:

- Split-brain across groups
- Inconsistent range metadata
- Partial splits/merges
- Orphaned ranges
- Cascade failures during rebalancing

Operators need sophisticated tooling to monitor and repair.

#### Con 4: Aspen's Target Scale Doesn't Require It

From Aspen's design philosophy:
> "Scale Target: Tens of nodes, gigabytes of metadata"

For coordination and metadata workloads:

- etcd runs Kubernetes clusters with 3-5 nodes
- FoundationDB control plane uses 5-7 coordinators
- Consul uses 3-5 servers for consensus

If your metadata fits in ~100GB and you have <100 nodes, single-raft is fine.

#### Con 5: Simpler Alternatives Exist

**Alternative A: Multiple Independent Clusters**

```
Cluster 1: Service A metadata (3 nodes)
Cluster 2: Service B metadata (3 nodes)
Application routes by service → cluster
```

**Alternative B: Tiered Architecture**

```
Control Plane: Aspen (small, consistent)
Data Plane: Object storage or specialized DB
Large values stored as blob references
```

**Alternative C: Federation**

```
Region 1: Aspen cluster (US)
Region 2: Aspen cluster (EU)
Cross-region sync via iroh-docs (eventual consistency)
```

#### Con 6: Network Layer Not Ready

Current `IrpcRaftNetworkFactory` is node-addressed, not group-addressed:

```rust
peer_addrs: Arc<RwLock<HashMap<NodeId, EndpointAddr>>>
// Missing: group_id -> (NodeId, EndpointAddr)
```

OpenRaft's `RaftNetworkFactory` trait doesn't pass group context. You'd need to either:

- Fork OpenRaft to add GroupId
- Use application-level context threading (fragile)

### Recommendation: Horizontal Scaling

**Do NOT add multi-raft horizontal scaling at this time.**

The complexity is enormous, the use case is unclear, and simpler alternatives exist.

**When to revisit**:

1. You have a concrete use case requiring >64 nodes
2. Single-leader write throughput is proven bottleneck
3. You have 6+ months of dedicated engineering time
4. You're willing to become a "distributed database company"

**In the meantime, use alternatives**:

- Multiple independent Aspen clusters for service isolation
- Federation with iroh-docs for geographic distribution
- Blob storage for large values (already supported via iroh-blobs)

---

## Implementation Roadmap

### Phase 1: Add CAS and Conditional Writes (4-6 weeks)

**Week 1-2: API Design and Types**

```rust
// src/api/mod.rs
pub enum WriteCommand {
    Set { key: String, value: String },
    SetMulti { pairs: Vec<(String, String)> },
    Delete { key: String },
    DeleteMulti { keys: Vec<String> },

    // NEW
    CompareAndSwap {
        key: String,
        expected: Option<String>,
        new_value: String,
    },
    CompareAndDelete {
        key: String,
        expected: String,
    },
    ConditionalSetMulti {
        preconditions: Vec<Precondition>,
        writes: Vec<(String, String)>,
        deletes: Vec<String>,
    },
}

pub enum Precondition {
    Exists(String),
    NotExists(String),
    Equals { key: String, value: String },
}

pub enum CasError {
    PreconditionFailed {
        index: usize,
        precondition: Precondition,
        actual_value: Option<String>,
    },
}
```

**Week 3-4: State Machine Implementation**

```rust
// src/raft/storage_sqlite.rs
fn apply_compare_and_swap(
    conn: &Connection,
    key: &str,
    expected: Option<&str>,
    new_value: &str,
) -> Result<AppResponse, CasError> {
    let actual = get_value(conn, key)?;

    match (expected, actual.as_deref()) {
        (None, None) => {
            // Key doesn't exist, can insert
            insert(conn, key, new_value)?;
            Ok(AppResponse::CasSuccess)
        }
        (Some(exp), Some(act)) if exp == act => {
            // Key exists with expected value, can update
            update(conn, key, new_value)?;
            Ok(AppResponse::CasSuccess)
        }
        _ => {
            Err(CasError::PreconditionFailed {
                actual_value: actual
            })
        }
    }
}
```

**Week 5-6: Testing and Documentation**

- Property tests for CAS linearizability
- Madsim tests for concurrent CAS operations
- Client library examples
- API documentation

### Phase 2: Evaluate Multi-Raft (Research Only) (2-3 weeks)

If horizontal scaling becomes necessary:

1. Document concrete scale requirements
2. Prototype Placement Driver design
3. Estimate engineering effort
4. Evaluate build vs. adopt (TiKV, FoundationDB layers)
5. Make go/no-go decision

---

## Summary Table

| Feature | Add? | Effort | Benefit | Risk |
|---------|------|--------|---------|------|
| CAS (Compare-And-Swap) | **YES** | 2-3 weeks | High - enables locks, counters | Low |
| Conditional SetMulti | **YES** | 2-3 weeks | Medium - batch preconditions | Low |
| Read-Modify-Write Txns | **NO** | 8-12 weeks | Medium - complex ops | High |
| Full ACID Transactions | **NO** | 6+ months | Low for Aspen's use case | Very High |
| Multi-Raft Horizontal Scale | **NO** | 12-24 months | Low until hitting limits | Extreme |

---

## Sources

### Distributed Transactions

- [Stanford: RAFT Based Key-Value Store with Transaction Support](http://www.scs.stanford.edu/20sp-cs244b/projects/RAFT%20based%20Key-Value%20Store%20with%20Transaction%20Support.pdf)
- [JetBrains Xodus: Distributed Transactions in Raft](https://github.com/JetBrains/xodus/wiki/Specification-for-distributed-transactions-in-a-Raft-based-system)
- [Percolator vs Spanner (YugabyteDB)](https://www.yugabyte.com/blog/implementing-distributed-transactions-the-google-way-percolator-vs-spanner/)
- [Martin Fowler: Two-Phase Commit](https://martinfowler.com/articles/patterns-of-distributed-systems/two-phase-commit.html)

### Multi-Raft Scaling

- [TiKV: Multi-Raft Deep Dive](https://tikv.org/deep-dive/scalability/multi-raft/)
- [CockroachDB: Scaling Raft](https://www.cockroachlabs.com/blog/scaling-raft/)
- [PingCAP: Building Large-Scale Distributed Storage on Raft](https://www.pingcap.com/blog/building-a-large-scale-distributed-storage-system-based-on-raft/)
- [CockroachDB Design Document](https://github.com/cockroachdb/cockroach/blob/master/docs/design.md)

### Two-Phase Commit

- [Wikipedia: Two-Phase Commit Protocol](https://en.wikipedia.org/wiki/Two-phase_commit_protocol)
- [Baeldung: 2PC vs Saga Pattern](https://www.baeldung.com/cs/two-phase-commit-vs-saga-pattern)
- [LinkedIn: Benefits and Challenges of 2PC](https://www.linkedin.com/advice/3/what-benefits-challenges-using-two-phase-commit)

### Percolator

- [TiKV: Percolator Deep Dive](https://tikv.org/deep-dive/distributed-transaction/percolator/)
- [Google Percolator Paper](https://research.google/pubs/pub36726/)
