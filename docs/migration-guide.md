# Aspen Storage Migration Guide

This guide explains how to migrate Aspen Raft state machine data from redb to SQLite format.

## Overview

Aspen supports two persistent storage backends for the Raft state machine:

- **redb**: Legacy embedded ACID storage (deprecated)
- **SQLite**: Current recommended storage with WAL mode and connection pooling

The migration tool (`aspen-migrate`) safely converts existing redb databases to SQLite format with optional verification.

## Prerequisites

Before starting migration:

1. **Stop all Aspen nodes** using the redb database
2. **Backup existing redb files** to a safe location
3. **Ensure sufficient disk space** (SQLite database will be similar size to redb)
4. **Verify node is healthy** before shutdown

## Migration Steps

### 1. Stop the Aspen Node

Gracefully shut down the node to ensure all pending writes are flushed:

```bash
# If using HTTP admin API
curl -X POST http://localhost:8080/admin/shutdown

# Or send SIGTERM
pkill -TERM aspen-node

# Wait for clean shutdown (check logs)
tail -f /var/log/aspen/node.log
```

Verify the process has stopped:

```bash
ps aux | grep aspen-node
```

### 2. Backup the redb Database

Create a backup copy of the redb state machine database:

```bash
# Example paths (adjust to your deployment)
cp data/node-1/state-machine.redb data/node-1/state-machine.redb.backup-$(date +%Y%m%d-%H%M%S)

# Verify backup
ls -lh data/node-1/state-machine.redb*
```

Store backups in a separate location for disaster recovery:

```bash
# Copy to backup storage
rsync -av data/node-1/state-machine.redb.backup-* /backup/aspen/
```

### 3. Run the Migration Tool

Execute the migration with verification:

```bash
aspen-migrate \
  --source data/node-1/state-machine.redb \
  --target data/node-1/state-machine.db \
  --verify
```

#### Migration Options

- `--source <PATH>`: Path to source redb database (required)
- `--target <PATH>`: Path to target SQLite database (required)
- `--verify`: Enable verification (recommended)
- `--full-verify`: Perform full checksum verification (expensive, checks all keys)

#### Expected Output

```
Aspen State Machine Migration Tool
===================================
Source (redb):  data/node-1/state-machine.redb
Target (SQLite): data/node-1/state-machine.db

[1/6] Opening source redb database...
[2/6] Reading all key-value pairs from redb...
      Found 1234 key-value pairs
[3/6] Reading metadata from redb...
      Last applied log: term=5, index=1234
[4/6] Creating target SQLite database...
[5/6] Migrating data to SQLite...
      Migration complete!
[6/6] Verifying migration integrity...
      Count verification: 1234 keys
      Data verification: 100 of 1234 keys sampled
      Metadata verification: passed
      Verification passed!

Migration Summary
=================
  Keys migrated:  1234
  Source:         data/node-1/state-machine.redb
  Target:         data/node-1/state-machine.db
  Verified:       true

Next steps:
  1. Backup the original redb file
  2. Update configuration to use SQLite storage
  3. Restart the Aspen node
```

### 4. Update Configuration

Modify the node configuration to use SQLite storage:

```toml
# config.toml
[cluster]
# Change from 'redb' to 'sqlite'
storage_backend = "sqlite"

# Optional: Configure SQLite-specific settings
[cluster.sqlite]
pool_size = 10  # Number of read connections
```

Or via environment variable:

```bash
export ASPEN_STORAGE_BACKEND=sqlite
```

### 5. Restart the Node

Start the node with the new SQLite backend:

```bash
# Using systemd
systemctl start aspen-node

# Or directly
aspen-node --config config.toml
```

Monitor startup logs for successful initialization:

```bash
tail -f /var/log/aspen/node.log

# Look for:
# INFO aspen::raft: Storage backend initialized: sqlite
# INFO aspen::raft: State machine loaded: last_applied=1234
```

### 6. Verify Node Health

Check that the node is healthy and can serve requests:

