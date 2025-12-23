# SQL Layer on Redb (Single-Fsync Architecture)

## Status

| Phase | Status | Date |
|-------|--------|------|
| Phase 1: Single-Fsync Redb | **COMPLETE** | 2025-12-23 |
| Phase 2: Tuple Encoding | Not Started | - |
| Phase 3: DataFusion SQL | Not Started | - |
| Phase 4: Secondary Indexes | Not Started | - |

### Phase 1 Results

**Target exceeded!** Achieved 6.3x write improvement (better than 3x target):

| Metric | SQLite (Before) | Redb (After) | Improvement |
|--------|-----------------|--------------|-------------|
| Single-node write | 10.5 ms | **1.66 ms** | **6.3x faster** |
| Single-node read | 13.9 µs | **5.6 µs** | **2.5x faster** |
| Write throughput | 94 ops/sec | **603 ops/sec** | **6.4x higher** |
| Read throughput | 72K ops/sec | **178K ops/sec** | **2.5x higher** |

---

## Problem Statement

Current architecture has **two fsyncs per write** (~9ms):

1. RedbLogStore (Raft log) - fsync #1
2. SqliteStateMachine (state) - fsync #2

Target: Single fsync path (~2-3ms) while maintaining full SQL query capability.

### Why This Matters

Recent benchmarks on modern hardware (Ryzen 9950X3D, Samsung 9100 PRO NVMe):

| Database | Individual Writes | Batch Writes |
|----------|-------------------|--------------|
| **Redb** | **920ms** | 1595ms |
| SQLite | 7040ms | 2625ms |

Redb is **7.6x faster** for individual writes. Additionally, NVMe SSDs have a **4.9-8.7x latency penalty** for frequent fsyncs (vs 1.8-2.9x on SATA), making single-fsync optimization critical on modern hardware.

## Requirements

- **Full SQL**: JOINs, aggregates, GROUP BY (use DataFusion, not hand-built)
- **Incremental migration**: Keep SQLite as fallback, add Redb+SQL as new option
- **Write performance first**: Get single-fsync working before full query parity

## Architecture Overview

```
Current (2 fsyncs):                    Target (1 fsync):
┌─────────────────┐                    ┌─────────────────┐
│ Write Request   │                    │ Write Request   │
└────────┬────────┘                    └────────┬────────┘
         ▼                                      ▼
┌─────────────────┐                    ┌─────────────────┐
│ RedbLogStore    │ ← fsync #1         │ RedbLogStore    │
└────────┬────────┘                    │ + StateMachine  │ ← single fsync
         ▼                             └────────┬────────┘
┌─────────────────┐                             ▼
│ SqliteStateMachine│ ← fsync #2       ┌─────────────────┐
└─────────────────┘                    │ DataFusion SQL  │ ← query layer
                                       └─────────────────┘
```

---

## OpenRaft Integration Strategy

**Critical insight**: OpenRaft treats log storage and state machine as separate components with separate transaction lifecycles. The `RaftLogStorage::append()` and `RaftStateMachine::apply()` methods are called asynchronously by different tasks.

### The Challenge

Simply sharing a `Database` handle between log and state machine does NOT achieve single-fsync:

```rust
// This DOES NOT work - two separate transactions = two fsyncs
impl RaftLogStorage for RedbLogStore {
    async fn append(&mut self, entries, callback) {
        let txn = self.db.begin_write()?;
        // insert log entries
        txn.commit()?;  // fsync #1
        callback.io_completed(Ok(()));
    }
}

impl RaftStateMachine for RedbStateMachine {
    async fn apply(&mut self, entries) {
        let txn = self.db.begin_write()?;
        // apply state mutations
        txn.commit()?;  // fsync #2 - STILL TWO FSYNCS!
    }
}
```

### The Solution: Bundle State Mutations into Log Append

Apply state changes during `RaftLogStorage::append()`, making `RaftStateMachine::apply()` a no-op:

