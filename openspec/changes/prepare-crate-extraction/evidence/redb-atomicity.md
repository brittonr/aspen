# Redb Single-Fsync Atomicity Evidence

Generated: 2026-04-25T02:24Z

## Invariant

Raft log entries and state-machine mutations must be committed in a single Redb
write transaction so that crash recovery observes either both or neither.

## Code Path Audit

The single-transaction path is implemented in `RedbKvStorage::append()` at
`crates/aspen-redb-storage/src/raft_storage/log_storage.rs`.

### Append flow

```
append() {
    prev_hash = self.append_read_chain_tip()?;          // read lock only
    write_txn = self.db.begin_write()?;                  // single transaction

    (hash, index, has, pending) =
        self.append_process_entries(&write_txn, entries, prev_hash)?;
        // inside this call, for EACH entry:
        //   1. serialize entry -> log_table.insert(index, bytes)
        //   2. compute chain hash -> hash_table.insert(index, hash)
        //   3. apply_request_in_txn -> kv_table, index_table, leases_table mutations
        //   4. sm_meta_table.insert("last_applied_log", log_id)
        //   (all within the same write_txn)

    write_txn.commit()?;                                 // SINGLE fsync
    // post-commit: update chain_tip cache, store pending responses
}
```

### Key observations

1. `begin_write()` opens ONE Redb write transaction
2. `append_process_entries()` opens ALL tables within that transaction:
   - `RAFT_LOG_TABLE`, `CHAIN_HASH_TABLE` (log)
   - `SM_KV_TABLE`, `SM_INDEX_TABLE`, `SM_LEASES_TABLE`, `SM_META_TABLE` (state machine)
3. Each entry's log insert AND state mutation happen on the same tables
4. `commit()` writes everything atomically — Redb guarantees a single fsync
5. `apply()` is a no-op that retrieves pre-computed responses

### Apply path (no-op)

```
apply() {
    // State already applied during append()
    // Just retrieve pending_responses and send through responders
    // Update confirmed_last_applied monotonically
}
```

## Crash Safety Argument

- **Crash before commit()**: Transaction rolls back. No log entry AND no state mutation
  are durable. Raft re-proposes the entry.
- **Crash after commit()**: Both log entry and state mutation are durable. No replay needed.
- **Partial commit impossible**: Redb write transactions are atomic — the OS sees a single
  `fsync()` call that makes the entire transaction durable.

## Structural Equivalence

The `RedbKvStorage` append path in `aspen-redb-storage` is structurally identical to
`SharedRedbStorage` in `aspen-raft`, minus:
- Trust table operations (stay in `aspen-raft`)
- HLC timestamp generation (not needed for reusable KV)
- Log broadcast notifications (Aspen-specific subscriber integration)

The table layout, transaction structure, chain hashing, and single-commit pattern
are preserved exactly.

## Pre-existing Safety Rails

- `aspen-raft` madsim tests verify single-fsync behavior for `SharedRedbStorage`
- `aspen-raft` Verus specs prove chain integrity and crash safety invariants
- These tests continue to pass against the original code path in `aspen-raft`
- The moved code in `aspen-redb-storage` shares the same verified functions
  from `aspen-redb-storage::verified::*`