```bash
# Health check
curl http://localhost:8080/health

# Expected response:
# {"status":"healthy","storage":"sqlite","last_applied":1234}

# Test a read operation
curl http://localhost:8080/api/kv/test_key

# Test a write operation
curl -X POST http://localhost:8080/api/kv/test_key \
  -H "Content-Type: application/json" \
  -d '{"value":"test_value"}'
```

### 7. Monitor Performance

SQLite with WAL mode should provide better read concurrency:

```bash
# Check WAL file size
ls -lh data/node-1/state-machine.db-wal

# Monitor checkpoint activity in logs
grep "WAL checkpoint" /var/log/aspen/node.log
```

## Rollback Procedure

If migration fails or issues arise, rollback to redb:

### 1. Stop the Node

```bash
systemctl stop aspen-node
# Or
pkill -TERM aspen-node
```

### 2. Restore from Backup

```bash
# Remove SQLite database
rm -f data/node-1/state-machine.db
rm -f data/node-1/state-machine.db-wal
rm -f data/node-1/state-machine.db-shm

# Restore redb backup
cp data/node-1/state-machine.redb.backup-TIMESTAMP data/node-1/state-machine.redb
```

### 3. Update Configuration

```toml
# config.toml
[cluster]
storage_backend = "redb"
```

### 4. Restart Node

```bash
systemctl start aspen-node
```

### 5. Verify Recovery

```bash
curl http://localhost:8080/health
# Should show storage="redb"
```

## Verification Details

The migration tool performs several verification steps when `--verify` is enabled:

### 1. Count Verification

Ensures the number of key-value pairs matches between source and target:

```
source_count == target_count
```

### 2. Data Verification

Samples up to 100 keys (or all keys with `--full-verify`) and verifies values match:

```
for key in sample(keys, 100):
    assert source.get(key) == target.get(key)
```

### 3. Metadata Verification

Verifies Raft metadata consistency:

- `last_applied_log`: Ensures the last applied log index matches
- `last_membership`: Ensures the cluster membership config matches

### 4. Checksum Verification (optional)

With `--full-verify`, computes deterministic checksums of all data:

```
source_checksum == target_checksum
```

This is expensive for large databases but provides maximum confidence.

## Troubleshooting

### Migration Tool Errors

#### Error: "Source database does not exist"

**Cause**: Invalid path to redb database

**Solution**:

```bash
# Verify path
ls -l data/node-1/state-machine.redb

# Use absolute path if needed
aspen-migrate --source /absolute/path/to/state-machine.redb ...
```

#### Error: "Target database already exists"

**Cause**: SQLite database file already exists (prevents accidental overwrites)

**Solution**:

```bash
# Remove existing target or choose different path
rm data/node-1/state-machine.db

# Or use different target
aspen-migrate --target data/node-1/state-machine-new.db ...
```

#### Error: "Key count mismatch"

**Cause**: Migration failed to copy all keys

**Solution**:

1. Check disk space: `df -h`
2. Check redb database integrity
3. Review migration tool logs for errors
4. Retry migration after fixing underlying issue

#### Error: "Value mismatch for key"

**Cause**: Data corruption during migration

**Solution**:

1. Remove target database
2. Verify source database integrity
3. Retry migration
4. If persistent, report as a bug

### Node Startup Errors

#### Error: "Failed to open SQLite database"

**Cause**: Permissions issue or corrupted database

**Solution**:

```bash
# Check file permissions
ls -l data/node-1/state-machine.db

# Ensure node user has read/write access
chown aspen:aspen data/node-1/state-machine.db
chmod 600 data/node-1/state-machine.db
```

#### Error: "State machine corruption detected"

**Cause**: Inconsistent state between log and state machine

**Solution**:

1. Stop the node
2. Rollback to redb backup
3. Verify redb database is healthy
4. Retry migration with `--full-verify`

### Performance Issues

#### Slow Reads After Migration

**Cause**: SQLite needs to build page cache

**Solution**:

- Allow warm-up period (5-10 minutes)
- SQLite WAL mode optimizes over time
- Monitor with `PRAGMA wal_checkpoint` status