```rust
pub struct SharedRedbStorage {
    db: Arc<Database>,
}

impl RaftLogStorage for SharedRedbStorage {
    async fn append(&mut self, entries: I, callback: IOFlushed) {
        let txn = self.db.begin_write()?;

        for entry in entries {
            // Insert into log table
            let mut log_table = txn.open_table(RAFT_LOG_TABLE)?;
            log_table.insert(entry.log_id.index, serialize(&entry)?)?;

            // Apply state mutation in same transaction
            if let EntryPayload::Normal(req) = &entry.payload {
                let mut kv_table = txn.open_table(SM_KV_TABLE)?;
                apply_write_command(&mut kv_table, req)?;
            }

            // Update last_applied
            let mut meta_table = txn.open_table(SM_META_TABLE)?;
            meta_table.insert("last_applied", serialize(&entry.log_id)?)?;
        }

        txn.commit()?;  // Single fsync for both log AND state
        callback.io_completed(Ok(()));
    }
}

impl RaftStateMachine for SharedRedbStorage {
    async fn apply(&mut self, entries: S) {
        // Already applied during append - this is intentionally a no-op
        // Only update in-memory metrics/indices if needed
    }
}
```

### Why This Is Safe

Raft's correctness requires only that **committed entries survive crashes**, NOT that log and state are durably stored sequentially. With atomic log + state commits:

1. Either both log entry and state mutation are durable, or neither is
2. Crash before `commit()` → clean rollback, Raft re-proposes
3. Crash after `commit()` → fully durable, no replay needed
4. The elimination of replay is a **feature** (simpler recovery)

### Membership Changes

Membership changes flow through the state machine and must also be bundled:

```rust
if let EntryPayload::Membership(membership) = &entry.payload {
    let mut meta_table = txn.open_table(SM_META_TABLE)?;
    meta_table.insert("membership", serialize(membership)?)?;
}
```

---

## Implementation Phases

### Phase 1: Redb State Machine (Single Fsync) - COMPLETE

**Goal**: Eliminate second fsync by storing state machine data in Redb.

**Status**: **COMPLETE** (2025-12-23)

**Implementation**:

| File | Purpose | Lines |
|------|---------|-------|
| `src/raft/storage_shared.rs` | SharedRedbStorage (log + state machine) | ~1600 |
| `src/raft/storage.rs` | Added `StorageBackend::Redb` variant | +30 |
| `src/raft/mod.rs` | Added `StateMachineVariant::Redb` | +25 |
| `src/cluster/bootstrap.rs` | Wired up Redb in bootstrap | +60 |
| `src/raft/node.rs` | Integrated Redb for read/scan/SQL | +50 |
| `benches/production.rs` | Added Redb benchmarks | +95 |

**Key Design**:

```rust
// Single struct implements BOTH RaftLogStorage AND RaftStateMachine
pub struct SharedRedbStorage {
    db: Arc<Database>,
    path: PathBuf,
    chain_tip: Arc<RwLock<ChainTipState>>,
}

// Tables (all in single Redb database)
const RAFT_LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");
const CHAIN_HASH_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("chain_hash");
const SM_KV_TABLE: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sm_kv");
const SM_LEASES_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("sm_leases");
const SM_META_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("sm_meta");

// Key innovation: apply() is a no-op since state is applied during append()
impl RaftStateMachine for SharedRedbStorage {
    async fn apply(&mut self, entries: Strm) -> Result<(), io::Error> {
        // State was already applied during append() - just send responses
        while let Some((entry, responder)) = entries.try_next().await? {
            if let Some(r) = responder { r.send(AppResponse::default()); }
        }
        Ok(())
    }
}
```

**Actual Performance** (benchmarked 2025-12-23):

| Metric | Target | Actual | Status |
|--------|--------|--------|--------|
| Single-node write | <3ms | **1.66ms** | Exceeded |
| Write improvement | 3x | **6.3x** | Exceeded |
| Read improvement | - | **2.5x** | Bonus |

---

### Phase 2: Tuple Encoding Layer

**Goal**: FoundationDB-style ordered key encoding for complex queries.

**New Files**:

| File | Purpose |
|------|---------|
| `src/layer/mod.rs` | Module root |
| `src/layer/tuple.rs` | Order-preserving tuple encoding (~500 lines) |
| `src/layer/subspace.rs` | Namespace isolation (~200 lines) |

**Tuple Encoding** (FoundationDB-compatible):

