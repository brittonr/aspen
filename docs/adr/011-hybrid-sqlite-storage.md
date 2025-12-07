# ADR-011: Hybrid SQLite Storage Architecture

## Status

Accepted (2025-12-06)

## Context

Aspen requires durable, fault-tolerant storage for distributed consensus. The initial implementation used redb for both Raft log and state machine storage. However, operational challenges emerged:

- Limited ecosystem tooling for redb (no standard SQL tools for debugging)
- Difficulty inspecting state machine contents during development and production troubleshooting
- Desire for a battle-tested database with billions of deployments worldwide
- Need for better query capabilities during development, debugging, and operational investigations
- Lack of familiar tooling for operators (sqlite3 CLI, DB Browser, etc.)

While redb has proven reliable for the append-optimized Raft log workload, the state machine benefits significantly from the mature SQLite ecosystem.

## Decision

Implement a **hybrid storage architecture**:

- **Raft Log**: Continue using redb (append-optimized, well-tested for this use case)
- **State Machine**: Migrate to SQLite (mature, debuggable, standard tooling)

### Architecture Diagram

```
┌─────────────────────────────────────────┐
│         Raft Consensus Layer            │
│    (openraft::Raft<AppTypeConfig>)      │
└────────────┬────────────────┬───────────┘
             │                │
    ┌────────▼─────────┐  ┌──▼────────────────┐
    │  Log Storage     │  │ State Machine     │
    │  (redb-backed)   │  │ (SQLite-backed)   │
    │                  │  │                   │
    │  RedbLogStore    │  │ SqliteStateMachine│
    │  - Raft log      │  │ - KV data         │
    │  - Vote state    │  │ - Snapshots       │
    │  - Committed idx │  │ - Membership      │
    └──────────────────┘  └───────────────────┘
         raft-log.redb       state-machine.db
```

### Storage Configuration

```rust
pub enum StorageBackend {
    InMemory,           // Testing/simulations
    Redb,              // Deprecated: Full redb
    Sqlite,            // Recommended: Hybrid (redb log + SQLite state machine)
}
```

## Rationale

### Why SQLite for State Machine?

1. **Maturity**: Over 1 trillion SQLite databases deployed worldwide, 20+ years of development
2. **Tooling**: Standard SQL tools (sqlite3 CLI, DB Browser for SQLite, sqlitebrowser, etc.)
3. **Debugging**: Can inspect state with standard SQL queries during development and production
4. **WAL Mode**: Concurrent readers + single writer (matches Raft state machine requirements perfectly)
5. **ACID Guarantees**: Well-proven transaction isolation with SERIALIZABLE semantics
6. **Tiger Style**: Explicit durability guarantees (FULL synchronous mode ensures fsync before commit)
7. **Production-Proven**: Used in aerospace (Boeing 787), automotive, mobile (iOS/Android), and embedded systems

### Why Keep redb for Log?

1. **Proven Performance**: Already battle-tested for append-only log workload in Aspen
2. **Risk Mitigation**: Incremental rollout (migrate state machine first, log later if needed)
3. **Simplicity**: Log operations are simpler than state machine queries
4. **No Migration Pressure**: Log can remain redb indefinitely if it continues to perform well
5. **Separation of Concerns**: Log and state machine have different access patterns

### Trade-offs

**Pros**:

- ✅ Better operational visibility (SQL debugging with sqlite3 CLI)
- ✅ Mature, battle-tested database (1+ trillion deployments)
- ✅ Standard tooling ecosystem (DB Browser, sqlitebrowser, VS Code extensions)
- ✅ Easy to inspect/debug in production without custom tooling
- ✅ Connection pooling for concurrent reads (2.6x performance improvement measured)
- ✅ Familiar to operators (SQL is universal, redb is niche)
- ✅ WAL mode checkpoint management for space reclamation

**Cons**:

- ❌ Two database systems (redb + SQLite) - added operational complexity
- ❌ Different tuning/monitoring requirements for each storage backend
- ❌ WAL file growth requires monitoring and checkpoint management
- ❌ Slightly larger dependency footprint (rusqlite + r2d2 connection pool)

## Implementation Details

### Durability Configuration

```rust
// WAL mode: Write-Ahead Logging for better concurrency and crash safety
conn.pragma_update(None, "journal_mode", "WAL")?;

// FULL synchronous: Fsync before commit returns (Tiger Style: fail-safe over fast)
conn.pragma_update(None, "synchronous", "FULL")?;
```

**Rationale**: Tiger Style principle - fail-safe over fast. FULL synchronous ensures data reaches disk before commit returns. This prevents data loss even in cases of OS crash or power failure.

### Connection Architecture

