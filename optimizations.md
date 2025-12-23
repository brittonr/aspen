# Aspen Performance Optimizations

This document tracks researched and potential performance optimizations for Aspen.

---

## rkyv Zero-Copy Deserialization

**Researched:** 2025-12-19
**Status:** Not Implemented (research complete)

### Overview

rkyv is a zero-copy deserialization framework for Rust that could improve Aspen's serialization performance by 20-40% on critical consensus paths.

### Current Serialization Stack

| Component | Format | Usage |
|-----------|--------|-------|
| Log entries (redb) | bincode | Every Raft append |
| Snapshots (SQLite) | JSON | State machine snapshots |
| RPC messages | postcard | Raft consensus protocol |
| Metadata | bincode | Vote, committed index |

### High-Value Integration Opportunities

#### 1. Raft Log Entry Serialization (CRITICAL)

**Location:** `src/raft/storage.rs:940-967`

```rust
// Current: bincode::serialize per entry
for entry in entries {
    let data = bincode::serialize(&entry).context(SerializeSnafu)?;
    serialized_entries.push((index, term, data, entry_hash));
}
```

- Called on every Raft consensus write
- Up to 1,000 entries per batch (MAX_BATCH_SIZE)
- **Estimated improvement:** 20-35% faster appends

#### 2. State Machine Snapshots (HIGH IMPACT)

**Location:** `src/raft/storage_sqlite.rs:1358, 1576`

```rust
// Current: JSON for snapshot state
let snapshot_data = serde_json::to_vec(&data)?;
let new_data: BTreeMap<String, String> = serde_json::from_slice(&snapshot_data)?;
```

- Snapshots up to 100 MB
- JSON is slow and verbose
- **Estimated improvement:** 40-60% faster restore, 2-3x smaller size

#### 3. Raft RPC Messages (NETWORK)

**Location:** `src/raft/network.rs:349, 382, 397`

- Every heartbeat/replication uses this path
- AppendEntries contains batched log entries
- **Estimated improvement:** 15-25% lower RPC latency

#### 4. Chain Integrity Verification

**Location:** `src/raft/storage.rs:1167-1239`

```rust
// Current: Deserialize just to extract term
let entry: Entry = bincode::deserialize(&entry_bytes)?;
let term = entry.log_id.leader_id.term;
```

- Verification loop deserializes 1,000 entries
- Only needs term field
- **Estimated improvement:** 25-40% faster verification

#### 5. Metadata Persistence

**Location:** `src/raft/storage.rs:776-810`

- vote, committed, last_purged_log_id
- **Estimated improvement:** 5-10% faster startup

### Not Recommended for rkyv

- **Gossip messages:** Too small (100-500 bytes)
- **Client RPC:** Would break existing postcard API
- **Cluster tickets:** Infrequent operations

### Implementation Notes

If implementing in the future:

1. Start with log entries in RedbLogStore (highest frequency)
2. Add rkyv derives to vendored openraft types (Entry, LogId, Vote)
3. Implement dual-format detection for backward compatibility
4. Add criterion benchmarks to validate improvements

### Architecture Considerations

- **redb:** Already stores `&[u8]` - rkyv bytes work directly
- **SQLite BLOBs:** May need alignment handling
- **openraft types:** Require derives in vendored copy
- **Validation:** Use `rkyv::access` with validation for untrusted data

### References

- [rkyv Documentation](https://rkyv.org/)
- [Zero-copy deserialization](https://rkyv.org/zero-copy-deserialization.html)
- [Apache Iggy on zero-copy (2025)](https://iggy.apache.org/blogs/2025/05/08/zero-copy-deserialization/)