```rust
// Type codes
const NULL_CODE: u8 = 0x00;
const BYTES_CODE: u8 = 0x01;
const STRING_CODE: u8 = 0x02;
const INT_ZERO_CODE: u8 = 0x14;

pub struct Tuple { elements: Vec<TupleElement> }

impl Tuple {
    pub fn pack(&self) -> Vec<u8>;
    pub fn unpack(bytes: &[u8]) -> Result<Self, TupleError>;
    pub fn range(&self) -> (Vec<u8>, Vec<u8>);  // For prefix scans
}
```

**Subspace Pattern**:

```rust
// Namespace isolation
let kv_space = Subspace::new(Tuple::new().push(0u8));       // /0x00/...
let lease_space = Subspace::new(Tuple::new().push(1u8));    // /0x01/...
let index_space = Subspace::new(Tuple::new().push(2u8));    // /0x02/...
```

---

### Phase 3: DataFusion SQL Integration

**Goal**: Full SQL support via Apache DataFusion query engine.

**New Dependencies** (Cargo.toml):

```toml
datafusion = { version = "45", default-features = false, features = ["nested_expressions"] }
arrow = "54"
```

**New Files**:

| File | Purpose |
|------|---------|
| `src/sql/mod.rs` | Module root |
| `src/sql/provider.rs` | DataFusion TableProvider for Redb (~600 lines) |
| `src/sql/executor.rs` | SQL execution engine (~400 lines) |
| `src/sql/schema.rs` | Arrow schema definitions (~200 lines) |

**TableProvider Pattern**:

```rust
#[async_trait]
impl TableProvider for RedbTableProvider {
    async fn scan(
        &self,
        projection: Option<&Vec<usize>>,
        filters: &[Expr],
        limit: Option<usize>,
    ) -> Result<Arc<dyn ExecutionPlan>> {
        // Extract range from WHERE clause filters
        let (start_key, end_key) = extract_range_from_filters(filters)?;

        // Push down to Redb range scan
        let read_txn = self.db.begin_read()?;
        let table = read_txn.open_table(KV_TABLE)?;
        let range = table.range(start_key..end_key)?;

        // Return streaming RecordBatch iterator
        Ok(Arc::new(RedbScanExec::new(range, projection, limit)))
    }
}
```

**Files to Modify**:

| File | Changes |
|------|---------|
| `src/raft/node.rs` | Route `execute_sql` to DataFusion for Redb backend |
| `src/lib.rs` | Add `pub mod layer;` and `pub mod sql;` |

---

### Phase 4: Secondary Indexes

**Goal**: Efficient queries on non-key columns.

**New Files**:

| File | Purpose |
|------|---------|
| `src/layer/index.rs` | Secondary index framework (~400 lines) |

**Index Pattern**:

```rust
// Primary: /kv/{key} → value
// Index:  /idx/mod_revision/{revision}/{key} → ()

pub struct SecondaryIndex {
    name: String,
    subspace: Subspace,
    key_extractor: Box<dyn Fn(&KvEntry) -> Vec<u8>>,
}
```

**Built-in Indexes**:

- `idx_mod_revision`: Query by modification revision
- `idx_create_revision`: Query by creation revision
- `idx_expires_at`: Query expired keys (for cleanup)
- `idx_lease_id`: Query keys by lease

---

## Safety Analysis

### Is Sharing a Single Redb Database Safe?

**Yes**, with proper transaction handling. Redb provides:

1. **Atomicity**: Two-slot commit mechanism ensures all-or-nothing
2. **Isolation**: Read transactions see consistent snapshots (MVCC)
3. **Durability**: `Durability::Immediate` guarantees fsync on commit

### Failure Mode Analysis

| Scenario | Outcome | Safety |
|----------|---------|--------|
| Crash before `commit()` | Transaction rolled back | SAFE |
| Crash during fsync | Redb checksums detect corruption, uses previous slot | SAFE |
| Crash after `commit()` | Fully durable | SAFE |
| Crash mid-batch | Entire batch rolled back | SAFE (but wasteful) |

### The Dangerous Pattern to Avoid

```rust
// DON'T DO THIS - loses atomicity guarantee
{
    let log_txn = db.begin_write()?;
    insert_log_entry(&log_txn, entry);
    log_txn.commit()?;  // Log is durable
}
// CRASH HERE = inconsistent state!
{
    let state_txn = db.begin_write()?;
    apply_mutation(&state_txn, entry);
    state_txn.commit()?;  // State never committed
}
```

