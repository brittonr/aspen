# Raft Consensus Testing Guide

## Overview

Aspen supports two control backend modes for different testing purposes. This document explains when to use each and what they validate.

## Control Backend Comparison

### Deterministic Backend (`--control-backend deterministic`)

**Purpose**: Fast API shape validation

**Implementation**:
- In-memory HashMap-based storage
- No network communication
- Single-node operation
- Immediate responses

**What it tests**:
- HTTP endpoint availability
- Request/response format validation
- API contract correctness
- Basic key-value operations

**What it does NOT test**:
- Leader election
- Log replication
- Consensus conflicts
- Network failures
- Membership changes affecting actual Raft state

**Use cases**:
- CI fast checks (completes in ~5 seconds)
- API contract testing
- Development iteration
- Quick validation of HTTP layer

**Script**: `./scripts/aspen-cluster-smoke.sh`

### RaftActor Backend (`--control-backend raft_actor`)

**Purpose**: Real distributed consensus validation

**Implementation**:
- OpenRaft consensus engine
- redb persistent storage
- IRPC network communication
- Multi-node Raft cluster

**What it tests**:
- Leader election (1500-3000ms timeout)
- Log replication to followers
- Membership changes via Raft
- Consensus protocol correctness
- Multi-node coordination

**Slower execution**:
- Leader election: 1500-3000ms
- Log replication: 1-2s per write
- Membership changes: 2-3s
- Total test time: ~30-60 seconds

**Use cases**:
- Integration testing
- Production-like validation
- Consensus bug detection
- Pre-deployment verification

**Script**: `./scripts/aspen-cluster-raft-smoke.sh`

## Running Tests

### Quick API Check

```bash
# Build and test API shape (fast, ~5 seconds)
./scripts/aspen-cluster-smoke.sh
```

### Full Consensus Test

```bash
# Build and test real Raft consensus (slower, ~30-60 seconds)
./scripts/aspen-cluster-raft-smoke.sh
```

### Custom Configuration

Both scripts accept additional CLI flags:

```bash
# Disable gossip discovery
./scripts/aspen-cluster-raft-smoke.sh --disable-gossip

# Use custom cookie
./scripts/aspen-cluster-smoke.sh --cookie "my-custom-cookie"

# Enable DNS discovery
./scripts/aspen-cluster-raft-smoke.sh --enable-dns-discovery
```

## Validation Checklist

When testing with `raft_actor` backend, verify:

- [ ] **Leader Election**: Completes within 5 seconds after cluster init
- [ ] **Leader Consensus**: All nodes report same leader in `/metrics`
- [ ] **Log Replication**: Writes on leader appear on followers within 2 seconds
- [ ] **Membership Changes**: `add_learner()` and `change_membership()` update actual Raft state
- [ ] **Term Progression**: `current_term` increases across operations
- [ ] **Multi-Key Operations**: Multiple writes replicate independently
- [ ] **Membership Shrink**: Cluster continues operating after removing nodes

## Debugging Failed Tests

### Check Node Logs

```bash
# View all logs
cat n*.log

# Filter for election events
grep -i 'election\|leader\|vote' n*.log

# Check for errors
grep -i 'error\|panic\|fail' n*.log
```

### Inspect Metrics

```bash
# While cluster is running
curl http://127.0.0.1:21001/metrics

# Look for these lines:
# aspen_current_leader{node_id="1"} 1
# aspen_current_term{node_id="1"} 2
# aspen_state{node_id="1"} Leader
```

### Common Issues

#### Leader Not Elected

**Symptom**: `wait_for_leader()` times out after 20 seconds

**Possible causes**:
- Nodes not receiving each other's messages (check Iroh connectivity)
- Election timeout too short (increase to 3000ms)
- Nodes not discovering peers (check gossip/manual peer configuration)

**Fix**:
```bash
# Check if nodes can connect
grep -i "iroh\|peer" n*.log

# Verify Raft ports are open
netstat -an | grep 26001
```

#### Replication Failures

**Symptom**: `verify_replication()` fails - write succeeds but read returns empty

**Possible causes**:
- Reading from non-voter (learners can't serve linearizable reads)
- Replication lag (follower behind leader)
- Leader not achieving quorum

**Fix**:
```bash
# Increase sleep time in verify_replication() to 3 seconds
# Check follower logs for replication messages
grep -i "append_entries\|replicate" n*.log
```

#### Membership Change Failures

**Symptom**: `add_learner()` or `change_membership()` returns error

**Possible causes**:
- Calling on non-leader node
- Missing `raft_addr` field in request
- Learner not reachable

**Fix**:
```bash
# Always call membership changes on leader
leader=$(curl -s http://127.0.0.1:21001/metrics | grep current_leader | grep -oE '[0-9]+$')
curl "http://127.0.0.1:2100$leader/add-learner" -H "Content-Type: application/json" -d '...'
```

## CI Integration

### Recommended CI Pipeline

```yaml
# Fast checks (every commit)
- name: API Contract Tests
  run: ./scripts/aspen-cluster-smoke.sh

# Slower consensus tests (pre-merge)
- name: Raft Consensus Tests
  run: ./scripts/aspen-cluster-raft-smoke.sh
  if: github.event_name == 'pull_request'
```

### Expected Timing

| Test | Duration | Frequency |
|------|----------|-----------|
| `aspen-cluster-smoke.sh` (deterministic) | ~5 seconds | Every commit |
| `aspen-cluster-raft-smoke.sh` (raft_actor) | ~30-60 seconds | Pre-merge, pre-deploy |

## Architecture Comparison

### Deterministic Backend Flow

```
HTTP Request → DeterministicClusterController
                    ↓
            In-Memory HashMap (single node)
                    ↓
              HTTP Response
```

### RaftActor Backend Flow

```
HTTP Request → RaftControlClient
                    ↓
         RaftActor (ractor message)
                    ↓
        openraft::Raft (consensus)
                    ↓
      IRPC Network (log replication)
                    ↓
         StateMachineStore (redb)
                    ↓
              HTTP Response
```

## Performance Characteristics

### Deterministic Backend

- **Init**: < 100ms
- **Write**: < 10ms
- **Read**: < 5ms
- **Membership Change**: < 10ms
- **Total Test**: ~5 seconds

### RaftActor Backend

- **Init + Election**: 1500-3000ms (election timeout)
- **Write**: 500-2000ms (quorum + replication)
- **Read**: 100-500ms (ReadIndex protocol)
- **Membership Change**: 2000-3000ms (Raft configuration change)
- **Total Test**: 30-60 seconds

## Best Practices

1. **Development Iteration**: Use deterministic backend for rapid feedback
2. **Pre-Commit**: Run deterministic tests to catch API regressions
3. **Pre-Merge**: Run Raft tests to validate consensus behavior
4. **Pre-Deploy**: Run full Raft tests with production-like configuration
5. **Production Verification**: Deploy with Raft backend, never deterministic

## Further Reading

- [OpenRaft Documentation](https://docs.rs/openraft/)
- [Raft Consensus Algorithm](https://raft.github.io/)
- [Aspen Architecture](../CLAUDE.md)
- [Cluster Bootstrap Guide](cluster-smoke.md)
