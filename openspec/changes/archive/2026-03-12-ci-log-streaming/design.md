## Context

CI job logs are written to KV as `_ci:logs:{run_id}:{job_id}:{chunk_index:010}` by `CiLogWriter`, flushed every 500ms or when the 8KB buffer fills. The CLI's `ci logs --follow` polls `CiGetJobLogs` RPC every 1 second. The TUI fetches logs once and sets `is_streaming = true` but never actually polls for updates.

A `WatchSession` already exists in `aspen-client` that connects via `LOG_SUBSCRIBER_ALPN`, authenticates with the cluster cookie, and subscribes to KV prefix changes with historical replay. The Raft log subscriber (`aspen-raft/src/log_subscriber.rs`) pushes committed KV operations to subscribers in real-time. This is the same mechanism used by the coordination watch system.

The CLI's `AspenClient` has the iroh `Endpoint` and `bootstrap_addrs` internally but doesn't expose them. `WatchSession::connect` needs both plus the `cluster_id` (used as the cluster cookie for HMAC auth).

## Goals / Non-Goals

**Goals:**

- Sub-second log latency in CLI `--follow` mode (down from 1s polling)
- Eliminate empty polling RPCs during idle periods
- TUI gets live log updates without periodic refresh
- Graceful fallback to polling if watch connection fails

**Non-goals:**

- Changing the log storage format or KV key structure
- Multi-job log multiplexing (one watch subscription per job is fine)
- Log retention or garbage collection changes

## Decisions

### 1. CLI uses WatchSession from aspen-client library

The `WatchSession` in `crates/aspen-client/src/watch.rs` handles connection, auth, and prefix subscription. The CLI needs access to the iroh `Endpoint` and a peer's `EndpointAddr` to create one.

**Approach**: Add accessor methods to `AspenClient`:

- `pub fn endpoint(&self) -> &Endpoint`
- `pub fn cluster_id(&self) -> &str` (already exists, just needs `#[allow(dead_code)]` removed)
- `pub fn first_peer_addr(&self) -> Option<&EndpointAddr>`

**Alternative considered**: Build a new streaming RPC on `CLIENT_ALPN`. Rejected — the watch infrastructure already works and is tested. Adding another streaming mechanism duplicates effort.

### 2. Hybrid catch-up + watch pattern

For `ci logs --follow`:

1. Fetch all existing chunks via `CiGetJobLogs` (historical catch-up)
2. Note the last chunk index
3. Open `WatchSession` subscribing to `_ci:logs:{run_id}:{job_id}:` from the Raft log index at catch-up time
4. Filter watch events to only process Set operations whose keys match the log chunk prefix
5. Parse chunk JSON from watch event values, print content
6. Stop when completion marker key (`__complete__`) appears

This avoids missing chunks during the transition from polling to watching.

**Alternative considered**: Watch-only from start (no catch-up). Rejected — if the job already has logs, the client would need to reconstruct from watch history which requires `start_index = 0` and replaying potentially large history.

### 3. Fallback to polling on watch failure

If `WatchSession::connect` fails (e.g., node doesn't support `LOG_SUBSCRIBER_ALPN`, or connection drops), fall back to the existing 1-second polling loop silently.

### 4. TUI integration via background watch task

The TUI spawns a background `tokio::spawn` task that holds a `WatchSubscription` and sends parsed `CiLogLine` entries to the main app via an `mpsc` channel. The app event loop drains this channel on each tick.

**Alternative considered**: Polling timer in the TUI event loop. Rejected — the watch approach is already available and avoids adding another timer to the event loop.

### 5. Watch subscription filter by key prefix

`WatchSession::subscribe(prefix, start_index)` already supports key prefix filtering. The prefix `_ci:logs:{run_id}:{job_id}:` scopes events to exactly one job's log chunks plus its completion marker. No server-side changes needed.

## Risks / Trade-offs

- **[Watch connection may fail on followers]** → The log subscriber runs on all nodes, not just the leader. The CLI can connect to any bootstrap peer for watching. If the connection drops, fallback to polling handles it.
- **[Extra connection per follow session]** → Each `--follow` opens a separate QUIC connection on `LOG_SUBSCRIBER_ALPN` alongside the `CLIENT_ALPN` connection. Bounded by `MAX_LOG_SUBSCRIBERS = 100` per node.
- **[Chunk ordering in watch events]** → KV writes are Raft-ordered, so watch events arrive in commit order. Chunk indices will be monotonic.

## Migration Plan

No migration needed. The change is purely client-side (CLI + TUI). The server already supports everything via `LOG_SUBSCRIBER_ALPN`. Existing `ci logs` without `--follow` continues using `CiGetJobLogs` as before.

## Open Questions

None — all pieces exist, this is wiring them together.
