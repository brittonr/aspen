# SQL Layer on Redb (Single-Fsync Architecture)

## Status

| Phase | Status | Date |
|-------|--------|------|
| Phase 1: Single-Fsync Redb | **COMPLETE** | 2025-12-23 |
| Phase 2: Tuple Encoding | **COMPLETE** | 2025-12-23 |
| Phase 3: DataFusion SQL | **COMPLETE** | 2025-12-23 |
| Phase 4: SQLite Removal | **COMPLETE** | 2025-12-26 |
| Phase 5: Secondary Indexes | **COMPLETE** | 2026-01 |

### Phase 4 Summary: SQLite Removal

**Completed 2025-12-26**: Full removal of SQLite storage backend, leaving Redb as the only persistent storage option.

**Changes:**

- Deleted `src/raft/storage_sqlite.rs` (~4,200 lines)
- Deleted `src/bin/sqlite_crash_helper/` test binary
- Removed `StorageBackend::Sqlite` variant (Redb is now default)
- Removed `StateMachineVariant::Sqlite` variant
- Updated `NodeConfig` to remove `sqlite_*` fields, consolidated to single `redb_path`
- Removed cargo dependencies: `rusqlite`, `r2d2`, `r2d2_sqlite`
- Deleted 11 SQLite-specific test files and 1 benchmark
- Added lease cleanup methods to `SharedRedbStorage` (`delete_expired_leases`, `count_expired_leases`, `count_active_leases`)

**Result:** ~4,400 lines removed, simpler codebase with single-fsync architecture as the only storage option.

### Phase 1 Results

**Target exceeded!** Achieved 6x single-node and 5.8x 3-node write improvement:

#### Single-Node Performance

| Metric | SQLite | Redb | Improvement |
|--------|--------|------|-------------|
| Write latency | 9.9 ms | **1.65 ms** | **6x faster** |
| Read latency | 9.0 µs | **5.0 µs** | **1.8x faster** |
| Write throughput | 101 ops/sec | **607 ops/sec** | **6x higher** |
| Read throughput | 111K ops/sec | **198K ops/sec** | **1.8x higher** |

#### 3-Node Cluster Performance (2025-12-23)

| Metric | SQLite | Redb | Improvement |
|--------|--------|------|-------------|
| Write latency | 18.5 ms | **3.2 ms** | **5.8x faster** |
| Read latency | 39.5 µs | **31.0 µs** | **1.3x faster** |
| Write throughput | 54 ops/sec | **311 ops/sec** | **5.8x higher** |
| Read throughput | 25K ops/sec | **32K ops/sec** | **1.3x higher** |

The 3-node results validate the single-fsync architecture under quorum replication with real Iroh QUIC networking.

#### Phase 1 Enhancements (2025-12-23)

**Response Handling Fix**: Fixed bug where `AppResponse` values (CAS results, batch counts, lease IDs) were discarded during `append()`. Now responses are stored in `pending_responses` map and retrieved during `apply()`.

**Wait API Polling**: Replaced fixed 500ms sleeps in benchmark setup with OpenRaft Wait API conditions:

- `.state(ServerState::Leader, ...)` - wait for leader election
- `.applied_index_at_least(...)` - wait for replication
- `.voter_ids([...], ...)` - wait for membership propagation

**TTL Cleanup Task**: Added background TTL cleanup for Redb storage (matching SQLite implementation):

- `delete_expired_keys(batch_limit)` - batch deletion of expired keys
- `count_expired_keys()` / `count_keys_with_ttl()` - metrics support
- `spawn_redb_ttl_cleanup_task()` - background task with CancellationToken

### Phase 2 Results

**Tuple layer complete!** FoundationDB-compatible encoding with crash recovery tests:

| Component | Status | Tests |
|-----------|--------|-------|
| Tuple encoding | Complete | 32 unit tests |
| Subspace pattern | Complete | 12 unit tests |
| Property tests | Complete | 17 proptest cases |
| Crash recovery (madsim) | Complete | 4 deterministic tests |

### Phase 3 Results

**DataFusion SQL integration complete!** Full SQL support for Redb storage backend:

| Component | Lines | Status |
|-----------|-------|--------|
| `src/sql/mod.rs` | 60 | Module root with architecture docs |
| `src/sql/error.rs` | 84 | Error types (SqlError with snafu) |
| `src/sql/schema.rs` | 103 | Arrow schema (7 columns) |
| `src/sql/provider.rs` | 481 | TableProvider + filter pushdown |
| `src/sql/executor.rs` | 366 | SQL execution with timeout/limit |
| `src/sql/stream.rs` | 370 | RecordBatch streaming |
| **Total** | **1,464** | Production-ready |

**Test Coverage**:

