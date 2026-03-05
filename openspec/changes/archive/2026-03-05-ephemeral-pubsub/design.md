## Architecture

```text
                    Local publisher                   Remote subscriber
                         │                                  │
                    publish_ephemeral()              connect(ALPN)
                         │                                  │
                         ▼                                  ▼
                  ┌──────────────┐               ┌───────────────────┐
                  │  Ephemeral   │               │  EphemeralProto   │
                  │   Broker     │◄──subscribe───│  colHandler       │
                  │  (per-node)  │               │  (QUIC accept)    │
                  └──────┬───────┘               └───────────────────┘
                         │
            match topic against subscriptions
                         │
              ┌──────────┼──────────┐
              ▼          ▼          ▼
          local rx    local rx   QUIC stream
         (mpsc)      (mpsc)     (SendStream)
```

No Raft. No KV. No replication. The broker is per-node, in-memory only.

## Components

### EphemeralBroker

Lives in `aspen-hooks/src/pubsub/ephemeral/broker.rs`.

```rust
pub struct EphemeralBroker {
    subscriptions: RwLock<Vec<ActiveSubscription>>,
}

struct ActiveSubscription {
    id: u64,
    pattern: TopicPattern,
    sender: mpsc::Sender<Event>,
}
```

Core operations:

- `subscribe(pattern) -> (subscription_id, mpsc::Receiver<Event>)` — registers a local subscription
- `unsubscribe(subscription_id)` — removes a subscription
- `publish(topic, event)` — matches against all active subscriptions, sends to matching receivers via `try_send` (non-blocking, drops on full buffer)

The broker holds a `RwLock<Vec<ActiveSubscription>>`. Publish takes a read lock (concurrent publishes are fine), subscribe/unsubscribe take a write lock. The subscription list is expected to be small (tens, not thousands).

`try_send` is deliberate: a slow subscriber gets dropped events rather than blocking the publisher or growing memory without bound. This is the correct tradeoff for ephemeral streaming — if you need guaranteed delivery, use the durable pub/sub.

### EphemeralPublisher

Lives in `aspen-hooks/src/pubsub/ephemeral/publisher.rs`.

```rust
pub struct EphemeralPublisher {
    broker: Arc<EphemeralBroker>,
}

#[async_trait]
impl Publisher for EphemeralPublisher {
    async fn publish(&self, topic: &Topic, payload: &[u8]) -> Result<Cursor> {
        self.broker.publish(topic, payload);
        Ok(Cursor::EPHEMERAL) // sentinel value, no Raft log index
    }
}
```

Implements the existing `Publisher` trait. The `Cursor` returned is a sentinel (`Cursor::EPHEMERAL`) since there's no Raft log index. Code that's generic over `Publisher` works with both durable and ephemeral delivery. `publish_batch` iterates and publishes each event individually (no atomicity guarantee needed for ephemeral).

### EphemeralProtocolHandler

Lives in `aspen-transport/src/ephemeral/handler.rs`. Implements `iroh::protocol::ProtocolHandler`.

**ALPN**: `aspen-ephemeral/0`

**Protocol flow**:

1. Remote client connects via ALPN
2. Server accepts bidirectional stream
3. Client sends `EphemeralSubscribeRequest` (serialized via postcard): topic pattern string
4. Server validates pattern, registers subscription with broker
5. Server streams `EphemeralEvent` messages on the send stream (length-prefixed postcard)
6. When client disconnects or stream closes, subscription is removed

```rust
impl ProtocolHandler for EphemeralProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let (send, recv) = connection.accept_bi().await?;
        // Read subscription request
        let request: EphemeralSubscribeRequest = read_request(recv).await?;
        let pattern = TopicPattern::new(&request.pattern)?;
        // Register with broker
        let (sub_id, mut rx) = self.broker.subscribe(pattern);
        // Stream events until disconnect
        let result = stream_events(send, &mut rx).await;
        self.broker.unsubscribe(sub_id);
        result
    }
}
```

**Wire format**: Length-prefixed postcard frames. Each frame is a 4-byte big-endian length followed by postcard-serialized `EphemeralEvent`. Simple, fast, no framing ambiguity.

**Bounded resources**:

- Max concurrent subscriptions per connection: 1 (one stream = one subscription)
- Max concurrent connections: bounded by semaphore (same pattern as `ClientProtocolHandler`)
- Buffer size per subscription: configurable, default 256 events (via `mpsc::channel(256)`)

### RouterBuilder::custom()

Added to `aspen-cluster/src/router_builder.rs`:

```rust
pub fn custom(mut self, alpn: &[u8], handler: impl ProtocolHandler) -> Self {
    self.builder = self.builder.accept(alpn, handler);
    tracing::info!(alpn = %String::from_utf8_lossy(alpn), "registered custom protocol handler");
    self
}
```

One method. The ephemeral handler uses it:

```rust
manager.spawn_router_with(|b| b
    .auth_raft(raft_handler)
    .client(client_handler)
    .custom(EPHEMERAL_ALPN, ephemeral_handler));
```

This also unblocks any external application that needs a custom protocol on the shared endpoint.

### Cursor::EPHEMERAL

New sentinel value on `Cursor`:

```rust
impl Cursor {
    pub const EPHEMERAL: Cursor = Cursor(u64::MAX - 1);

    pub fn is_ephemeral(&self) -> bool {
        self.0 == u64::MAX - 1
    }
}
```

Distinguishes ephemeral publishes from durable ones. Code that checks `cursor.is_ephemeral()` knows not to use it for resumption.

## File Layout

```
crates/aspen-hooks/src/pubsub/
├── ephemeral/
│   ├── mod.rs           # Re-exports
│   ├── broker.rs        # EphemeralBroker (subscription registry + dispatch)
│   └── publisher.rs     # EphemeralPublisher (Publisher trait impl)

crates/aspen-transport/src/ephemeral/
│   ├── mod.rs           # Re-exports + ALPN constant
│   ├── handler.rs       # EphemeralProtocolHandler (ProtocolHandler impl)
│   └── wire.rs          # Wire types (EphemeralSubscribeRequest, EphemeralEvent)

crates/aspen-cluster/src/
│   └── router_builder.rs  # Add custom() method

crates/aspen-hooks/src/pubsub/
│   └── cursor.rs          # Add Cursor::EPHEMERAL sentinel
```

## Constraints

- **No Raft in the publish path.** The entire point is sub-millisecond delivery. If Raft touches the hot path, this is pointless.
- **No unbounded buffers.** `mpsc::channel` with fixed capacity + `try_send`. Slow subscribers drop events.
- **No cross-node relay.** A subscriber must connect to the node where events are published. If you need cluster-wide ephemeral pub/sub, that's a future change involving gossip or a forwarding layer.
- **No persistence.** Events exist only in flight. If nobody is subscribed, published events are silently dropped.
- **Same topic/pattern types.** Reuse `Topic` and `TopicPattern` from the existing pub/sub. No new topic format.
- **Connection-scoped subscriptions.** When the QUIC connection drops, the subscription is removed. No dangling state.

## What This Doesn't Do

- **Replace durable pub/sub.** Durable pub/sub (`RaftPublisher`) is unchanged. Use it when you need ordering, durability, consumer groups, or cursors.
- **Cluster-wide broadcast.** Events only reach subscribers connected to the publishing node. For broadcast, use gossip.
- **Authentication beyond transport.** Relies on Iroh QUIC transport-level identity. No application-level auth (cookie challenge, capability tokens). If needed, add it to the protocol handler later — same pattern as `LogSubscriberProtocolHandler`.
