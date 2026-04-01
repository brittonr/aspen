## Context

Aspen uses a single iroh QUIC endpoint for all inter-node traffic: Raft consensus (heartbeats, AppendEntries, Vote, InstallSnapshot), client RPC, git bridge object transfer, blob replication, gossip, and federation sync. All protocols share the same QUIC connection and congestion window per peer.

During 3-node dogfood with a 33K-object git push, the git bridge writes thousands of KV entries through Raft while the Raft protocol itself needs heartbeats and quorum confirmations to flow. QUIC's default stream priority is 0 for all streams — bulk data and Raft heartbeats get equal scheduling weight. A burst of git object writes fills the send buffer, delaying heartbeat responses by 21-140ms. This crosses Raft's tolerance and causes:

1. ReadIndex quorum confirmations time out (leader can't get ACKs from followers)
2. Followers see the leader as temporarily unreachable
3. Forwarded reads fail, `get_job` returns None, CI pipeline stalls

The existing retry logic (3 attempts × 50-150ms backoff) partially masks this but doesn't fix root cause: Raft protocol bytes compete equally with bulk data bytes for the same congestion window.

**Key discovery**: noq (iroh's QUIC implementation) exposes `SendStream::set_priority(i32)` — native QUIC stream prioritization. Higher-priority streams' buffered data is transmitted before lower-priority streams'. This operates at the QUIC scheduler level, directly controlling which bytes hit the wire first. This is the correct fix.

**Current constants:**

- `IROH_CONNECT_TIMEOUT_SECS`: 5s
- `IROH_STREAM_OPEN_TIMEOUT_SECS`: 2s
- `IROH_READ_TIMEOUT_SECS`: 10s
- `MAX_STREAMS_PER_CONNECTION`: 100
- `READ_INDEX_TIMEOUT_SECS`: 5s
- `ReadIndex retries`: 3 (hardcoded in kv_store.rs)
- All streams default to priority 0

## Goals / Non-Goals

**Goals:**

- Raft heartbeats and votes are scheduled ahead of bulk data on the wire, completing within 50ms even during sustained bulk transfers
- ReadIndex succeeds on first attempt during normal load (retries only during actual leader transitions)
- Git push of 33K objects completes without causing Raft quorum failures on a 3-node cluster
- Operators can observe stream contention via metrics before it causes failures
- Changes are backward-compatible (stream priority is sender-local, no wire protocol change)

**Non-Goals:**

- Custom congestion control algorithms
- Multi-connection-per-peer (QUIC multiplexing within one connection is the intended design)
- QoS at the OS/network level
- Changing the Raft election timeout or heartbeat interval

## Decisions

### 1. Use noq's native `SendStream::set_priority()` for Raft vs bulk scheduling

noq exposes `set_priority(i32)` on `SendStream`. Higher values = transmitted first. Default is 0. The QUIC scheduler in noq guarantees "higher priority streams always take precedence over lower priority streams."

Define two priority levels:

- **`STREAM_PRIORITY_RAFT`** (100): Raft Vote, AppendEntries (heartbeats), small AppendEntries batches
- **`STREAM_PRIORITY_BULK`** (0): Snapshot install, git bridge writes, blob transfer, federation sync

After `open_bi()`, immediately call `send.set_priority(priority)` before writing any bytes. This costs nothing — it's a single mutex-guarded field write inside noq.

**Why this works**: The congestion window is connection-scoped. When it's full, noq's scheduler decides which stream's buffered data to send next. With priorities, Raft heartbeat bytes jump the queue. The bulk data still flows — it just yields when Raft traffic is pending.

**Alternative rejected: Application-level semaphores splitting stream slots into tiers.** This limits concurrency (how many streams can open) but doesn't affect scheduling (which bytes get sent first). A heartbeat on a "reserved" slot still competes for the same congestion window. Semaphores solve the wrong problem.

**Alternative rejected: Token-bucket rate limiting on bulk writes.** This artificially slows throughput even when the network has spare capacity. Priority scheduling lets bulk data use full bandwidth when Raft is idle, and yields automatically when Raft traffic arrives.

### 2. Priority on both Raft protocol handler and network client

Priority must be set on both sides of the connection:

- **Outgoing (network client)**: `IrpcRaftNetwork::send_rpc` calls `send.set_priority()` after `open_bi()`, before `write_all()`.
- **Incoming (protocol handler)**: `handle_raft_rpc_stream` calls `send.set_priority()` on the response send stream after `accept_bi()`, before writing the response.

This ensures both request and response bytes for Raft RPCs are prioritized.

### 3. Adaptive ReadIndex retry with metrics-based decisions

Replace the fixed `READ_INDEX_RETRIES = 3` with a strategy that checks Raft metrics before retrying:

- If `current_leader == Some(self)` and the log is reasonably current (last_log near committed), retry with jittered backoff — this is transient contention.
- If `current_leader == None` or `current_leader != self`, fail fast — this is a real leadership change.
- Max retries: `READ_INDEX_MAX_RETRIES` (5), base backoff 50-100ms with jitter.

With stream priorities, these retries should almost never fire — but they remain as a safety net for genuine contention (e.g., during snapshot install which is large even at bulk priority).

### 4. Transport metrics via atomic counters

Add per-connection atomic counters tracking:

- Streams opened at each priority level
- ReadIndex retry count and success-after-retry count

Exposed via the existing `ConnectionPoolMetrics` struct. No new dependencies.

### 5. Enable `send_fairness` for bulk streams

iroh exposes `send_fairness(bool)` at the endpoint/transport config level. When enabled, streams at the same priority get round-robin scheduling instead of FIFO. This prevents a single large git object from starving other bulk streams at the same priority level.

We should ensure this is enabled (it may already be the default in noq).

## Risks / Trade-offs

**[Risk] Priority starvation of bulk streams** → Under sustained Raft traffic (unlikely — heartbeats are small, ~100 bytes), bulk streams could be starved. Mitigated by Raft traffic being inherently bursty and small. A heartbeat + response is ~200 bytes, completing in microseconds once scheduled.

**[Risk] `set_priority` after `open_bi` has a race window** → Between `open_bi()` and `set_priority()`, the stream briefly has default priority. Mitigated because no data is written until after priority is set, so there's nothing for noq to schedule during that window.

**[Risk] noq's "many different priority levels may have negative impact on performance"** → We use exactly 2 levels (0 and 100). This is well within the intended design.

**[Risk] Backward compatibility** → Stream priority is sender-local. The receiver doesn't know or care what priority the sender used. Nodes can be upgraded independently.

## Open Questions

- Should snapshot install use an intermediate priority (e.g., 50) instead of bulk (0)? Snapshots are critical for cluster health but are also large. Starting at bulk priority; can tune later based on dogfood results.
- Should the `send_fairness` setting be configurable or hardcoded? Starting hardcoded-on.