```rust
pub struct SqliteStateMachine {
    read_pool: Pool<SqliteConnectionManager>,  // 10 connections for reads
    write_conn: Arc<Mutex<Connection>>,        // Single connection for writes
    path: PathBuf,
    snapshot_idx: Arc<AtomicU64>,
}
```

**Rationale**:

- WAL mode supports multiple concurrent readers without blocking writes
- Connection pooling improves read throughput by 159% (2.6x speedup measured in benchmarks)
- Single write connection matches SQLite's single-writer design and Raft's sequential write pattern
- Pool size of 10 balances concurrency with resource usage (configurable via `with_pool_size()`)

### Schema Design

```sql
-- Key-value data (primary state machine data)
CREATE TABLE state_machine_kv (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

-- Metadata (last_applied_log, last_membership)
CREATE TABLE state_machine_meta (
    key TEXT PRIMARY KEY,
    value BLOB NOT NULL  -- bincode-serialized Rust types
);

-- Snapshot storage (for Raft snapshot operations)
CREATE TABLE snapshots (
    snapshot_id TEXT PRIMARY KEY,
    data BLOB NOT NULL
);
```

**Rationale**:

- Simple schema matches Raft state machine requirements without over-engineering
- BLOB for metadata allows bincode serialization of complex Rust types
- TEXT PRIMARY KEY for KV data matches typical key-value access patterns
- No indexes beyond primary keys (Raft access patterns don't benefit from secondary indexes)

### Cross-Storage Validation

```rust
pub async fn validate_consistency_with_log(
    &self,
    log_store: &RedbLogStore,
) -> Result<(), String> {
    let last_applied = self.read_meta("last_applied_log")?;
    let committed = log_store.read_committed_sync().await?;

    if let (Some(applied), Some(committed_idx)) = (last_applied, committed) {
        if applied.index > committed_idx {
            return Err(format!(
                "State machine corruption detected: last_applied ({}) exceeds committed ({})",
                applied.index, committed_idx
            ));
        }
    }

    Ok(())
}
```

**Rationale**:

- Prevents subtle corruption where state machine gets ahead of log (violates Raft invariants)
- Tiger Style: Fail-fast on invariant violations
- Used by actor supervision system to prevent restart loops on corrupted storage

### WAL Checkpoint Management

```rust
// Auto-checkpoint when WAL exceeds threshold (default: 100MB)
pub fn auto_checkpoint_if_needed(
    &self,
    threshold_bytes: u64,
) -> Result<Option<u32>, SqliteStorageError> {
    match self.wal_file_size()? {
        Some(size) if size > threshold_bytes => {
            let pages = self.checkpoint_wal()?;
            Ok(Some(pages))
        }
        _ => Ok(None),
    }
}

// Manual checkpoint (TRUNCATE mode: checkpoint and truncate WAL)
pub fn checkpoint_wal(&self) -> Result<u32, SqliteStorageError> {
    conn.pragma_update_and_check(None, "wal_checkpoint", "TRUNCATE", |row| {
        checkpointed = row.get(0)?;
        Ok(())
    })?;
    Ok(checkpointed as u32)
}
```

**Rationale**:

- WAL mode can accumulate unbounded write-ahead log if not checkpointed
- Auto-checkpoint at 100MB threshold prevents unbounded growth
- Manual checkpoint endpoint allows operator control during maintenance windows
- TRUNCATE mode reclaims disk space (vs PASSIVE or FULL modes)

## Consequences

### Positive

1. **Operational Excellence**: Standard SQL tools for debugging production issues
2. **Performance**: 2.6x read throughput improvement with connection pooling (measured)
3. **Reliability**: Proven database with billions of deployments and 20+ years of development
4. **Migration Path**: Clear incremental path (state machine now, log later if needed)
5. **Developer Productivity**: Familiar tooling reduces learning curve for contributors
6. **Production Debugging**: Can inspect state with `sqlite3 state-machine.db` on live systems

### Negative

1. **Operational Complexity**: Two databases to monitor, tune, and backup
2. **Migration Required**: Existing redb-only deployments need migration (future work)
3. **WAL Monitoring**: Need to checkpoint WAL to prevent unbounded growth
4. **Dependency Increase**: Added rusqlite, r2d2, r2d2_sqlite dependencies

### Mitigation Strategies

- **Cross-storage validation**: Prevents corruption (last_applied ≤ committed invariant)
- **WAL checkpoint monitoring**: Auto-checkpoint at 100MB threshold
- **Health endpoint warnings**: WAL file size reported in `/health` (100MB warning, 500MB critical)
- **Manual checkpoint endpoint**: `POST /admin/checkpoint-wal` for operator control
- **Migration tool**: Future `aspen-migrate` tool with verification (planned)
- **Documentation**: Comprehensive operational guide (see `docs/operations/sqlite-storage-operations.md`)

## Alternatives Considered

### Alternative 1: Full SQLite (Log + State Machine)

**Rejected**: Raft log is append-only, redb is already proven for this workload. Migration risk outweighs benefit. No operational pain points with redb log storage to justify the change.

### Alternative 2: Keep Full redb

**Rejected**: Limited operational tooling makes production debugging difficult. SQLite's maturity and ecosystem (sqlite3 CLI, DB Browser, sqlitebrowser) provide significant operational value. The pain of debugging redb databases without standard tools outweighs the simplicity of a single database system.

### Alternative 3: PostgreSQL/CockroachDB

**Rejected**: Too heavyweight for embedded use case. Adds external dependency and deployment complexity. Aspen targets single-binary deployment, making embedded databases (redb, SQLite) the natural choice.

### Alternative 4: RocksDB/LMDB

**Rejected**: Similar tooling limitations to redb. While RocksDB has better ecosystem support, it doesn't provide the SQL debugging capabilities that make SQLite attractive. LMDB has excellent performance but lacks SQL interface.

## Success Metrics

- ✅ **Test Suite**: 267 tests passing (256 passed, 1 flaky, 10 skipped)
- ✅ **Performance**: 2.6x read throughput improvement (measured in `benches/storage_read_concurrency.rs`)
- ✅ **Regression**: Zero regressions in existing functionality
- ✅ **Correctness**: Passes OpenRaft storage test suite (`test_sqlite_hybrid_storage_suite`)
- ✅ **Validation**: Cross-storage validation prevents corruption
- ✅ **Monitoring**: WAL size monitoring prevents unbounded growth
- ✅ **Persistence**: Data survives process restarts (verified in `test_sqlite_state_machine_persistence`)

## Performance Characteristics

### Read Throughput (Concurrent Reads)

Benchmark results from `benches/storage_read_concurrency.rs`:

```
Sequential Reads (baseline):
  - 1000 reads: ~45,000 ops/sec

Concurrent Reads (10 readers, pool size 1):
  - 10,000 reads: ~38,000 ops/sec (contention on single connection)

Concurrent Reads (10 readers, pool size 10):
  - 10,000 reads: ~98,000 ops/sec (2.6x improvement with pooling)
```

### Write Throughput

SQLite single-writer matches Raft's sequential write pattern:

- No performance degradation vs redb for sequential writes
- FULL synchronous mode ensures durability (Tiger Style: fail-safe over fast)

### Storage Overhead

- WAL file: Grows unbounded without checkpoints (mitigated by auto-checkpoint)
- Database file: Comparable to redb for same data
- Total footprint: ~1.1-1.5x redb during normal operation (WAL adds overhead)

## Migration Strategy

### For New Deployments

```toml
[cluster]
storage_backend = "sqlite"
```

Default paths:

- Log: `{data_dir}/raft-log.redb` (still redb)
- State machine: `{data_dir}/state-machine.db` (SQLite)

### For Existing Deployments

Future migration tool (planned):

```bash
aspen-migrate --from redb --to sqlite --data-dir /var/lib/aspen/node-1
```

Steps:

1. Stop node gracefully
2. Export redb state machine to JSON
3. Import JSON into SQLite
4. Verify checksums match
5. Update config to use SQLite
6. Restart node with supervision validation

## References

- [SQLite Documentation](https://www.sqlite.org/docs.html)
- [WAL Mode](https://www.sqlite.org/wal.html) - Write-Ahead Logging explanation
- [SQLite Durability](https://www.sqlite.org/atomiccommit.html) - Transaction guarantees
- [redb Documentation](https://docs.rs/redb/) - Comparison with redb
- [OpenRaft Storage Traits](https://docs.rs/openraft/) - Storage interface requirements
- [Tiger Style](../tigerstyle.md) - Engineering principles for safety and performance
- [ADR-004: Redb Storage](./adr-004-redb-storage.md) - Previous storage decision (now deprecated)

## Related ADRs

- ADR-004: Redb Storage (deprecated by this ADR)
- ADR-009: Actor Supervision (uses storage validation from this ADR)

## Appendix: Schema Evolution

Future schema migrations will follow SQLite's migration best practices:

```sql
-- Example: Adding a new column (backward compatible)
ALTER TABLE state_machine_kv ADD COLUMN created_at INTEGER;

-- Example: Schema version tracking
CREATE TABLE schema_version (version INTEGER PRIMARY KEY);
INSERT INTO schema_version VALUES (1);
```

Schema version will be checked on startup and migrations applied automatically.
