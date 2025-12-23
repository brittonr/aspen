# MVCC State Analysis for Aspen

**Created**: 2025-12-22
**Analysis Type**: ULTRA MODE Deep Investigation
**Topic**: Multi-Version Concurrency Control (MVCC) Implementation Status

---

## Executive Summary

Aspen implements **Optimistic Concurrency Control (OCC)**, NOT true Multi-Version Concurrency Control (MVCC). This is a deliberate architectural choice that provides linearizable semantics via Raft consensus with simpler implementation complexity than FoundationDB-style MVCC.

| Aspect | Status | Implementation |
|--------|--------|----------------|
| **OCC (Optimistic Concurrency Control)** | Complete | `apply_optimistic_transaction()` in storage_sqlite.rs |
| **Per-Key Version Tracking** | Complete | `version`, `create_revision`, `mod_revision` columns |
| **Conflict Detection** | Complete | Read set version validation at commit time |
| **True MVCC (Multi-Version Visibility)** | Not Implemented | Would require historical version storage |
| **Snapshot Isolation** | Not Implemented | All reads see latest committed state |
| **Time-Windowed Reads** | Not Implemented | No 5-second visibility window like FoundationDB |

---

## What Aspen HAS (OCC Implementation)

### 1. OptimisticTransaction WriteCommand

**Location**: `src/api/mod.rs:520-563`

```rust
WriteCommand::OptimisticTransaction {
    read_set: Vec<(String, i64)>,  // Keys with expected versions
    write_set: Vec<WriteOp>,       // Set/Delete operations
}
```

**How it works**:

1. Client reads keys and captures their `version` values
2. Client computes new values based on reads
3. Client submits transaction with `read_set` (version checks) and `write_set`
4. Server validates all versions still match
5. If conflict: returns error with `conflict_key`, `expected_version`, `actual_version`
6. If no conflict: applies write set atomically

### 2. Version Tracking Schema

**Location**: `src/raft/storage_sqlite.rs:860-870`

```sql
CREATE TABLE state_machine_kv (
    key TEXT PRIMARY KEY,
    value BLOB,
    version INTEGER NOT NULL DEFAULT 1,      -- Per-key counter (1, 2, 3...)
    create_revision INTEGER NOT NULL,        -- Raft log index at creation
    mod_revision INTEGER NOT NULL,           -- Raft log index at last modification
    expires_at_ms INTEGER,                   -- TTL support
    lease_id INTEGER                         -- Lease association
);

CREATE INDEX idx_kv_mod_revision ON state_machine_kv(mod_revision);
CREATE INDEX idx_kv_create_revision ON state_machine_kv(create_revision);
```

### 3. OCC Conflict Detection Implementation

**Location**: `src/raft/storage_sqlite.rs:2610-2703`

```rust
fn apply_optimistic_transaction(
    conn: &Connection,
    log_index: u64,
    read_set: &[(String, i64)],
    write_set: &[(bool, String, String)],
) -> Result<AppResponse, io::Error> {
    // Phase 1: Validate read set - check all versions match
    for (key, expected_version) in read_set {
        let current_version = query_version(conn, key)?;
        if current_version != *expected_version {
            return Ok(AppResponse {
                occ_conflict: Some(true),
                conflict_key: Some(key.clone()),
                conflict_expected_version: Some(*expected_version),
                conflict_actual_version: Some(current_version),
                ..Default::default()
            });
        }
    }

    // Phase 2: Apply write set with version tracking
    for (is_set, key, value) in write_set {
        if *is_set {
            let (new_version, create_rev) = match get_existing(conn, key)? {
                Some((version, create_rev)) => (version + 1, create_rev),
                None => (1, log_index as i64),
            };
            insert_or_replace(conn, key, value, new_version, create_rev, log_index)?;
        } else {
            delete(conn, key)?;
        }
    }

    Ok(AppResponse { occ_conflict: Some(false), ... })
}
```

### 4. TransactionBuilder Client API

**Location**: `src/client/transaction.rs` (284 lines)

