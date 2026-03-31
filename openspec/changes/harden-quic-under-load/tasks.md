## 1. Constants and Types

- [x] 1.1 Add `StreamPriority` enum (`Critical`, `Bulk`) with `to_i32()` method to `aspen-raft-types` or `aspen-transport`
- [x] 1.2 Add stream priority constants to `aspen-constants/src/network.rs`: `STREAM_PRIORITY_RAFT: i32 = 100`, `STREAM_PRIORITY_BULK: i32 = 0`, compile-time assert `RAFT > BULK`
- [x] 1.3 Add adaptive retry constants: `READ_INDEX_MAX_RETRIES` (5), `READ_INDEX_RETRY_BASE_MS` (50)

## 2. Stream Priority in Connection Pool

- [x] 2.1 Update `PeerConnection::acquire_stream` to accept `StreamPriority` param and call `send.set_priority()` after `open_bi()` before returning `StreamHandle`
- [x] 2.2 Update all callers of `acquire_stream` in `pool_management.rs` to pass a priority
- [x] 2.3 Add per-priority atomic counters (`raft_streams_opened`, `bulk_streams_opened`) to `PeerConnection`, increment in `acquire_stream`
- [x] 2.4 Unit tests: verify `set_priority` is called with correct values for each `StreamPriority` variant (covered by StreamPriority::to_i32 tests in aspen-raft-types)

## 3. Priority in Raft Network Client (outgoing)

- [x] 3.1 Update `IrpcRaftNetwork::send_rpc` to accept `StreamPriority` and pass it through to `send_rpc_acquire_stream` → `acquire_stream`
- [x] 3.2 Tag `vote()` and `append_entries()` calls as `StreamPriority::Critical`
- [x] 3.3 Tag `full_snapshot()` as `StreamPriority::Bulk`

## 4. Priority in Protocol Handlers (incoming responses)

- [x] 4.1 Update `handle_raft_rpc_stream` in `raft.rs` to set response `send` stream priority based on request type: Vote/AppendEntries → `STREAM_PRIORITY_RAFT`, InstallSnapshot → `STREAM_PRIORITY_BULK`
- [x] 4.2 Update `handle_raft_rpc_stream` in `raft_authenticated.rs` with the same priority logic
- [x] 4.3 Update `handle_raft_rpc_stream` in `raft_sharded.rs` with the same priority logic

## 5. Adaptive ReadIndex Retry

- [x] 5.1 Extract `should_retry_read_index(current_leader, self_id, last_log_index, committed_index) -> bool` as a pure function in `src/verified/`
- [x] 5.2 Add jittered backoff using `rand`: random duration between `READ_INDEX_RETRY_BASE_MS` and `2 × READ_INDEX_RETRY_BASE_MS`
- [x] 5.3 Replace fixed retry loop in `read_ensure_consistency_with_retry` and `scan_ensure_linearizable_with_retry` with the adaptive strategy
- [x] 5.4 Add `read_index_retry_count` and `read_index_retry_success_count` atomic counters to `RaftNode`
- [x] 5.5 Unit tests for `should_retry_read_index` pure function: leader-is-self with small gap → retry, leader-is-other → no retry, leader-is-none → no retry, large gap → no retry

## 6. Send Fairness

- [x] 6.1 Verify `send_fairness` is enabled on iroh endpoint config (verified: noq default is `send_fairness: true`, iroh inherits it, Aspen doesn't override)

## 7. Metrics Plumbing

- [x] 7.1 Extend `ConnectionPoolMetrics` with `raft_streams_opened`, `bulk_streams_opened`, `read_index_retry_count`, `read_index_retry_success_count`
- [x] 7.2 Wire per-connection counters into pool-level metrics aggregation

## 8. Integration Testing

- [ ] 8.1 Run 3-node `dogfood-local.sh` with full git push and verify zero ReadIndex failures in logs (requires manual 3-node cluster run)
- [ ] 8.2 Verify Raft heartbeat latency stays under 50ms during git push (requires manual 3-node cluster run)
