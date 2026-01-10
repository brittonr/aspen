# Test Coverage Gaps

**Severity:** MEDIUM
**Category:** Testing
**Date:** 2026-01-10

## Summary

Several critical modules have minimal direct unit test coverage, relying primarily on integration tests.

## Coverage Statistics

- **Total integration tests:** 1222+ test functions
- **Total unit tests in aspen-raft:** 289+ unit tests
- **Simulation/chaos tests:** 26+ madsim scenarios

## Critical Coverage Gaps

### 1. Network Layer - 0 Unit Tests

**File:** `crates/aspen-raft/src/network.rs` (881 lines)

- IrpcRaftNetworkFactory
- IrpcRaftNetwork RPC methods
- Connection pooling

### 2. RaftNode Implementation - 2 Unit Tests

**File:** `crates/aspen-raft/src/node.rs` (1547 lines)

- ClusterController operations
- KeyValueStore operations
- Coverage: 0.13%

### 3. Write Batcher - 3 Unit Tests

**File:** `crates/aspen-raft/src/write_batcher.rs` (815 lines)

- Batch timing
- Limit enforcement
- Coverage: 0.37%

### 4. Server/Listener - 0 Unit Tests

**File:** `crates/aspen-raft/src/server.rs`

- spawn() and shutdown() lifecycle
- Connection limit enforcement

### 5. Worker Coordinator - 2 Unit Tests

**File:** `crates/aspen-coordination/src/worker_coordinator.rs` (1227 lines)

### 6. Queue - No Unit Tests Found

**File:** `crates/aspen-coordination/src/queue.rs` (1391 lines)

## Edge Cases Not Tested

### Consensus Critical

- Single-node cluster behavior
- Read linearizability under partition
- Snapshot installation during writes
- Membership changes during snapshots

### Storage Critical

- Disk full during snapshot/append
- Database corruption detection
- Concurrent snapshot reads

### Networking Critical

- Connection timeout during append_entries
- Partial RPC message loss
- Max message size enforcement

## Recommendation

1. Add unit tests for network layer RPC methods
2. Add unit tests for write batcher limits
3. Add property-based tests for storage validation
4. Add edge case tests for consensus operations