```rust
// Build and execute an optimistic transaction
let result = TransactionBuilder::new()
    .read("user:123", current_version)
    .read("balance:123", balance_version)
    .set("user:123", "updated-data")
    .set("balance:123", new_balance.to_string())
    .execute(&kv_store)
    .await?;

match result {
    TransactionResult::Committed => println!("Success"),
    TransactionResult::Conflict { key, expected, actual } => {
        println!("Conflict on {key}: expected {expected}, got {actual}");
        // Retry logic (client responsibility)
    }
}
```

### 5. Version Tracking Across All Write Operations

All write operations properly track versions:

| Operation | Location | Version Behavior |
|-----------|----------|------------------|
| `apply_set()` | lines 2107-2134 | Increments version, preserves create_revision |
| `apply_set_multi()` | lines 2155-2215 | Same, applied to each key |
| `apply_batch()` | lines 2448-2497 | Set operations increment version |
| `apply_conditional_batch()` | lines 2534-2595 | Set operations increment version |
| `apply_compare_and_swap()` | lines 2269-2336 | Increments on success |
| `apply_optimistic_transaction()` | lines 2610-2703 | Full OCC validation + version tracking |

---

## What Aspen LACKS (True MVCC)

### 1. No Snapshot Isolation

**FoundationDB Behavior**:

- Transactions read from a consistent snapshot at a specific read version
- Readers see exactly the state at that version, even if keys are modified during the transaction
- 5-second visibility window allows concurrent readers to see historical versions

**Aspen Behavior**:

- All reads see the latest committed version
- No ability to read "as of" a specific version
- Readers at different times see different data for the same keys

### 2. No Multi-Version Storage

**FoundationDB Behavior**:

- Storage servers maintain a skiplist of (key, version, value) tuples
- Multiple versions of a key coexist in storage
- Older versions pruned after 5-second window
- Readers can query any version within the window

**Aspen Behavior**:

- SQLite stores only the latest version per key
- No version history maintained
- Deletes remove keys entirely (no tombstones with versions)
- Cannot query historical versions

### 3. No Read Version Timestamp

**FoundationDB Behavior**:

- Client obtains a "read version" at transaction start
- All reads use this version for consistency
- Writes commit at a "commit version" > read version
- Conflict detection: "Has any key in my read set been written between read_version and commit_version?"

**Aspen Behavior**:

- Reads return the current version of each key at read time
- No "transaction timestamp" concept
- Conflict detection: "Has the version changed since I read it?"
- Works for correctness, but different semantics

### 4. No Time-Windowed Visibility

**FoundationDB Behavior**:

- Resolvers maintain version history for ~5 seconds
- Enables parallel conflict detection across resolvers
- Allows "stale reads" for lower latency

**Aspen Behavior**:

- No time-based visibility window
- All reads are linearizable (via Raft ReadIndex)
- No stale read option at version level (only consistency level: Linearizable vs Stale)

---

## Comparison Matrix

| Feature | Aspen | FoundationDB | etcd |
|---------|-------|--------------|------|
| **Versioning Approach** | OCC with per-key version counters | True MVCC (5-second window) | MVCC with revision history |
| **Read Consistency** | Linearizable (via Raft) | Snapshot isolation | Linearizable or stale |
| **Snapshot Isolation** | No (all reads see latest) | Yes (time-windowed) | No |
| **Multi-Version Visibility** | No (one version per key) | Yes (readers see historical) | Limited (revision history) |
| **Transaction Window** | Single Raft log index | 5 seconds (configurable) | Based on revision index |
| **Version Counter** | Per-key (1, 2, 3...) | Logical timestamp (global) | Global revision number |
| **Conflict Detection** | Version mismatch at commit | Read-write set intersection | Revision-based |
| **Historical Reads** | No | Yes (within window) | Via revision (limited) |

---

## Architectural Decision: Why OCC Instead of MVCC?

### Advantages of Aspen's OCC Approach

