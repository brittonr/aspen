# Aspen Cluster Persistence and Lifecycle Analysis

**Date**: 2026-01-06
**Analysis Type**: ULTRA Mode Investigation

## Executive Summary

Aspen clusters **have full persistence capabilities** and can survive node restarts, add new nodes to running clusters, and stop/start entire clusters. However, there are specific operational considerations.

## Key Findings

### 1. State Persistence: FULLY SUPPORTED

**Storage Architecture**: SharedRedbStorage (single-fsync, ~2-3ms per write)

| Component | Storage Location | Persistence |
|-----------|------------------|-------------|
| Raft Log | `RAFT_LOG_TABLE` in redb | Durable |
| State Machine (KV) | `SM_KV_TABLE` in redb | Durable |
| Raft Metadata | `RAFT_META_TABLE` (vote, committed) | Durable |
| Memberships | `SM_META_TABLE` (last_membership) | Durable |
| Chain Hashes | `CHAIN_HASH_TABLE` (integrity) | Durable |
| Snapshots | `SNAPSHOT_TABLE` | Durable |
| Node Registry | `MetadataStore` (separate redb) | Durable |

**Key Files**:

- `crates/aspen-raft/src/storage_shared.rs` - Unified log + state machine
- `crates/aspen-cluster/src/metadata.rs` - Node registry

### 2. Node Restart: FULLY SUPPORTED (with configuration)

**Identity Preservation Requirements**:

```toml
# config.toml - REQUIRED for stable identity
node_id = 1
data_dir = "./data/node-1"

[iroh]
# CRITICAL: Without this, node gets NEW endpoint_id on restart!
secret_key = "deadbeef...64hex..."
```

**Recovery Flow**:

1. Load config (same `node_id` as before)
2. Open existing redb database at `data_dir/raft.redb`
3. Restore Iroh endpoint with same `secret_key` (same network identity)
4. Raft reads persisted state: vote, log, committed index
5. State machine reads: last_applied_log, membership
6. Node rejoins cluster with full state

**Test Coverage**: `tests/madsim_redb_crash_recovery_test.rs` - 4 crash scenarios validated

### 3. Adding Nodes to Running Cluster: FULLY SUPPORTED

**API Methods** (ClusterController trait):

- `add_learner(request)` - Add non-voting node for replication
- `change_membership(request)` - Promote learners to voters

**Flow**:

```
1. New node starts with unique node_id
2. Configure with cluster cookie or ticket (for gossip discovery)
3. Leader calls add_learner() - new node begins replicating
4. Wait for log catch-up (< 100 entries behind)
5. Leader calls change_membership() - promotes to voter
6. Joint consensus: C-old -> C-old,new -> C-new
```

**Safety Mechanisms**:

- Membership cooldown: 300s between changes
- Learner health verification
- Log catchup verification (< 100 entries lag)
- Quorum preservation check
- Max voters limit: 100 nodes

**Test Coverage**: `tests/multi_node_cluster_test.rs`, `tests/chaos_membership_change.rs`

### 4. Stop and Start Cluster: SUPPORTED (with caveats)

**Full Cluster Restart**:

1. Stop all nodes gracefully (Raft shutdown)
2. Restart nodes with same `node_id` and `secret_key` configurations
3. Each node recovers from redb
4. First node to start initiates election
5. Cluster reforms with persisted state

**Quorum Requirement**:

- For 3-node cluster: Need 2+ nodes online
- For 5-node cluster: Need 3+ nodes online
- Minority cannot make progress (Raft safety)

**Caution**: If starting fewer than quorum, cluster is read-only until quorum restored.

## Architecture Diagram

```
Node Restart Flow:

+--------------+     +------------------+
| Config File  |---->| Load Config      |
| (node_id,    |     | (TOML -> struct) |
|  secret_key) |     +--------+---------+
+--------------+              |
                              v
+--------------+     +------------------+
| data_dir/    |---->| Open Existing    |
| raft.redb    |     | SharedRedbStorage|
+--------------+     +--------+---------+
                              |
                    +---------v---------+
                    | Read Persisted:   |
                    | - vote            |
                    | - log entries     |
                    | - last_applied    |
                    | - membership      |
                    +---------+---------+
                              |
                    +---------v---------+
                    | Restore Iroh      |
                    | (same secret_key) |
                    +---------+---------+
                              |
                    +---------v---------+
                    | Rejoin Cluster    |
                    | (gossip or peers) |
                    +-------------------+
```

## Current Gaps and Considerations

### Identity Persistence Gap

- **Issue**: If `secret_key` not persisted in config, new endpoint_id on restart
- **Impact**: Other nodes see "new" peer, not the restarted node
- **Solution**: Always persist `iroh.secret_key` in config

### Operational Best Practices

1. **Always persist** Iroh secret_key to config file
2. **Use stable data_dir** paths (absolute paths preferred)
3. **Start majority first** when restarting entire cluster
4. **Wait for log catchup** before promoting learners
5. **Monitor redb** file integrity (chain hashes auto-verify)

## Test Coverage Summary

| Scenario | Test File | Status |
|----------|-----------|--------|
| Crash during transaction | `madsim_redb_crash_recovery_test.rs` | PASS |
| Crash after commit | `madsim_redb_crash_recovery_test.rs` | PASS |
| Multiple crash cycles | `madsim_redb_crash_recovery_test.rs` | PASS |
| Leader restart | `router_t50_leader_restart.rs` | PASS |
| Add learners | `multi_node_cluster_test.rs` | PASS |
| Membership change | `chaos_membership_change.rs` | PASS |
| Snapshot recovery | `router_t50_snapshot_when_lacking_log.rs` | PASS |
| Corruption detection | `redb_corruption_scenarios_test.rs` | PASS |

## Recommendations

1. **Document identity persistence** in user documentation
2. **Consider auto-persisting** secret_key on first boot
3. **Add restart integration test** with full node lifecycle
4. **Add cluster restart test** with all nodes stopped/started

## References

- `crates/aspen-raft/src/storage_shared.rs:375-427` - Database open/create
- `crates/aspen-cluster/src/metadata.rs:112-139` - MetadataStore init
- `crates/aspen-raft/src/node.rs:332-393` - add_learner/change_membership
- `tests/madsim_redb_crash_recovery_test.rs` - Crash recovery validation