#### WAL File Growing Large

**Cause**: Checkpoint not running frequently enough

**Solution**:

```bash
# Manual checkpoint
sqlite3 data/node-1/state-machine.db "PRAGMA wal_checkpoint(TRUNCATE);"

# Or trigger via Aspen API (if available)
curl -X POST http://localhost:8080/admin/checkpoint
```

## Multi-Node Cluster Migration

For a cluster with multiple nodes:

### Strategy 1: Rolling Migration (Zero Downtime)

Migrate nodes one at a time while maintaining quorum:

```bash
# For a 3-node cluster:

# 1. Migrate node-3 (non-leader)
#    - Stop node-3
#    - Migrate node-3
#    - Start node-3 with SQLite
#    - Verify health

# 2. Migrate node-2 (non-leader)
#    - Stop node-2
#    - Migrate node-2
#    - Start node-2 with SQLite
#    - Verify health

# 3. Migrate node-1 (leader)
#    - Trigger leader election
#    - Stop node-1
#    - Migrate node-1
#    - Start node-1 with SQLite
#    - Verify health

# All nodes now running SQLite
```

**Requirements**:

- Cluster must have at least 3 nodes
- Quorum must be maintained (n/2 + 1 nodes up)
- Leader may change during migration

### Strategy 2: Full Shutdown Migration (Simpler)

Shut down entire cluster and migrate all nodes:

```bash
# 1. Stop all nodes
for node in node-1 node-2 node-3; do
    ssh $node systemctl stop aspen-node
done

# 2. Migrate all nodes in parallel
for node in node-1 node-2 node-3; do
    ssh $node aspen-migrate \
      --source data/state-machine.redb \
      --target data/state-machine.db \
      --verify &
done
wait

# 3. Update configurations
for node in node-1 node-2 node-3; do
    ssh $node "sed -i 's/storage_backend = \"redb\"/storage_backend = \"sqlite\"/' /etc/aspen/config.toml"
done

# 4. Start all nodes
for node in node-1 node-2 node-3; do
    ssh $node systemctl start aspen-node
done

# 5. Verify cluster health
curl http://node-1:8080/health
```

**Downtime**: Proportional to database size (typically 1-5 minutes)

## FAQ

### Q: Can I migrate back from SQLite to redb?

A: Not directly with the migration tool. You would need to:

1. Export data from SQLite
2. Create new redb database
3. Import data into redb
4. Verify consistency

However, this is not recommended. SQLite is the recommended storage backend.

### Q: How long does migration take?

A: Migration time depends on database size:

- Small (< 1 GB): 1-10 seconds
- Medium (1-10 GB): 10-60 seconds
- Large (10-100 GB): 1-5 minutes

Verification adds ~20% overhead.

### Q: Can I run migration on a live node?

A: **No**. Always stop the node before migration to prevent:

- Data inconsistency
- File locking conflicts
- Partial writes during migration

### Q: What if migration fails halfway?

A: The migration tool uses a transaction to ensure atomicity:

- If migration fails, target database is not created
- Source database remains untouched
- Safe to retry after fixing the issue

### Q: Does migration affect the Raft log?

A: No. Migration only affects the state machine (key-value data and metadata). The Raft log remains in redb format. Future versions may support full migration.

### Q: Can I delete the redb backup after migration?

A: Keep backups for at least 7 days or until you're confident the migration was successful and the node is stable.

## Best Practices

1. **Always backup** before migration
2. **Test in staging** environment first
3. **Migrate during maintenance window** to minimize risk
4. **Monitor closely** after migration for 24-48 hours
5. **Keep redb backups** for at least one week
6. **Use `--verify`** flag for production migrations
7. **Use `--full-verify`** for critical data
8. **Document** the migration process in runbooks
9. **Plan rollback strategy** before starting

## Support

For issues or questions:

- GitHub Issues: <https://github.com/your-org/aspen/issues>
- Documentation: <https://docs.aspen.io>
- Community Chat: <https://discord.gg/aspen>
