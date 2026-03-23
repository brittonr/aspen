## Why

When Raft leadership changes during a long-running job (e.g., nix clippy build taking 30+ min), the worker's ack/nack fails permanently. The worker holds a reference to the local `RaftNode`'s `KeyValueStore`, which returns `NotLeader` on every write attempt. The job manager retries 100 times with exponential backoff, but all retries hit the same follower — it never forwards the write to the new leader. The job stays "Running" in KV forever, blocking the entire CI pipeline.

Observed in 3-node multi-node dogfood: network blip at 04:39 → election → node 3 became leader → clippy job on node 1 couldn't ack → pipeline stuck indefinitely.

## What Changes

Add transparent write-forwarding to `RaftNode`'s `KeyValueStore::write()` implementation. When a follower gets `ForwardToLeader` from `raft().client_write()`, instead of returning `NotLeader`, it forwards the write to the leader via the Raft network (same iroh QUIC transport used for AppendEntries). This makes all internal KV writes leader-transparent — workers, coordination primitives, and any code using the local `KeyValueStore` automatically reach the leader.

## Capabilities

### New Capabilities

- `write-forwarding`: Transparent follower-to-leader write forwarding in RaftNode's KeyValueStore implementation

### Modified Capabilities

- None

## Scope

- RaftNode's `KeyValueStore::write()` in `crates/aspen-raft/src/node/kv_store.rs`
- Raft network transport for forwarding writes (new ALPN or reuse existing)
- Job manager retry logic (can be simplified once forwarding works)
- Worker stats writes (currently spamming WARN on every 10s cycle after leader change)

## Out of Scope

- Read forwarding (reads already handle NotLeader via linearizer)
- Client API changes (clients already retry via iroh discovery)
- Changing the Raft protocol itself

## Risks

- Forwarding adds latency (follower → leader hop)
- Must handle the case where the forwarding target is also not the leader (stale info)
- Must avoid infinite forwarding loops
- Write batcher interaction: batched writes on followers need forwarding too
