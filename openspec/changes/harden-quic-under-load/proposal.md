## Why

During 3-node dogfood runs with heavy git push workloads (33K objects), QUIC stream contention causes 21-140ms node unreachability blips. These blips cause ReadIndex quorum confirmations to fail, which cascades: forwarded reads return `None`, `get_job` returns `JobNotFound`, and CI pipeline steps stall or fail. The root cause is that all QUIC streams default to priority 0 — Raft heartbeat bytes and bulk git object bytes compete equally for the congestion window. noq (iroh's QUIC implementation) exposes `SendStream::set_priority(i32)` which schedules higher-priority stream data first. We should use it.

## What Changes

- **QUIC stream priority for Raft traffic**: Set `SendStream::set_priority(100)` on all Raft heartbeat, AppendEntries, and Vote streams so they are scheduled ahead of bulk data (priority 0) by noq's QUIC scheduler. Set on both outgoing (network client) and incoming (protocol handler response) streams.
- **Adaptive ReadIndex retry**: Replace the fixed 3×50ms retry with an adaptive strategy that checks Raft metrics (leader identity, log lag) to decide whether to retry or fail fast. Add jittered backoff.
- **Transport metrics**: Expose per-priority stream counts and ReadIndex retry counters so operators can observe contention.
- **Send fairness**: Ensure `send_fairness` is enabled so bulk streams at the same priority get round-robin scheduling.

## Capabilities

### New Capabilities

- `stream-priority`: QUIC stream prioritization using noq's native `set_priority()` API to schedule Raft protocol traffic ahead of bulk data
- `bulk-transfer-throttle`: Bulk streams use default priority (0) and fair queuing; yields to Raft traffic under congestion
- `transport-metrics`: Per-priority stream counters and ReadIndex retry counters

### Modified Capabilities

- `transport`: `PeerConnection::acquire_stream` accepts `StreamPriority` and sets noq priority on the `SendStream`
- `consensus`: Adaptive ReadIndex retry using Raft metrics instead of fixed retry counts

## Impact

- **crates/aspen-constants/src/network.rs**: New constants `STREAM_PRIORITY_RAFT`, `STREAM_PRIORITY_BULK`, `READ_INDEX_MAX_RETRIES`, `READ_INDEX_RETRY_BASE_MS`
- **crates/aspen-raft/src/connection_pool/peer_connection.rs**: `acquire_stream` takes `StreamPriority`, calls `send.set_priority()`
- **crates/aspen-raft/src/network/client.rs**: Priority-tagged RPC sends (Vote/AppendEntries → Critical, Snapshot → Bulk)
- **crates/aspen-transport/src/raft.rs, raft_authenticated.rs, raft_sharded.rs**: Response streams set priority based on request type
- **crates/aspen-raft/src/node/kv_store.rs**: Adaptive ReadIndex retry with metrics-based decisions
- **No wire protocol changes**: Stream priority is sender-local; fully backward compatible