### Comparison to FoundationDB

| Aspect | FoundationDB | Proposed Aspen |
|--------|--------------|----------------|
| Log durability | Immediate fsync | Immediate fsync (same) |
| State durability | Delayed ~5s | Immediate (bundled with log) |
| Recovery | Replay from transaction logs | No replay needed |
| Write amplification | Higher (log + state) | Lower (single write) |

FoundationDB separates log and state durability; Aspen's approach bundles them, which is actually **stronger** (no replay on recovery).

---

## Alternatives Considered

### Alternative 1: SQLite Optimization (No Migration)

Keep SQLite but optimize with `PRAGMA synchronous = NORMAL`.

- **Expected**: 9ms → ~5-6ms
- **Verdict**: Less improvement, but zero risk. Good fallback.

### Alternative 2: RocksDB Instead of Redb

- **Pros**: Battle-tested at scale, built-in column families
- **Cons**: C++ FFI, complex configuration, larger binary
- **Verdict**: Overkill. Redb's pure-Rust simplicity is better for Aspen.

### Alternative 3: Fjall (LSM-Tree)

- **Pros**: Best batch write performance (353ms vs 1595ms redb)
- **Cons**: Read amplification for point lookups
- **Verdict**: Consider if write-heavy workloads dominate. Not now.

### Alternative 4: GlueSQL Instead of DataFusion

- **Pros**: Fewer dependencies (~30 vs ~100)
- **Cons**: Less mature, smaller community, similar overhead
- **Verdict**: DataFusion is more proven. Stick with it.

### Alternative 5: TigerBeetle-Style Direct I/O

- **Pros**: Could achieve ~1ms latency
- **Cons**: Radical architectural change, bypasses filesystem
- **Verdict**: Too complex. Reserve for future if needed.

---

## File Summary

### New Files (9 files, ~4400 lines)

```
src/raft/storage_redb_sm.rs    # RedbStateMachine (~2000 lines)
src/layer/mod.rs               # Layer module root (~50 lines)
src/layer/tuple.rs             # Tuple encoding (~500 lines)
src/layer/subspace.rs          # Subspace isolation (~200 lines)
src/layer/index.rs             # Secondary indexes (~400 lines)
src/sql/mod.rs                 # SQL module root (~50 lines)
src/sql/provider.rs            # DataFusion TableProvider (~600 lines)
src/sql/executor.rs            # SQL executor (~400 lines)
src/sql/schema.rs              # Arrow schemas (~200 lines)
```

### Modified Files (6 files)

```
src/lib.rs                     # Add pub mod layer, sql
src/raft/storage.rs            # Add StorageBackend::Redb
src/raft/mod.rs                # Add StateMachineVariant::Redb
src/raft/node.rs               # Wire up DataFusion SQL
src/node/mod.rs                # Wire up Redb backend in NodeBuilder
Cargo.toml                     # Add datafusion, arrow deps
```

---

## Migration Strategy

1. **SQLite remains default**: No breaking changes
2. **Opt-in Redb**: `StorageBackend::Redb` for new deployments
3. **Config-driven**: Add `storage_backend = "redb"` to TOML config
4. **No data migration**: Redb nodes start fresh (snapshots incompatible)

---

## Performance Validation

### Required Benchmarks

Add to `benches/production.rs`:

```rust
// Single-fsync write (Redb state machine)
c.bench_function("prod_write/single_redb", |b| {
    b.iter(|| {
        rt.block_on(async {
            node.write(WriteRequest::Set {
                key: format!("key-{}", rand::random::<u64>()),
                value: vec![0u8; 64],
            }).await
        })
    })
});

// Fsync batching validation
c.bench_function("prod_write/batch_redb", |b| {
    b.iter(|| {
        rt.block_on(async {
            // 10 writes should be ~3ms total, not 30ms
            for i in 0..10 {
                node.write(WriteRequest::Set { ... }).await;
            }
        })
    })
});
```

### Success Metrics