1. **Simpler Implementation**: Single-version storage is much simpler than multi-version
2. **Lower Storage Overhead**: No version history to maintain
3. **Consistent with Raft Semantics**: Linearizable operations align with Raft's model
4. **Sufficient for Coordination**: OCC covers most coordination use cases (locks, counters, elections)
5. **SQLite Compatibility**: Leverages SQLite's ACID transactions without custom storage

### When True MVCC Would Be Needed

1. **Long-Running Transactions**: Transactions that span seconds, not milliseconds
2. **Analytics Queries**: Reading consistent snapshots while writes continue
3. **Parallel Conflict Detection**: Multiple resolvers processing conflict ranges
4. **Stale Reads at Specific Versions**: "Read as of 5 seconds ago"

---

## Implementation Gaps and TODOs

### Current Gaps (Minor Impact)

1. **In-Memory Test Backend**: Now implements OCC validation (updated 2025-12-22)
2. **Delete Version Tombstones**: Deletes reset version to 0, not versioned tombstones
3. **Automatic Retry**: Client responsibility, no built-in retry with backoff

### Future MVCC Enhancement Path (If Needed)

If true MVCC is ever required, the implementation path would be:

1. **Schema Change**: Add version column to primary key

   ```sql
   CREATE TABLE state_machine_kv (
       key TEXT,
       version INTEGER,  -- Part of primary key
       value BLOB,
       timestamp_ns INTEGER,
       PRIMARY KEY (key, version)
   );
   ```

2. **Version History Pruning**: Background task to prune versions older than window

   ```rust
   fn prune_old_versions(conn: &Connection, cutoff_ns: i64) {
       conn.execute(
           "DELETE FROM state_machine_kv
            WHERE timestamp_ns < ?1
            AND (key, version) NOT IN (
                SELECT key, MAX(version) FROM state_machine_kv GROUP BY key
            )",
           params![cutoff_ns]
       )?;
   }
   ```

3. **Read Version Assignment**: Assign read version at transaction start
4. **Version-Aware Reads**: Query max version <= read_version
5. **Skiplist Optimization**: Consider in-memory structure for hot path conflict detection

**Estimated Complexity**: 4-8 weeks of focused development
**Recommendation**: Current OCC is sufficient for most use cases. Consider MVCC only if specific requirements emerge.

---

## Test Coverage

### OCC Tests (Existing)

- `src/api/mod.rs:validation_tests` - WriteCommand validation
- `src/client/transaction.rs:tests` - TransactionBuilder unit tests
- `src/raft/storage_sqlite.rs` - Integration tests with SQLite
- `tests/madsim_replication_test.rs` - Distributed OCC under simulation

### Recommended Additional Tests

1. **Concurrent OCC conflicts**: Two transactions reading same key, both trying to write
2. **Read set spanning multiple keys**: Validate all-or-nothing semantics
3. **Large read/write sets**: Tiger Style limit validation
4. **Conflict recovery**: Client retry patterns

---

## Sources

### Aspen Implementation

- `src/api/mod.rs:520-563` - OptimisticTransaction definition
- `src/client/transaction.rs` - TransactionBuilder client API
- `src/raft/storage_sqlite.rs:2610-2703` - OCC apply logic
- `src/raft/types.rs:507-524` - AppResponse OCC fields

### FoundationDB Reference

- [Transaction Processing](https://apple.github.io/foundationdb/transaction-processing.html)
- [SIGMOD Paper](https://www.foundationdb.org/files/fdb-paper.pdf) - Section on MVCC and resolvers

### etcd Reference

- [API Guarantees](https://etcd.io/docs/v3.5/learning/api_guarantees/)

---

## Conclusion

Aspen has a **complete and production-ready OCC implementation** that provides:

- Per-key version tracking
- Atomic conflict detection at commit time
- TransactionBuilder client API
- Full integration with Raft consensus

It does **NOT** have true MVCC, which would require:

- Multi-version storage
- Time-windowed visibility
- Snapshot isolation semantics

This is an intentional design choice that prioritizes simplicity and aligns with Aspen's coordination-focused use case. True MVCC can be added later if specific requirements emerge, but the current OCC implementation is sufficient for most distributed coordination patterns.
