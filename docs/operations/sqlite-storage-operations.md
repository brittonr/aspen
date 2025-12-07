# SQLite Storage Operations Guide

This guide covers monitoring, maintenance, and troubleshooting for Aspen's SQLite-backed state machine storage.

## Table of Contents

- [Monitoring](#monitoring)
- [Maintenance](#maintenance)
- [Troubleshooting](#troubleshooting)
- [Performance Tuning](#performance-tuning)
- [Configuration Reference](#configuration-reference)
- [Best Practices](#best-practices)

## Monitoring

### Health Checks

The `/health` endpoint provides detailed storage health information:

```bash
curl http://localhost:8080/health
```

**Healthy Response**:

```json
{
  "status": "healthy",
  "checks": {
    "raft_actor": {"status": "ok"},
    "storage": {"status": "ok"},
    "disk_space": {"status": "ok"},
    "wal_file": {"status": "ok"}
  },
  "node_id": 1,
  "raft_node_id": 1
}
```

**Warning Response** (WAL file growing):

```json
{
  "status": "degraded",
  "checks": {
    "raft_actor": {"status": "ok"},
    "storage": {"status": "ok"},
    "disk_space": {"status": "ok"},
    "wal_file": {
      "status": "warning",
      "message": "WAL file size: 150MB (exceeds 100MB threshold)"
    }
  },
  "node_id": 1,
  "raft_node_id": 1
}
```

**Critical Response** (WAL file very large):

```json
{
  "status": "unhealthy",
  "checks": {
    "raft_actor": {"status": "ok"},
    "storage": {"status": "ok"},
    "disk_space": {"status": "ok"},
    "wal_file": {
      "status": "critical",
      "message": "WAL file size: 550MB (exceeds 500MB critical threshold)"
    }
  },
  "node_id": 1,
  "raft_node_id": 1
}
```

### WAL Status Levels

- **ok**: WAL file < 100MB (normal operation)
- **warning**: WAL file 100MB - 500MB (checkpoint recommended)
- **critical**: WAL file > 500MB (checkpoint required immediately)

### Metrics

Prometheus-compatible metrics are exposed on `/metrics`:

```bash
curl http://localhost:8080/metrics | grep sqlite
```

**Available Metrics** (future):

```
# HELP sqlite_wal_size_bytes Current WAL file size in bytes
# TYPE sqlite_wal_size_bytes gauge
sqlite_wal_size_bytes{node_id="1"} 104857600

# HELP sqlite_write_latency_seconds Write operation latency
# TYPE sqlite_write_latency_seconds histogram
sqlite_write_latency_seconds_bucket{node_id="1",le="0.001"} 950
sqlite_write_latency_seconds_bucket{node_id="1",le="0.010"} 990
sqlite_write_latency_seconds_bucket{node_id="1",le="0.100"} 1000

# HELP sqlite_read_latency_seconds Read operation latency
# TYPE sqlite_read_latency_seconds histogram
sqlite_read_latency_seconds_bucket{node_id="1",le="0.001"} 9800
sqlite_read_latency_seconds_bucket{node_id="1",le="0.010"} 10000

# HELP sqlite_checkpoint_duration_seconds WAL checkpoint duration
# TYPE sqlite_checkpoint_duration_seconds histogram
sqlite_checkpoint_duration_seconds_bucket{node_id="1",le="0.100"} 5
sqlite_checkpoint_duration_seconds_bucket{node_id="1",le="1.000"} 10
```

### Log Monitoring

Enable structured logging to track SQLite operations:

```bash
RUST_LOG=aspen::raft::storage_sqlite=debug ./aspen-node --config config.toml
```

**Key log events**:

```
DEBUG aspen::raft::storage_sqlite: WAL checkpoint completed pages=1024 duration_ms=150
DEBUG aspen::raft::storage_sqlite: Snapshot built entries=1000 size_kb=256
WARN  aspen::raft::storage_sqlite: WAL file exceeds threshold size_mb=120 threshold_mb=100
ERROR aspen::raft::storage_sqlite: Cross-storage validation failed last_applied=105 committed=100
```

## Maintenance

### Manual WAL Checkpoint

Checkpoint the WAL file to reclaim disk space:

```bash
curl -X POST http://localhost:8080/admin/checkpoint-wal
```

**Success Response**:

```json
{
  "status": "success",
  "pages_checkpointed": 1024,
  "wal_size_before_bytes": 104857600,
  "wal_size_after_bytes": 0
}
```

**When to Checkpoint**:

- WAL file exceeds 100MB (warning threshold)
- Before backup operations
- After large batch writes (e.g., snapshot installation)
- During low-traffic periods (e.g., maintenance windows)
- Before planned node shutdown

### Database Integrity Check

Run SQLite's built-in integrity check:

```bash
sqlite3 /var/lib/aspen/node-1/state-machine.db "PRAGMA integrity_check;"
```

**Expected Output**: `ok`

**On Corruption**:

```
*** in database main ***
Page 42: btreeInitPage() returns error code 11
```

If corrupted, restore from backup or re-sync from Raft cluster.

### Backup Procedures

#### Hot Backup (Recommended)

Use SQLite's online backup API (future tool):

```bash
# TODO: Implement aspen-backup tool using SQLite backup API
# This allows backup while node is running
aspen-backup --source /var/lib/aspen/node-1/state-machine.db \
             --dest /backups/state-machine-$(date +%Y%m%d).db
```

#### Cold Backup (Simple but requires downtime)

Stop the node, copy files, restart:

```bash
# 1. Stop node gracefully
curl -X POST http://localhost:8080/admin/shutdown

# 2. Wait for shutdown
sleep 5

# 3. Backup database and WAL file
cp /var/lib/aspen/node-1/state-machine.db \
   /backups/state-machine-$(date +%Y%m%d).db

cp /var/lib/aspen/node-1/state-machine.db-wal \
   /backups/state-machine-$(date +%Y%m%d).db-wal

# 4. Restart node
systemctl start aspen-node
```

#### Continuous Backup Strategy

```bash
# Daily backup via cron (3 AM)
0 3 * * * /usr/local/bin/aspen-backup-daily.sh

# Backup script (aspen-backup-daily.sh)
#!/bin/bash
set -euo pipefail

BACKUP_DIR="/backups/aspen/$(date +%Y%m%d)"
mkdir -p "$BACKUP_DIR"

# Checkpoint WAL first (reduces backup size)
curl -X POST http://localhost:8080/admin/checkpoint-wal

# Copy database (SQLite handles concurrent reads in WAL mode)
cp /var/lib/aspen/node-1/state-machine.db "$BACKUP_DIR/"

# Verify backup
sqlite3 "$BACKUP_DIR/state-machine.db" "PRAGMA integrity_check;" | grep -q "ok"

# Retain 7 days of backups
find /backups/aspen -type d -mtime +7 -exec rm -rf {} \;
```

### Database Vacuum

Reclaim unused space (requires downtime):

```bash
# Stop node
curl -X POST http://localhost:8080/admin/shutdown

# Vacuum database (rebuilds file to reclaim space)
sqlite3 /var/lib/aspen/node-1/state-machine.db "VACUUM;"

# Check new size
ls -lh /var/lib/aspen/node-1/state-machine.db

# Restart node
systemctl start aspen-node
```

**When to Vacuum**:

- Database file significantly larger than actual data
- After deleting large amounts of data
- During scheduled maintenance windows
- Not frequently (vacuum is I/O intensive)

## Troubleshooting

### Issue: WAL File Growing Unbounded

**Symptoms**:

- `/health` endpoint shows `critical` WAL status
- Disk space decreasing rapidly
- WAL file size > 500MB

**Root Causes**:

1. High write load without checkpoints
2. Long-running read transactions blocking checkpoints
3. Auto-checkpoint disabled or threshold too high

**Resolution**:

1. **Immediate**: Manual checkpoint

   ```bash
   curl -X POST http://localhost:8080/admin/checkpoint-wal
   ```

2. **Check disk space**:

   ```bash
   df -h /var/lib/aspen
   ```

3. **If disk full**: Free space then checkpoint

   ```bash
   # Find and remove old logs/backups
   find /var/log -name "*.gz" -mtime +30 -delete

   # Checkpoint WAL
   curl -X POST http://localhost:8080/admin/checkpoint-wal
   ```

4. **If checkpoint fails**: Check write load

   ```bash
   # Monitor write rate
   curl http://localhost:8080/metrics | grep sqlite_write

   # Check active connections
   lsof -p $(pidof aspen-node) | grep state-machine.db
   ```

5. **Long-term fix**: Adjust auto-checkpoint threshold

   ```toml
   [cluster]
   storage_backend = "sqlite"
   # Lower threshold for more frequent checkpoints
   sqlite_auto_checkpoint_threshold_mb = 50
   ```

### Issue: Database Corruption Detected

**Symptoms**:

- Node fails to start
- `/health` endpoint shows `storage: failed`
- Logs show "integrity_check failed"

**Resolution**:

1. **Verify corruption**:

   ```bash
   sqlite3 /var/lib/aspen/node-1/state-machine.db "PRAGMA integrity_check;"
   ```

2. **If corrupted**: Restore from backup

   ```bash
   # Stop node
   systemctl stop aspen-node

   # Move corrupted database
   mv /var/lib/aspen/node-1/state-machine.db \
      /var/lib/aspen/node-1/state-machine.db.corrupted

   # Restore latest backup
   cp /backups/state-machine-latest.db \
      /var/lib/aspen/node-1/state-machine.db

   # Start node
   systemctl start aspen-node
   ```

3. **If no backup**: Re-sync from Raft cluster

   ```bash
   # Remove corrupted database
   rm /var/lib/aspen/node-1/state-machine.db

   # Start node (will re-sync from leader via snapshot)
   systemctl start aspen-node

   # Monitor re-sync progress
   tail -f /var/log/aspen/node-1.log | grep snapshot
   ```

4. **Report issue**: Corruption should never happen in normal operation
   - Save corrupted database for analysis
   - File bug report with reproduction steps
   - Include logs, metrics, and hardware details

### Issue: Cross-Storage Validation Failed

**Symptoms**:

- Supervisor prevents node restart
- Logs show "State machine corruption detected: last_applied (N) exceeds committed (M)"

**Explanation**:
This indicates a serious consistency violation. The state machine claims to have applied log entries that the Raft log hasn't committed yet. This violates Raft invariants.

**Resolution**:

**DO NOT bypass this check.** This error indicates potential data corruption.

1. **Investigate root cause**:

   ```bash
   # Check for disk corruption
   dmesg | grep -i error

   # Check file system
   fsck /dev/sda1  # (requires unmount)

   # Review recent changes
   journalctl -u aspen-node --since "1 hour ago"
   ```

2. **Check both databases**:

   ```bash
   # State machine last_applied
   sqlite3 /var/lib/aspen/node-1/state-machine.db \
     "SELECT value FROM state_machine_meta WHERE key='last_applied_log';"

   # Log committed index
   # (requires custom tool to read redb)
   aspen-debug-storage --log /var/lib/aspen/node-1/raft-log.redb \
     --show-committed
   ```

3. **Restore from known-good backup**:

   ```bash
   systemctl stop aspen-node

   # Restore both log and state machine from same backup
   cp /backups/raft-log-latest.redb /var/lib/aspen/node-1/
   cp /backups/state-machine-latest.db /var/lib/aspen/node-1/

   systemctl start aspen-node
   ```

4. **Report to development team**:
   - This should never happen in normal operation
   - Indicates a bug in storage layer or hardware issue
   - Preserve corrupted databases for analysis

### Issue: Slow Write Performance

**Symptoms**:

- High write latency (> 100ms per write)
- Raft leader falling behind
- Clients timing out on writes

**Root Causes**:

1. Disk I/O saturation
2. Large WAL file (not checkpointed)
3. Slow fsync (USB drives, network storage)

**Resolution**:

1. **Check disk I/O**:

   ```bash
   iostat -x 5
   # Look for high %util on storage device
   ```

2. **Checkpoint WAL**:

   ```bash
   curl -X POST http://localhost:8080/admin/checkpoint-wal
   ```

3. **Check storage type**:

   ```bash
   # Is it on SSD or HDD?
   lsblk -d -o name,rota
   # rota=1 means HDD (slow), rota=0 means SSD (fast)
   ```

4. **Ensure local storage**:
   - Never use NFS/CIFS for SQLite databases
   - Prefer local SSD over network storage
   - Avoid USB drives (unreliable fsync)

5. **Monitor fsync latency**:

   ```bash
   strace -c -p $(pidof aspen-node)
   # Look for slow fsync/fdatasync calls
   ```

### Issue: "Database is Locked" Errors

**Symptoms**:

- Logs show "database is locked"
- Writes failing intermittently

**Root Causes**:

1. Write connection deadlock
2. External process holding lock (sqlite3 CLI)
3. File system locking issues (NFS)

**Resolution**:

1. **Check for external locks**:

   ```bash
   lsof | grep state-machine.db
   # Kill any sqlite3 CLI sessions
   ```

2. **Verify WAL mode**:

   ```bash
   sqlite3 /var/lib/aspen/node-1/state-machine.db \
     "PRAGMA journal_mode;"
   # Should return "wal"
   ```

3. **Restart node**:

   ```bash
   systemctl restart aspen-node
   ```

4. **Check file system**:
   - Ensure local file system (not NFS)
   - Check for file system errors: `dmesg | grep -i error`

## Performance Tuning

### Read-Heavy Workloads

If you have high read concurrency (many simultaneous readers):

1. **Increase connection pool size**:

   ```rust
   // In code (requires recompile)
   let sm = SqliteStateMachine::with_pool_size(&path, 20)?;
   ```

2. **Monitor pool contention**:

   ```bash
   # Future: metrics will show pool utilization
   curl http://localhost:8080/metrics | grep sqlite_pool
   ```

3. **Expected performance**:
   - Pool size 1: ~38,000 reads/sec (10 concurrent readers)
   - Pool size 10: ~98,000 reads/sec (2.6x improvement)
   - Pool size 20: ~105,000 reads/sec (diminishing returns)

### Write-Heavy Workloads

SQLite single-writer is an inherent limitation. For write-heavy workloads:

1. **Monitor write latency**:

   ```bash
   curl http://localhost:8080/metrics | grep sqlite_write_latency
   ```

2. **Ensure SSD storage**:
   - SQLite benefits significantly from SSD (10-100x faster fsync)
   - HDDs: ~10ms fsync latency
   - SSDs: ~0.1ms fsync latency

3. **Checkpoint during low traffic**:

   ```bash
   # Schedule checkpoints during low-traffic periods
   0 2 * * * curl -X POST http://localhost:8080/admin/checkpoint-wal
   ```

4. **Batch operations**:
   - Use `SetMulti` instead of multiple `Set` operations
   - Reduces transaction overhead

### Large State Machines

For state machines with millions of keys:

1. **Monitor snapshot build time**:

   ```bash
   tail -f /var/log/aspen/node-1.log | grep "snapshot built"
   ```

2. **Optimize snapshot builds**:
   - Snapshots use read pool (non-blocking for writes)
   - Large snapshots (> 1GB) may take minutes
   - Schedule manual snapshots during maintenance windows

3. **Tune checkpoint frequency**:

   ```toml
   [cluster]
   # For large databases, checkpoint more frequently
   sqlite_auto_checkpoint_threshold_mb = 50
   ```

## Configuration Reference

### TOML Configuration

```toml
[cluster]
# Storage backend (inmemory, redb, sqlite)
storage_backend = "sqlite"

# Data directory (default: ./data/node-{node_id})
data_dir = "/var/lib/aspen/node-1"

# SQLite state machine path (default: {data_dir}/state-machine.db)
sqlite_sm_path = "/var/lib/aspen/node-1/state-machine.db"

# Redb log path (default: {data_dir}/raft-log.redb)
redb_log_path = "/var/lib/aspen/node-1/raft-log.redb"
```

### Environment Variables

```bash
# Override storage backend
export ASPEN_STORAGE_BACKEND=sqlite

# Override state machine path
export ASPEN_SQLITE_SM_PATH=/custom/path/state-machine.db

# Override log path
export ASPEN_REDB_LOG_PATH=/custom/path/raft-log.redb
```

### SQLite Pragmas (Internal Configuration)

These are set automatically by `SqliteStateMachine::new()`:

```sql
-- Write-Ahead Logging (concurrent reads)
PRAGMA journal_mode = WAL;

-- Full synchronous (durability over speed)
PRAGMA synchronous = FULL;
```

**Do not override these settings.** They are critical for correctness and durability.

## Best Practices

### 1. Always Use FULL Synchronous Mode

**Do**: Use default FULL synchronous mode (already configured)

```rust
conn.pragma_update(None, "synchronous", "FULL")?;
```

**Don't**: Never disable for performance gains

```rust
// NEVER DO THIS - risk of data loss
conn.pragma_update(None, "synchronous", "OFF")?;
```

**Rationale**: Tiger Style principle - fail-safe over fast. FULL mode ensures data reaches disk before commit returns, preventing data loss on OS crash or power failure.

### 2. Monitor WAL Size

**Do**: Set up alerts for WAL file size

```bash
# Prometheus alert rule
- alert: SqliteWALFileLarge
  expr: sqlite_wal_size_bytes > 104857600  # 100MB
  for: 5m
  labels:
    severity: warning
  annotations:
    summary: "SQLite WAL file exceeds 100MB"
```

**Don't**: Ignore WAL growth

- WAL can grow unbounded without checkpoints
- Large WAL files slow down database operations
- Can cause disk space exhaustion

### 3. Regular Backups

**Do**: Implement automated daily backups

```bash
# Cron job
0 3 * * * /usr/local/bin/aspen-backup-daily.sh
```

**Don't**: Rely on Raft replication alone

- Raft provides fault tolerance, not backup
- Correlated failures can affect all replicas
- Backups enable point-in-time recovery

### 4. Test Restore Procedures

**Do**: Regularly test backup restoration

```bash
# Monthly restore test
./scripts/test-backup-restore.sh
```

**Don't**: Assume backups work without testing

- Untested backups are as good as no backups
- Restoration may reveal corruption or configuration issues

### 5. Cross-Storage Validation

**Do**: Never bypass validation errors

```bash
# If validation fails, investigate root cause
journalctl -u aspen-node | grep "validation failed"
```

**Don't**: Disable validation to "fix" startup issues

- Validation errors indicate real corruption
- Bypassing can lead to data loss or inconsistency

### 6. Checkpoint During Low Traffic

**Do**: Schedule checkpoints during maintenance windows

```bash
# 2 AM checkpoint (low traffic)
0 2 * * * curl -X POST http://localhost:8080/admin/checkpoint-wal
```

**Don't**: Checkpoint during peak traffic

- Checkpoints can briefly block writes
- May increase latency during busy periods

### 7. Use SSD Storage

**Do**: Deploy on SSD-backed storage

- 10-100x faster fsync than HDDs
- Lower write latency (< 1ms vs 10ms)

**Don't**: Use HDDs for production

- SQLite's FULL synchronous mode requires fast fsync
- HDDs will bottleneck write throughput

### 8. Local File Systems Only

**Do**: Use local ext4, xfs, or btrfs file systems

```bash
# Check file system type
df -T /var/lib/aspen
```

**Don't**: Use network file systems (NFS, CIFS)

- SQLite requires working file locks
- Network file systems have unreliable locking
- Risk of database corruption

### 9. Monitor Disk Space

**Do**: Set up disk space alerts

```bash
# Alert when disk > 80% full
- alert: DiskSpaceHigh
  expr: node_filesystem_avail_bytes{mountpoint="/var/lib/aspen"} / node_filesystem_size_bytes{mountpoint="/var/lib/aspen"} < 0.2
```

**Don't**: Let disk fill up

- SQLite needs space for WAL file growth
- Out-of-space errors can corrupt database

### 10. Keep SQLite Updated

**Do**: Use recent SQLite versions (3.38+)

```bash
# Check SQLite version
sqlite3 --version
```

**Don't**: Use ancient SQLite versions

- Older versions have bugs and performance issues
- WAL mode has improved significantly in recent versions

## Summary

SQLite storage in Aspen provides:

- **Durability**: ACID transactions with FULL synchronous mode
- **Debuggability**: Standard SQL tools (sqlite3, DB Browser)
- **Performance**: 2.6x read throughput with connection pooling
- **Operability**: Health checks, metrics, and checkpoint management

Key operational tasks:

1. Monitor WAL file size via `/health` endpoint
2. Checkpoint when WAL exceeds 100MB
3. Daily backups with restoration testing
4. Integrity checks during maintenance windows
5. Disk space monitoring and alerts

For questions or issues, refer to:

- [ADR-011: Hybrid SQLite Storage](../adr/011-hybrid-sqlite-storage.md)
- [GitHub Issues](https://github.com/your-org/aspen/issues)
- Production oncall: <oncall@example.com>