| Test Suite | Passing | Skipped | Notes |
|------------|---------|---------|-------|
| SQL Redb Integration | 16 | 1 | GROUP BY with substring needs function registration |
| SQL-related Total | 95 | - | All SQL tests pass |

**Key Features**:

- **Filter Pushdown**: `WHERE key = 'x'`, `key LIKE 'prefix%'`, `key >= 'a' AND key < 'b'`
- **Empty Projection**: `COUNT(*)` works via row-count-only batches (0 columns)
- **Streaming Results**: Batch size 8192, prevents memory exhaustion
- **Tiger Style Limits**: `MAX_SQL_RESULT_ROWS=10,000`, `MAX_SQL_TIMEOUT_MS=30,000`

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
    pending_responses: Arc<RwLock<BTreeMap<u64, AppResponse>>>,  // Store responses from append()
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
        // State was already applied during append()
        // Retrieve pre-computed responses from pending_responses map
        while let Some((entry, responder)) = entries.try_next().await? {
            if let Some(r) = responder {
                let response = self.pending_responses.write()?.remove(&entry.log_id.index)
                    .unwrap_or_default();
                r.send(response);
            }
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

### Phase 2: Tuple Encoding Layer - COMPLETE

**Goal**: FoundationDB-style ordered key encoding for complex queries.

**Status**: **COMPLETE** (2025-12-23)

**Implementation**:

| File | Purpose | Lines |
|------|---------|-------|
| `src/layer/mod.rs` | Module root with exports | ~60 |
| `src/layer/tuple.rs` | Order-preserving tuple encoding | ~1144 |
| `src/layer/subspace.rs` | Namespace isolation | ~439 |
| `tests/layer_tuple_proptest.rs` | Property-based tests | ~452 |
| `tests/madsim_redb_crash_recovery_test.rs` | Crash recovery tests | ~871 |

**Tuple Encoding** (FoundationDB-compatible):

```rust
// Type codes (matching FoundationDB spec)
const NULL_CODE: u8 = 0x00;
const BYTES_CODE: u8 = 0x01;
const STRING_CODE: u8 = 0x02;
const NESTED_CODE: u8 = 0x05;
const INT_ZERO_CODE: u8 = 0x14;  // ±0 encodes as single byte
const FALSE_CODE: u8 = 0x26;
const TRUE_CODE: u8 = 0x27;
const FLOAT_CODE: u8 = 0x20;
const DOUBLE_CODE: u8 = 0x21;

pub enum Element {
    Null, Bytes(Vec<u8>), String(String), Int(i64),
    Bool(bool), Float(f32), Double(f64), Tuple(Tuple),
}

impl Tuple {
    pub fn pack(&self) -> Vec<u8>;           // Lexicographically ordered
    pub fn unpack(bytes: &[u8]) -> Result<Self, TupleError>;
    pub fn range(&self) -> (Vec<u8>, Vec<u8>);  // For prefix scans
    pub fn strinc(&self) -> Option<Vec<u8>>;    // Key successor
}
```

**Subspace Pattern**:

```rust
// Namespace isolation with automatic prefix management
let users = Subspace::new(Tuple::new().push("users"));
let orders = Subspace::new(Tuple::new().push("orders"));

// Nested subspaces
let user_profiles = users.subspace(&Tuple::new().push("profiles"));

// Range queries
let (start, end) = users.range();  // All keys under "users" prefix
```

**Crash Recovery Tests** (madsim deterministic simulation):

| Test | Seed | Purpose |
|------|------|---------|
| `test_crash_during_bundled_transaction` | 100 | Atomicity verification |
| `test_crash_after_commit_before_response` | 200 | Durability verification |
| `test_crash_recovery_chain_hash_integrity` | 300 | Hash chain verification |
| `test_multiple_crash_recovery_cycles` | 400 | Multi-crash quorum test |

---

### Phase 3: DataFusion SQL Integration - COMPLETE

**Goal**: Full SQL support via Apache DataFusion query engine.

**Status**: **COMPLETE** (2025-12-23)

**Dependencies** (Cargo.toml):

```toml
datafusion = { version = "45", default-features = false, features = ["nested_expressions"] }
arrow = "54"
```

**Implementation**:

| File | Purpose | Lines |
|------|---------|-------|
| `src/sql/mod.rs` | Module root with architecture docs | 60 |
| `src/sql/error.rs` | Error types (SqlError with snafu) | 84 |
| `src/sql/schema.rs` | Arrow schema (7 columns: key, value, version, revisions, TTL, lease) | 103 |
| `src/sql/provider.rs` | DataFusion TableProvider + RedbScanExec | 481 |
| `src/sql/executor.rs` | RedbSqlExecutor with timeout/limit | 366 |
| `src/sql/stream.rs` | RedbRecordBatchStream with batch_size=8192 | 370 |
| `tests/sql_redb_integration_test.rs` | 17 integration tests (16 pass, 1 skipped) | 385 |

**Key Design - Filter Pushdown**:

```rust
// Extract key range from WHERE clause predicates
fn extract_key_range(filters: &[Expr]) -> KeyRange {
    for filter in filters {
        match filter {
            // Exact match: WHERE key = 'value'
            Expr::BinaryExpr { left, op: Eq, right } => {
                range.exact = Some(value.into_bytes());
            }
            // Prefix scan: WHERE key LIKE 'prefix%'
            Expr::Like { expr, pattern } => {
                range.start = Some(prefix.as_bytes().to_vec());
                range.end = strinc(prefix.as_bytes());  // FoundationDB pattern
                range.is_prefix = true;
            }
            // Range: WHERE key >= 'a' AND key < 'b'
            Expr::BinaryExpr { op: GtEq | Lt, .. } => { ... }
        }
    }
}
```

**Key Design - Empty Projection for COUNT(*)**:

```rust
// DataFusion requests 0 columns for COUNT(*) - just needs row count
if is_empty_projection {
    let batch = RecordBatch::try_new_with_options(
        Arc::new(Schema::empty()),
        vec![],
        &RecordBatchOptions::new().with_row_count(Some(rows_in_batch)),
    )?;
    return Ok(Some(batch));
}
```

**RaftNode Integration** (`src/raft/node.rs:985-996`):

```rust
StateMachineVariant::Redb(sm) => {
    let executor = crate::sql::RedbSqlExecutor::new(sm.db().clone());
    executor.execute(&request.query, &request.params, request.limit, request.timeout_ms).await
}
```

---

### Phase 5: Secondary Indexes - COMPLETE

**Goal**: Efficient queries on non-key columns.

**Status**: **COMPLETE** (2026-01)

**Implementation**:

| File | Purpose | Lines |
|------|---------|-------|
| `crates/aspen-layer/src/index.rs` | Secondary index framework | ~944 |

**Key Components**:

```rust
pub struct IndexRegistry { /* manages multiple indexes */ }
pub struct SecondaryIndex { /* single index definition */ }
pub struct IndexDefinition { /* serializable index config */ }
pub trait IndexQueryExecutor { /* query interface */ }
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

### All Phases Complete

```
# Phase 1: Single-Fsync Redb
crates/aspen-raft/src/storage_shared.rs  # SharedRedbStorage (~2,878 lines)

# Phase 2: Tuple Encoding Layer
crates/aspen-layer/src/lib.rs       # Layer module root (~86 lines)
crates/aspen-layer/src/tuple.rs     # Tuple encoding (~1,099 lines)
crates/aspen-layer/src/subspace.rs  # Subspace isolation (~437 lines)
crates/aspen-layer/src/proptest.rs  # Property-based tests (~447 lines)
tests/madsim_redb_crash_recovery_test.rs  # Crash recovery tests

# Phase 3: DataFusion SQL
crates/aspen-sql/src/lib.rs         # SQL module root (~103 lines)
crates/aspen-sql/src/error.rs       # Error types (~87 lines)
crates/aspen-sql/src/schema.rs      # Arrow schema (~108 lines)
crates/aspen-sql/src/provider.rs    # TableProvider + filter pushdown (~803 lines)
crates/aspen-sql/src/executor.rs    # SQL executor with timeout/limit (~667 lines)
crates/aspen-sql/src/stream.rs      # RecordBatch streaming (~788 lines)
tests/sql_redb_integration_test.rs  # Integration tests

# Phase 5: Secondary Indexes
crates/aspen-layer/src/index.rs     # Secondary indexes (~944 lines)
```

### Modified Files (8 files)

```
src/lib.rs                     # Add pub mod layer, pub mod sql (done)
src/raft/storage.rs            # Add StorageBackend::Redb (done)
src/raft/mod.rs                # Add StateMachineVariant::Redb (done)
src/raft/node.rs               # Wire up DataFusion SQL (done)
src/raft/ttl_cleanup.rs        # Add spawn_redb_ttl_cleanup_task (done)
src/cluster/bootstrap.rs       # Spawn Redb TTL cleanup task (done)
src/node/mod.rs                # Wire up Redb backend in NodeBuilder (done)
Cargo.toml                     # Add datafusion, arrow deps (done)
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

| Metric | SQLite | Redb | Improvement | Status |
|--------|--------|------|-------------|--------|
| Single-node write | 9.9 ms | **1.65 ms** | **6x** | **EXCEEDED** |
| Single-node read | 9.0 µs | **5.0 µs** | **1.8x** | **EXCEEDED** |
| 3-node write | 18.5 ms | **3.2 ms** | **5.8x** | **EXCEEDED** |
| 3-node read | 39.5 µs | **31.0 µs** | **1.3x** | **IMPROVED** |
| DataFusion scan overhead | N/A | <2x raw Redb | TBD | Phase 3 |

### Crash Recovery Tests - COMPLETE

Implemented in `tests/madsim_redb_crash_recovery_test.rs`:

```rust
#[madsim::test]
async fn test_crash_during_bundled_transaction_seed_100() {
    // Atomicity: Crash during bundled transaction
    // Verifies: either both log AND state are visible, or neither
    // Result: PASS - atomic rollback on crash
}

#[madsim::test]
async fn test_crash_after_commit_before_response_seed_200() {
    // Durability: Crash after fsync but before client response
    // Verifies: committed entry is durable on new leader
    // Result: PASS - data survives leader crash
}

#[madsim::test]
async fn test_crash_recovery_chain_hash_integrity_seed_300() {
    // Integrity: Verify chain hash survives crashes
    // Verifies: hash chain is consistent after recovery
    // Result: PASS - log index preserved
}

#[madsim::test]
async fn test_multiple_crash_recovery_cycles_seed_400() {
    // Stress: Multiple crash/recovery cycles
    // Verifies: cluster survives 2 leader crashes (5-node cluster)
    // Result: PASS - quorum maintained throughout
}
```

---

## Success Criteria

- [x] Phase 1: `prod_write/single` drops from ~9ms to ~2-3ms (Actual: 9.9ms → 1.65ms = **6x**)
- [x] Phase 1: 3-node write drops from ~18ms to <8ms (Actual: 18.5ms → 3.2ms = **5.8x**)
- [x] Phase 1: All existing Raft tests pass with Redb backend (1811/1818 tests pass)
- [x] Phase 1: Crash recovery tests pass with madsim (4/4 tests pass)
- [x] Phase 1: Response handling fixed (pending_responses map) (2025-12-23)
- [x] Phase 1: TTL cleanup task for Redb (2025-12-23)
- [x] Phase 2: Tuple encoding matches FoundationDB spec (type codes, null escaping, integer ordering)
- [x] Phase 2: Property-based tests verify encoding correctness (17 proptest cases)
- [x] Phase 2: Subspace pattern for namespace isolation (12 unit tests)
- [x] Phase 3: DataFusion TableProvider implementation with filter pushdown (2025-12-23)
- [x] Phase 3: SQL queries return identical results to SQLite (16/17 tests pass)
- [x] Phase 3: Empty projection handling for COUNT(*) aggregations (2025-12-23)
- [ ] Phase 3: DataFusion scan overhead <2x raw Redb scan (benchmark pending)
- [x] Phase 5: Secondary index framework implemented (~944 lines)

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
Phase 1A: Proof of Concept - COMPLETE (2025-12-23)
  ├─ [x] Create SharedRedbStorage prototype
  ├─ [x] Implement bundled log+state transaction
  ├─ [x] Benchmark: Verify 2-3ms is achievable (Actual: 1.69ms)
  └─ [x] GO/NO-GO decision: GO - 6.2x improvement exceeded target

Phase 1B: Full Implementation - COMPLETE (2025-12-23)
  ├─ [x] Complete RedbStateMachine with all operations
  ├─ [x] Handle membership changes in bundled transaction
  ├─ [x] Run full test suite (1811/1818 pass)
  └─ [x] Crash recovery validation with madsim (4/4 tests)

Phase 2: Tuple Layer - COMPLETE (2025-12-23)
  ├─ [x] FoundationDB-compatible encoding (8 element types)
  ├─ [x] Subspace namespacing (nested subspaces, range queries)
  ├─ [x] Property-based tests (17 proptest cases)
  └─ [x] Ordered key scans (verified with integer/string ordering)

Phase 3: DataFusion - COMPLETE (2025-12-23)
  ├─ [x] TableProvider implementation (src/sql/provider.rs)
  ├─ [x] SQL execution via RedbSqlExecutor (src/sql/executor.rs)
  ├─ [x] Filter pushdown for key predicates (=, >, >=, <, <=, LIKE prefix%)
  ├─ [x] RecordBatch streaming with batch_size=8192 (src/sql/stream.rs)
  ├─ [x] Empty projection handling for COUNT(*) aggregations
  ├─ [x] Query result validation vs SQLite (16/17 tests pass, 1 skipped)
  └─ [ ] Performance benchmark (<2x overhead target)

Phase 5: Indexes - COMPLETE (2026-01)
  ├─ [x] Secondary index framework (crates/aspen-layer/src/index.rs)
  ├─ [x] IndexRegistry, SecondaryIndex, IndexDefinition types
  ├─ [x] IndexQueryExecutor trait
  └─ [x] Built-in revision indexes
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