| Metric | Before | Target | Actual | Status |
|--------|--------|--------|--------|--------|
| Single-node write | 10.5ms | <3ms | **1.66ms** | **EXCEEDED** |
| Single-node read | 13.9µs | - | **5.6µs** | **2.5x faster** |
| 3-node write | ~17ms | <10ms | TBD | Pending |
| DataFusion scan overhead | N/A | <2x raw Redb | TBD | Phase 3 |

### Crash Recovery Tests

Use madsim to inject crashes at transaction boundaries:

```rust
#[madsim::test]
async fn test_crash_during_bundled_transaction() {
    // Crash between log insert and state apply (within same txn)
    // Verify: either both are visible, or neither
}

#[madsim::test]
async fn test_crash_after_commit_before_response() {
    // Crash after fsync but before client response
    // Verify: entry is durable, Raft handles re-election correctly
}
```

---

## Success Criteria

- [x] Phase 1: `prod_write/single` drops from ~9ms to ~2-3ms (Actual: 10.5ms → 1.66ms)
- [x] Phase 1: All existing Raft tests pass with Redb backend (345/345 tests pass)
- [ ] Phase 1: Crash recovery tests pass with madsim (TODO)
- [ ] Phase 2: Tuple encoding matches FoundationDB spec
- [ ] Phase 2: Property-based tests verify encoding correctness
- [ ] Phase 3: SQL queries return identical results to SQLite
- [ ] Phase 3: DataFusion scan overhead <2x raw Redb scan
- [ ] Phase 4: Index lookups show measurable speedup

---

## Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| OpenRaft integration complexity | High | Medium | POC in Phase 1A before full commitment |
| DataFusion adds ~100 transitive deps | Low | High | Use minimal features, accept trade-off |
| Arrow memory overhead | Low | Medium | Stream RecordBatches, set batch_size=8192 |
| Redb edge cases slower than SQLite | Medium | Low | Extensive benchmarks before shipping |
| Tuple encoding bugs | High | Medium | Property-based tests (proptest), fuzz testing (bolero) |
| NVMe fsync penalty underestimated | Medium | Low | Benchmark on target hardware |

---

## Execution Order

```
Phase 1A: Proof of Concept (3-5 days)
  ├─ Create SharedRedbStorage prototype
  ├─ Implement bundled log+state transaction
  ├─ Benchmark: Verify 2-3ms is achievable
  └─ GO/NO-GO decision based on benchmarks

Phase 1B: Full Implementation (1 week)
  ├─ Complete RedbStateMachine with all operations
  ├─ Handle membership changes in bundled transaction
  ├─ Run full test suite
  └─ Crash recovery validation with madsim

Phase 2: Tuple Layer (1 week)
  ├─ FoundationDB-compatible encoding
  ├─ Subspace namespacing
  ├─ Property-based tests
  └─ Ordered key scans

Phase 3: DataFusion (2 weeks)
  ├─ TableProvider implementation
  ├─ SQL execution
  ├─ Filter pushdown
  └─ Query result validation vs SQLite

Phase 4: Indexes (1 week)
  ├─ Secondary index framework
  ├─ Built-in revision indexes
  └─ Query optimization
```

---

## References

### Database Architecture

- [FoundationDB Data Modeling](https://apple.github.io/foundationdb/data-modeling.html)
- [FoundationDB Tuple Layer](https://forums.foundationdb.org/t/application-design-using-subspace-and-tuple/452)
- [TigerBeetle Architecture](https://github.com/tigerbeetle/tigerbeetle/blob/main/docs/internals/ARCHITECTURE.md)

### Rust Embedded Databases

- [Redb GitHub](https://github.com/cberner/redb)
- [Fjall LSM-Tree](https://github.com/fjall-rs/fjall)
- [Sled Alternatives Discussion](https://www.libhunt.com/r/sled)

### Query Engines

- [DataFusion Documentation](https://datafusion.apache.org/)
- [DataFusion vs Polars](https://thinhdanggroup.github.io/composable-query-engines-with-polars-and-datafusion/)

### Performance

- [NVMe Fsync Latency Study](https://www.vldb.org/pvldb/vol16/p2090-haas.pdf)
- [PostgreSQL Group Commit](https://www.percona.com/blog/2006/05/03/group-commit-and-real-fsync/)
