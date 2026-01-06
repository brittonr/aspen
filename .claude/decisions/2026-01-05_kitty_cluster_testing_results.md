# Kitty Cluster Testing Results

**Date**: 2026-01-05
**Test Environment**: Linux 6.17.12, Rust 2024 Edition

## Summary

Successfully tested the Aspen cluster formation using `scripts/cluster.sh`. The cluster is **fully functional** and all core distributed operations work correctly.

## Test Results

### Cluster Formation

| Test | Status | Notes |
|------|--------|-------|
| Node startup (3 nodes) | PASS | All nodes start and generate endpoint IDs |
| Ticket generation | PASS | Cluster ticket generated and extracted from logs |
| Gossip discovery | PASS | Nodes share same gossip topic (derived from cookie) |
| Cluster init | PASS | Node 1 becomes single-node leader |
| Add learners | PASS | Nodes 2-3 added as non-voting learners |
| Promote to voters | PASS | All nodes promoted to voting members |
| Leader election | PASS | Node 1 elected leader |

### Raft Consensus

| Test | Status | Notes |
|------|--------|-------|
| Single leader | PASS | Only node 1 shows as leader |
| All voters | PASS | All 3 nodes show Voter=Y after membership change |
| Cluster status query | PASS | All nodes visible in cluster status |

### Key-Value Operations

| Test | Status | Notes |
|------|--------|-------|
| Write (kv set) | PASS | Writes succeed and return "OK" |
| Read (kv get) | PASS | Reads return correct values |
| Delete (kv delete) | PASS | Deletes succeed |
| Scan (kv scan) | PASS | Prefix scan works correctly |
| System keys | PASS | Rate limiting, job queues, metrics present |

## Issues Discovered

### 1. Cluster Init Timing Issue (Script)

**Issue**: The `cluster.sh` script's init attempt often fails during automated cluster formation due to timing:

```
Initializing cluster on node 1....... failed
```

**Root Cause**: The script waits only 5 seconds for gossip discovery before attempting cluster init. The CLI connects and sends init before the Iroh endpoint is fully ready.

**Workaround**: Manual init succeeds immediately after the script completes:

```bash
aspen-cli --ticket $TICKET cluster init
```

**Recommendation**: Increase the gossip discovery wait time or add retry logic with exponential backoff in `scripts/cluster.sh`.

### 2. IPv6 Network Unreachable Warnings

**Issue**: Consistent warnings about IPv6 network unreachability:

```
sendmsg error: Network is unreachable, destination: [2a01:4ff:f0:febe::1]:7842
```

**Impact**: Low - fallback to IPv4/relay works correctly.

**Recommendation**: Consider suppressing these warnings when IPv6 is not configured, or add a configuration option to disable IPv6 in Iroh.

### 3. Kernel NLA Warning Spam

**Issue**: Repeated warnings about netlink attributes:

```
Specified IFLA_INET6_CONF NLA attribute holds more data which is unknown to netlink-packet-route crate
```

**Impact**: Low - cosmetic, does not affect functionality.

**Cause**: The netlink-packet-route crate is older than the current kernel (6.17).

**Recommendation**: Update `netlink-packet-route` dependency when a compatible version is available.

## Cluster Architecture Summary

### Node Discovery Flow

1. Each node starts with same cluster cookie
2. Gossip topic derived deterministically from cookie via blake3 hash
3. Nodes broadcast their endpoint IDs on the shared topic
4. mDNS provides additional local discovery

### Cluster Formation Steps

1. `cluster init` - First node becomes single-node cluster leader
2. `cluster add-learner` - Additional nodes join as non-voting replicas
3. `cluster change-membership` - Promote learners to voting members

### Key Configuration

- Default: 4 nodes (kitty-cluster.sh), 3 nodes (cluster.sh)
- Storage: inmemory (cluster.sh), redb (kitty-cluster.sh)
- Gossip discovery: Always enabled
- mDNS: Enabled by default

## Performance Observations

- Node startup: < 1 second
- Cluster init: < 100ms
- Add learner: < 100ms per node
- Membership change: < 100ms
- KV write: < 50ms
- KV read: < 10ms

## Recommendations

1. **Fix script timing**: Increase `wait_for_ticket` timeout and add retries to `init_cluster` in both `cluster.sh` and `kitty-cluster.sh`.

2. **Reduce log noise**: Add log filters to suppress known-benign warnings (IPv6, NLA attributes) unless at debug level.

3. **Add health checks**: The scripts should verify nodes are healthy before proceeding with cluster formation.

4. **Document the manual workflow**: If automated cluster formation fails, provide clear instructions for manual recovery.

## Test Commands Reference

```bash
# Start cluster (background mode)
ASPEN_NODE_COUNT=3 ASPEN_FOREGROUND=false ./scripts/cluster.sh start

# Get ticket
cat /tmp/aspen-cluster-*/ticket.txt

# Initialize cluster manually
aspen-cli --ticket $TICKET cluster init

# Add learners
aspen-cli --ticket $TICKET cluster add-learner --node-id 2 --addr $ENDPOINT_ID2
aspen-cli --ticket $TICKET cluster add-learner --node-id 3 --addr $ENDPOINT_ID3

# Promote to voters
aspen-cli --ticket $TICKET cluster change-membership 1 2 3

# Check cluster status
aspen-cli --ticket $TICKET cluster status

# Test KV operations
aspen-cli --ticket $TICKET kv set key value
aspen-cli --ticket $TICKET kv get key
aspen-cli --ticket $TICKET kv scan ""
aspen-cli --ticket $TICKET kv delete key

# Stop cluster
./scripts/cluster.sh stop
```
