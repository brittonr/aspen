# Aspen Storage Architecture

## Default Configuration

As of the latest changes, Aspen uses a **hybrid storage architecture** by default:

- **Storage Backend**: `sqlite` (default)
- **Raft Log**: redb (always, regardless of backend setting)
- **State Machine**: SQLite with optimizations

## Storage Layout

When running `nix run .#cluster`, each node creates these files:

```
/tmp/aspen-cluster/node1/
├── metadata.redb           # Raft metadata (redb)
├── raft-log.db             # Raft log (SQLite when backend=sqlite)
├── state-machine.db        # State machine with vault data (SQLite)
├── state-machine.db-shm    # SQLite shared memory
└── state-machine.db-wal    # SQLite write-ahead log
```

## Why This Hybrid Approach?

### redb for Raft Log

- **Append-only operations**: Optimized for sequential writes
- **Simple key-value**: No need for complex queries
- **Efficient B-tree**: Fast lookups by log index
- **Zero-copy reads**: Better performance for log replay

### SQLite for State Machine

- **Complex queries**: Vault operations, prefix scans, aggregations
- **Rich indexing**: Custom indexes for vault prefix queries
- **Metadata tables**: Track vault statistics and access patterns
- **ACID transactions**: Ensure consistency across operations
- **Mature tooling**: Easy debugging with standard SQLite tools

## Optimizations Applied

### 1. Database Schema Enhancements

```sql
-- Fast prefix queries for vaults
CREATE INDEX idx_kv_prefix ON state_machine_kv(key);

-- Vault metadata tracking
CREATE TABLE vault_metadata (
    vault_name TEXT PRIMARY KEY,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    key_count INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    description TEXT,
    owner TEXT,
    tags TEXT
);

-- Access pattern monitoring
CREATE TABLE vault_access_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    vault_name TEXT NOT NULL,
    operation TEXT NOT NULL,
    key TEXT,
    timestamp INTEGER NOT NULL DEFAULT (unixepoch()),
    duration_us INTEGER
);

CREATE INDEX idx_access_time ON vault_access_log(timestamp);
```

### 2. Query Optimizations

- **Range queries instead of LIKE**: Uses index efficiently
- **Prepared statement caching**: Reduces parse overhead
- **Bounded results**: MAX_BATCH_SIZE limits prevent OOM

### 3. Tiger Style Bounds

```rust
pub const MAX_VAULTS: usize = 1000;
pub const MAX_KEYS_PER_VAULT: usize = 10000;
pub const MAX_VALUE_SIZE: usize = 1_048_576; // 1MB
pub const MAX_SCAN_RESULTS: usize = 1000;
```

## Performance Characteristics

### Vault Operations

- **List vaults**: O(1) with metadata table (vs O(n) scan)
- **Prefix queries**: 10-100x faster with indexes
- **Range scans**: Uses covering index (optimal)
- **Write throughput**: WAL mode for concurrent reads

### Example Query Plan

```sql
EXPLAIN QUERY PLAN
SELECT key FROM state_machine_kv
WHERE key >= 'vault:config:' AND key < 'vault:config:~'

-- Result: SEARCH state_machine_kv USING COVERING INDEX idx_kv_prefix
```

## Configuration Options

### Default (Recommended)

```bash
nix run .#cluster                    # Uses sqlite backend
# or explicitly:
ASPEN_STORAGE=sqlite nix run .#cluster
```

### Pure redb (Experimental)

```bash
ASPEN_STORAGE=redb nix run .#cluster
# Both log and state machine use redb
# Note: Vault queries less optimized
```

### In-Memory (Testing Only)

```bash
ASPEN_STORAGE=inmemory nix run .#cluster
# No persistence, data lost on restart
```

## Storage Backend Comparison

| Feature | SQLite (Default) | redb | In-Memory |
|---------|-----------------|------|-----------|
| **Raft Log** | redb | redb | redb |
| **State Machine** | SQLite | redb | HashMap |
| **Persistence** | ✅ | ✅ | ❌ |
| **Vault Indexes** | ✅ | ❌ | ❌ |
| **Metadata Tables** | ✅ | ❌ | ❌ |
| **Complex Queries** | ✅ | Limited | ❌ |
| **Debug Tools** | sqlite3 CLI | Custom | None |
| **Performance** | Optimized | Good | Best |

## Future Improvements

While the current hybrid architecture works well, potential enhancements include:

1. **Vault-aware triggers**: Auto-update metadata on KV changes
2. **Compression**: Store large values compressed
3. **Partitioning**: Separate tables for hot/cold vaults
4. **Replication lag tracking**: Monitor follower sync status
5. **Query cache**: Cache frequently accessed vault data

## Migration from Previous Versions

If upgrading from an older version:

1. **Backup existing data**: `cp -r /tmp/aspen-cluster /tmp/aspen-cluster.bak`
2. **Start with new defaults**: `nix run .#cluster`
3. **Data is auto-migrated**: New tables/indexes created on first run
4. **No action needed**: Backward compatible with existing KV data

## Debugging

### Inspect SQLite Database

```bash
# View all tables
sqlite3 /tmp/aspen-cluster/node1/state-machine.db ".tables"

# Check vault data
sqlite3 /tmp/aspen-cluster/node1/state-machine.db \
  "SELECT key, LENGTH(value) FROM state_machine_kv WHERE key LIKE 'vault:%' LIMIT 10"

# View vault metadata
sqlite3 /tmp/aspen-cluster/node1/state-machine.db \
  "SELECT * FROM vault_metadata"

# Check query performance
sqlite3 /tmp/aspen-cluster/node1/state-machine.db \
  "EXPLAIN QUERY PLAN SELECT * FROM state_machine_kv WHERE key LIKE 'vault:config:%'"
```

### Monitor Access Patterns

```sql
-- Top accessed vaults in last hour
SELECT vault_name, COUNT(*) as access_count
FROM vault_access_log
WHERE timestamp > unixepoch() - 3600
GROUP BY vault_name
ORDER BY access_count DESC;

-- Slow operations
SELECT * FROM vault_access_log
WHERE duration_us > 10000
ORDER BY duration_us DESC;
```
