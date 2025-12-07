# ADR-004: redb for Raft State Machine Storage

**Status:** Accepted - In Progress (70% Complete)

**Date:** 2025-12-03
**Updated:** 2025-12-03

**Context:** Aspen requires a persistent storage engine for Raft's log and state machine. The storage layer must provide ACID guarantees, perform efficiently under concurrent workloads, and align with Tiger Style principles.

## Decision

Use **redb** as the embedded storage engine for Raft's state machine and log persistence, with an in-memory implementation for testing.

## Current Implementation

The current implementation (`src/raft/storage.rs`) uses in-memory storage backed by `BTreeMap`:

```rust
#[derive(Debug, Default)]
struct LogStoreInner {
    last_purged_log_id: Option<LogIdOf<AppTypeConfig>>,
    log: BTreeMap<u64, <AppTypeConfig as openraft::RaftTypeConfig>::Entry>,
    committed: Option<LogIdOf<AppTypeConfig>>,
    vote: Option<VoteOf<AppTypeConfig>>,
}
```

State machine operations are performed on an in-memory `BTreeMap<String, String>`:

```rust
#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct StateMachineData {
    pub last_applied_log: Option<openraft::LogId<AppTypeConfig>>,
    pub last_membership: StoredMembership<AppTypeConfig>,
    pub data: BTreeMap<String, String>,
}
```

## Rationale

### Why redb?

1. **ACID Guarantees**: redb provides full ACID transactions with MVCC (Multi-Version Concurrency Control), ensuring linearizable state machine operations required by Raft consensus.

2. **Tiger Style Alignment**:
   - **Static memory allocation**: redb uses memory-mapped files with predictable memory usage
   - **Explicit limits**: File size and page counts can be bounded at initialization
   - **Zero-copy reads**: Memory-mapped architecture avoids unnecessary data copying
   - **Fail-fast**: redb panics on corruption rather than returning corrupted data

3. **Performance**:
   - Memory-mapped I/O provides near-memory speeds for hot data
   - B-tree structure optimizes range scans (critical for log replication)
   - Copy-on-write design minimizes write amplification

4. **Type Safety**: Strongly-typed tables prevent serialization errors at compile time:

   ```rust
   // Example (planned):
   const LOG_TABLE: TableDefinition<u64, &[u8]> = TableDefinition::new("raft_log");
   ```

5. **Simplicity**: Pure Rust with no C dependencies, single-file database, no separate daemon process.

### Tiger Style Principles Applied

From `tigerstyle.md`:

- **Explicitly sized types**: redb keys are `u64` log indices, not `usize`
- **Fixed limits**: Database file size set at creation, preventing unbounded growth
- **Static allocation**: Memory-mapped regions allocated at startup
- **Predictable performance**: B-tree operations have O(log n) guarantees
- **Fail fast**: Database corruption is detected immediately on open

## Alternatives Considered

### 1. sled

**Pros:**

- Pure Rust embedded database
- Good write performance

**Cons:**

- Less mature (beta quality)
- Higher memory usage due to in-memory index
- Complex crash recovery model
- Project maintenance concerns

**Rejected:** Stability and maturity concerns make sled unsuitable for production consensus storage.

### 2. RocksDB

**Pros:**

- Battle-tested (used in TiKV, CockroachDB, etcd)
- Excellent write throughput via LSM-tree
- Rich tuning options

**Cons:**

- C++ dependency (FFI overhead, build complexity)
- Complex tuning required for optimal performance
- Write amplification from compaction
- Against Tiger Style preference for Rust-native solutions

**Rejected:** Foreign dependencies and operational complexity outweigh performance benefits.

### 3. LMDB

**Pros:**

- Extremely stable and mature
- Zero-copy reads via memory mapping
- Used in production databases (OpenLDAP)

**Cons:**

- C dependency (similar to RocksDB issues)
- Copy-on-write can cause large file growth
- Limited documentation for Rust bindings

**Rejected:** C dependency and Tiger Style preference for Rust-native solutions.

### 4. Custom Implementation

**Pros:**

- Full control over design
- Tailored specifically to Raft's access patterns

**Cons:**

- High development cost
- Database engines are notoriously complex
- Risk of subtle correctness bugs

**Rejected:** Violates "zero technical debt" principle. Using a proven storage engine is safer than building from scratch.

## Consequences

### Positive

1. **Durability**: Raft state survives process restarts
2. **Consistency**: ACID transactions ensure state machine determinism
3. **Performance**: Memory-mapped I/O provides low-latency access
4. **Simplicity**: Single-file database simplifies backup/restore operations

### Negative

1. **File I/O overhead**: Slower than pure in-memory for testing (mitigated by keeping in-memory implementation)
2. **Disk space**: Log history requires periodic compaction via Raft snapshots

### Migration Path

The current in-memory implementation will remain for:

- Unit tests (faster, deterministic)
- Madsim simulations (no file I/O in simulations)

Production deployments will use redb-backed storage.

## Implementation Notes

The redb integration will:

1. Maintain compatibility with openraft's `RaftLogStorage` and `RaftStateMachine` traits
2. Use separate tables for log entries, committed index, vote, and state machine data
3. Implement atomic snapshot operations using redb transactions
4. Set fixed database size limits (e.g., 10 GB max) at creation

Example table schema (planned):

```rust
const RAFT_LOG: TableDefinition<u64, &[u8]> = TableDefinition::new("log");
const RAFT_STATE: TableDefinition<&str, &[u8]> = TableDefinition::new("state");
const STATE_MACHINE_KV: TableDefinition<&str, &str> = TableDefinition::new("kv");
```

## Implementation Progress (2025-12-03)

**Status: 70% Complete**

### ✅ Completed

- `StorageBackend` enum for InMemory/Redb selection
- `RedbLogStore` (~270 lines) fully implementing `RaftLogStorage`:
  - Vote persistence (`save_vote`, `read_vote`)
  - Committed index tracking (`save_committed`, `read_committed`)
  - Log operations (`append`, `truncate`, `purge`, `get_log_state`)
  - ACID transactions with proper error handling
- `RedbStateMachine` (~370 lines) fully implementing `RaftStateMachine`:
  - KV data persistence (`apply`)
  - Snapshot building and installation
  - Metadata tracking (last_applied_log, last_membership)
  - ACID guarantees via redb transactions
- Persistence tests: `test_redb_log_persistence`, `test_redb_state_machine_persistence` (both passing)
- Tiger Style compliance: explicit u64 types, bounded operations, fail-fast semantics

### ⏸️ In Progress

- OpenRaft comprehensive test suite: 1 failure in `test_redb_storage_suite` (last_applied_log state sync issue)
- Bootstrap integration: need to wire Redb storage into `bootstrap_node()`
- Configuration: add backend selection to `ClusterBootstrapConfig`

### 📊 Test Results

```
test_redb_log_persistence            PASS ✅ (0.167s)
test_redb_state_machine_persistence  PASS ✅
test_redb_storage_suite              FAIL ❌ (assertion failure - investigating)
```

**Next Steps**: Debug storage suite failure, integrate into bootstrap, validate all existing tests pass.

## References

- redb documentation: <https://docs.rs/redb>
- OpenRaft storage traits: `src/raft/storage.rs`
- Tiger Style philosophy: `tigerstyle.md`
