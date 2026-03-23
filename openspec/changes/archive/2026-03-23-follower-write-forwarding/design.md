## Architecture

When `RaftNode::write()` gets `ForwardToLeader` from `raft().client_write()`, it currently returns `NotLeader`. The fix adds a retry path that forwards the write to the leader via iroh QUIC, using the same connection pool already maintained for Raft RPCs.

```
  Follower Node                              Leader Node
  ┌─────────────────────┐                    ┌─────────────────────┐
  │ Worker              │                    │                     │
  │   └─ ack_job()      │                    │                     │
  │       └─ KV write() │                    │                     │
  │           │         │                    │                     │
  │     ForwardToLeader │                    │                     │
  │           │         │                    │                     │
  │     WriteForwarder  │ ──iroh QUIC──────► │ ForwardedWrite RPC  │
  │           │         │                    │   └─ KV write()     │
  │           │         │ ◄────result─────── │       └─ raft()     │
  │     Ok(WriteResult) │                    │          .client_   │
  └─────────────────────┘                    │           write()   │
                                             └─────────────────────┘
```

## Key Design Decisions

### 1. WriteForwarder trait injected into RaftNode

Add an optional `Arc<dyn WriteForwarder>` to `RaftNode`. When set, `write()` catches `ForwardToLeader` and delegates to the forwarder. The forwarder uses the existing `RaftConnectionPool` to reach the leader.

```rust
#[async_trait]
pub trait WriteForwarder: Send + Sync {
    async fn forward_write(
        &self,
        leader_id: NodeId,
        request: WriteRequest,
    ) -> Result<WriteResult, KeyValueStoreError>;
}
```

This keeps `RaftNode` independent of the network layer. The forwarder is injected during cluster bootstrap when the connection pool is available.

### 2. Reuse CLIENT_ALPN, not a new ALPN

Forwarded writes go through the existing client RPC protocol. The leader already handles `WriteRequest` via the `ClientProtocolHandler`. The forwarder acts as an internal client connecting to the leader's client handler.

Alternative considered: new `WRITE_FORWARD_ALPN` — rejected because it duplicates the write handling logic and requires a new protocol handler.

### 3. Single retry with leader hint

On `ForwardToLeader`, try once to forward to the indicated leader. If the indicated leader is also not the leader (stale hint), return `NotLeader` — the caller's existing retry loop handles it. No recursive forwarding, no loops.

### 4. Write batcher bypass on forwarding

When forwarding, bypass the local write batcher entirely. The batcher is a leader-side optimization. The forwarded write goes directly to the leader, which applies its own batcher.

### 5. Worker stats writes: downgrade from WARN to DEBUG

The `failed to write worker stats to KV` warnings flood logs after every leader change (8 workers × every 10s). These are non-critical — downgrade to DEBUG. The actual critical failure (job ack) gets its own error path.

## Components Modified

1. **`crates/aspen-raft/src/node/mod.rs`** — Add `write_forwarder: Option<Arc<dyn WriteForwarder>>` field, setter method
2. **`crates/aspen-raft/src/node/kv_store.rs`** — Catch `ForwardToLeader` in `write()`, delegate to forwarder
3. **`crates/aspen-raft/src/write_forwarder.rs`** (new) — `WriteForwarder` trait + `IrohWriteForwarder` implementation using iroh connections
4. **`crates/aspen-cluster/src/bootstrap/`** — Wire up `WriteForwarder` during node initialization
5. **`crates/aspen-jobs/src/worker.rs`** — Downgrade worker stats WARN to DEBUG

## Not Changed

- `atomic_update_job` retry logic stays (handles version conflicts, not just leader changes)
- `MAX_NOT_LEADER_RETRIES` stays at 100 as a safety net
- Client API path unchanged (clients already connect directly to leader via iroh discovery)
