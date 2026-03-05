## Why

Aspen has a full pub/sub system (`aspen-hooks/src/pubsub/`) with topics, wildcards, consumer groups, and cursors. Every publish goes through Raft consensus and fsync to redb. That's correct for durable event delivery — hooks, webhooks, event sourcing — where you need ordering guarantees and survival across node failures.

But there's no mechanism for streaming ephemeral data between nodes at interactive latency. The Raft path costs 2-5ms per event. For use cases like LLM token streaming (50+ tokens/second to a remote TUI), that's a 100-250ms pipeline delay that makes the UI feel broken. You need sub-millisecond, fire-and-forget delivery where dropping a frame is acceptable but fsync latency is not.

The existing alternatives don't work either:

- **Gossip**: Unordered, 10-100ms latency, broadcast to all peers
- **KV watch**: Same Raft path as pub/sub
- **Hooks**: Post-commit reactions, not real-time push
- **Client RPC**: Request-response only, no streaming

Ephemeral data in flight is not distributed state. It doesn't need consensus, durability, or total ordering. It needs a direct QUIC stream from publisher to subscriber with topic-based routing.

## What Changes

- **New `EphemeralPublisher` in `aspen-hooks/src/pubsub/`**: A publisher implementation that delivers events directly to connected subscribers over QUIC streams, bypassing Raft entirely. Reuses existing `Topic`, `TopicPattern`, `Event`, and wildcard matching — same API surface, different delivery backend.

- **New `EphemeralBroker`**: Per-node in-memory broker that tracks local subscriptions and routes events to matching subscribers. No KV storage, no replication. When an event is published, it's matched against active subscriptions and pushed to subscriber streams. If no subscribers match, the event is dropped silently.

- **New ALPN protocol (`aspen-ephemeral/0`)**: Registered on the Iroh Router via `RouterBuilder`. Remote clients connect, send a subscription request (topic pattern), and receive a stream of matching events. Uses the existing `ProtocolHandler` trait.

- **`RouterBuilder::custom()` method**: Generic escape hatch on `RouterBuilder` for registering arbitrary ALPN + handler pairs. The ephemeral protocol uses this, and it also unblocks external applications (like clankers) that need custom protocols on the shared endpoint.

- **Same `Publisher` trait**: `EphemeralPublisher` implements the existing `Publisher` trait so callers can be generic over durable vs. ephemeral delivery. The only API difference is construction — you get an `EphemeralPublisher` from the broker instead of from a KV store.

## Capabilities

### New Capabilities

**Ephemeral topic publish**:

- Publish events to a topic with sub-millisecond delivery to connected subscribers
- Same `Topic` and wildcard matching as durable pub/sub
- Fire-and-forget semantics — no Raft, no fsync, no persistence
- Bounded in-memory buffers with backpressure (slow subscriber gets dropped events, not unbounded memory growth)

**Ephemeral topic subscribe (local)**:

- Subscribe to a topic pattern on the local node
- Returns a `tokio::sync::broadcast` or `mpsc` receiver
- Wildcard patterns (`*`, `>`) work identically to durable pub/sub

**Ephemeral topic subscribe (remote)**:

- Connect to a remote node via `aspen-ephemeral/0` ALPN
- Send a subscription request with topic pattern
- Receive events streamed over a QUIC uni-stream
- Automatic reconnection semantics left to the subscriber

**Custom ALPN registration**:

- `RouterBuilder::custom(alpn, handler)` for registering arbitrary protocols
- Enables external applications to register protocol handlers on the shared Iroh endpoint

### Existing Capabilities (unchanged)

- Durable pub/sub through `RaftPublisher` — still goes through Raft
- Consumer groups, cursors, resumable subscriptions — all unchanged
- All existing ALPN protocols — unchanged

## Out of Scope

- Cross-node pub/sub relay (events are only delivered to subscribers connected to the publishing node)
- Durable fallback (if you need durability, use `RaftPublisher`)
- Guaranteed delivery or ordering beyond QUIC stream ordering
- Encryption or auth beyond what Iroh QUIC provides at the transport layer
