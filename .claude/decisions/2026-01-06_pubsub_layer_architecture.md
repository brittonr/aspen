# Aspen Pub/Sub Layer Architecture Decision

**Date**: 2026-01-06
**Status**: Proposed
**Author**: Claude (ULTRA Mode Analysis)

## Executive Summary

This document proposes a Pub/Sub layer for Aspen that leverages the existing Raft consensus log as an ordered event stream. The architecture provides strong consistency guarantees that NATS JetStream cannot match, while using iroh's networking primitives for efficient fan-out to subscribers.

## Context

The user requested a pub/sub system with the following API:

```rust
// Publish events (goes through Raft for ordering)
pubsub.publish("orders.created", &order_event).await?;

// Subscribe with cursor (resumable)
let mut stream = pubsub.subscribe("orders.*", cursor).await?;
while let Some(event) = stream.next().await {
    process(event)?;
    cursor = event.cursor;  // Checkpoint for resume
}
```

## Key Insight: Aspen Already Has 80% of This

After extensive codebase exploration, I discovered that Aspen already has:

1. **`LogSubscriberProtocolHandler`** (`crates/aspen-transport/src/log_subscriber.rs`):
   - Streams committed Raft log entries over Iroh QUIC
   - Supports prefix filtering (`key_prefix`)
   - Supports resumable subscriptions (`start_index`)
   - Historical replay from any log index
   - HLC timestamps for ordering
   - Keepalive mechanism

2. **`KvOperation`** enum with all operation types (Set, Delete, etc.)

3. **`broadcast::Sender<LogEntryPayload>`** for fan-out to subscribers

4. **FoundationDB-style layer primitives** (`crates/aspen-layer/`) for key organization

## Architecture: Pub/Sub as a Layer over Raft Log

### Layer Stack

```
Application Code
    pubsub.publish("orders.created", &event)
    pubsub.subscribe("orders.*", cursor)
         |
         v
+------------------------------------------+
|           Pub/Sub Layer (NEW)            |
|  - Topic-to-key translation              |
|  - Wildcard matching ("orders.*")        |
|  - Consumer groups (competing consumers) |
|  - Ack/Nack with cursor tracking         |
+------------------------------------------+
         |
         v
+------------------------------------------+
|     Log Subscriber Protocol (EXISTS)     |
|  - LogSubscriberProtocolHandler          |
|  - Prefix-based filtering                |
|  - Historical replay                     |
|  - HLC timestamps                        |
+------------------------------------------+
         |
         v
+------------------------------------------+
|         Raft Consensus (EXISTS)          |
|  - SharedRedbStorage                     |
|  - Linearizable writes                   |
|  - Log index = cursor                    |
+------------------------------------------+
         |
         v
+------------------------------------------+
|      Iroh P2P Transport (EXISTS)         |
|  - QUIC streams                          |
|  - NAT traversal                         |
|  - Multi-path delivery                   |
+------------------------------------------+
```

### Why This Beats NATS

| Feature | NATS JetStream | Aspen Pub/Sub |
|---------|---------------|---------------|
| **Ordering** | Per-stream, eventually consistent | Global linearizable (Raft) |
| **Consistency** | At-least-once by default | Exactly-once via Raft |
| **Cursor** | Consumer sequence number | Raft log index (global) |
| **Replay** | Stream-specific retention | Full Raft log replay |
| **Transactions** | Not supported | Atomic multi-key via Raft |
| **Dependencies** | Separate NATS cluster | Built into Aspen cluster |
| **Wildcards** | Subject patterns | Prefix patterns + globs |
| **Durability** | Configurable | Single-fsync Redb |

### Unique Aspen Advantages from Iroh

1. **NAT Traversal**: QUIC hole-punching works across NATs without port forwarding
2. **Relay Fallback**: If direct connection fails, relay servers provide connectivity
3. **Multi-path**: Multiple connection paths for resilience
4. **Content Addressing**: Blob attachments can be referenced by hash (iroh-blobs)
5. **mDNS Discovery**: Automatic local network peer discovery
6. **Mainline DHT**: Global peer discovery without central servers

## Proposed API

### Core Types

```rust
/// Topic name with hierarchical structure
#[derive(Debug, Clone)]
pub struct Topic(String);

impl Topic {
    /// Create topic from string (e.g., "orders.created", "users.*.updated")
    pub fn new(name: impl Into<String>) -> Self;

    /// Check if pattern matches topic
    pub fn matches(&self, pattern: &TopicPattern) -> bool;
}

/// Topic pattern with wildcards
#[derive(Debug, Clone)]
pub struct TopicPattern {
    segments: Vec<PatternSegment>,
}

enum PatternSegment {
    Literal(String),
    Single,    // * matches one segment
    Multi,     // > matches zero or more segments (must be last)
}

/// Cursor for resumable subscriptions (wraps Raft log index)
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Cursor(u64);

impl Cursor {
    pub const BEGINNING: Cursor = Cursor(0);
    pub const LATEST: Cursor = Cursor(u64::MAX);
}

/// Published event
#[derive(Debug, Clone)]
pub struct Event {
    pub topic: Topic,
    pub payload: Vec<u8>,
    pub cursor: Cursor,
    pub timestamp: HlcTimestamp,
    pub headers: HashMap<String, String>,
}
```

### Publisher API

```rust
#[async_trait]
pub trait Publisher: Send + Sync {
    /// Publish event to topic (goes through Raft consensus)
    async fn publish(&self, topic: &Topic, payload: &[u8]) -> Result<Cursor, PubSubError>;

    /// Publish with headers
    async fn publish_with_headers(
        &self,
        topic: &Topic,
        payload: &[u8],
        headers: HashMap<String, String>,
    ) -> Result<Cursor, PubSubError>;

    /// Publish batch atomically
    async fn publish_batch(&self, events: Vec<(Topic, Vec<u8>)>) -> Result<Cursor, PubSubError>;
}
```

### Subscriber API

```rust
#[async_trait]
pub trait Subscriber: Send + Sync {
    /// Subscribe to topics matching pattern
    async fn subscribe(
        &self,
        pattern: TopicPattern,
        cursor: Cursor,
    ) -> Result<EventStream, PubSubError>;

    /// Subscribe as part of consumer group (competing consumers)
    async fn subscribe_queue(
        &self,
        pattern: TopicPattern,
        group: &str,
        cursor: Cursor,
    ) -> Result<EventStream, PubSubError>;
}

/// Stream of events
pub struct EventStream {
    inner: Pin<Box<dyn Stream<Item = Result<Event, PubSubError>> + Send>>,
}

impl EventStream {
    pub async fn next(&mut self) -> Option<Result<Event, PubSubError>>;

    /// Acknowledge processing up to cursor (for durable subscriptions)
    pub async fn ack(&self, cursor: Cursor) -> Result<(), PubSubError>;
}
```

### Consumer Groups

For competing consumers (like NATS queue groups):

```rust
/// Consumer group for load-balanced event processing
pub struct ConsumerGroup {
    name: String,
    topic_pattern: TopicPattern,
    /// Stored in KV: __pubsub/groups/{name}/cursor
    cursor_key: String,
    /// Stored in KV: __pubsub/groups/{name}/members/{node_id}
    members: Vec<NodeId>,
}
```

Consumer groups use Raft consensus for:

1. Cursor checkpointing (exactly-once delivery)
2. Member registration/heartbeat
3. Partition assignment

## Implementation Plan

### Phase 1: Topic Key Encoding

Use the existing layer primitives for topic-to-key mapping:

```rust
use aspen_layer::{Subspace, Tuple};

const PUBSUB_PREFIX: &[u8] = b"__pubsub/";

fn topic_to_key(topic: &Topic) -> Vec<u8> {
    let subspace = Subspace::new(Tuple::new().push(PUBSUB_PREFIX));
    let mut tuple = Tuple::new();
    for segment in topic.segments() {
        tuple = tuple.push(segment);
    }
    subspace.pack(&tuple)
}
```

### Phase 2: Publish via Raft

```rust
impl Publisher for RaftPublisher {
    async fn publish(&self, topic: &Topic, payload: &[u8]) -> Result<Cursor, PubSubError> {
        let key = topic_to_key(topic);
        let value = encode_event(payload)?;

        // Use existing KeyValueStore trait
        let result = self.kv_store.write(WriteRequest::Set {
            key: String::from_utf8_lossy(&key).to_string(),
            value,
        }).await?;

        // Cursor is the Raft log index
        Ok(Cursor(result.log_index))
    }
}
```

### Phase 3: Subscribe via Log Subscriber

```rust
impl Subscriber for LogSubscriber {
    async fn subscribe(
        &self,
        pattern: TopicPattern,
        cursor: Cursor,
    ) -> Result<EventStream, PubSubError> {
        // Convert pattern to key prefix
        let prefix = pattern_to_prefix(&pattern);

        // Use existing log subscriber protocol
        let request = SubscribeRequest {
            start_index: cursor.0,
            key_prefix: prefix,
            protocol_version: LOG_SUBSCRIBE_PROTOCOL_VERSION,
        };

        let stream = self.connect_and_subscribe(request).await?;

        Ok(EventStream::new(stream, pattern))
    }
}
```

### Phase 4: Consumer Groups (Competing Consumers)

```rust
/// Coordinator for consumer group partition assignment
struct ConsumerGroupCoordinator {
    group_name: String,
    kv_store: Arc<dyn KeyValueStore>,
    node_id: NodeId,
}

impl ConsumerGroupCoordinator {
    /// Register this node as a group member
    async fn register(&self) -> Result<(), PubSubError> {
        let member_key = format!("__pubsub/groups/{}/members/{}",
            self.group_name, self.node_id);

        // Use lease for automatic cleanup on disconnect
        self.kv_store.write(WriteRequest::SetWithLease {
            key: member_key,
            value: self.node_addr.to_string(),
            lease_id: self.member_lease_id,
        }).await?;

        Ok(())
    }

    /// Get events assigned to this consumer
    async fn poll(&self) -> Result<Option<Event>, PubSubError> {
        // Partition assignment based on consistent hashing
        // Only process events where hash(event.key) % members.len() == our_index
        todo!()
    }
}
```

### Phase 5: Gossip-based Fan-out Optimization

For topics with many subscribers, use iroh-gossip for efficient fan-out:

```rust
/// Hybrid delivery: Raft for ordering, Gossip for fan-out
struct HybridDelivery {
    raft_subscriber: LogSubscriber,
    gossip: Arc<Gossip>,
    topic_id: TopicId,
}

impl HybridDelivery {
    /// Leader broadcasts to gossip after Raft commit
    async fn broadcast_committed(&self, entry: LogEntryPayload) {
        let msg = PubSubGossipMessage::Event(entry);
        let bytes = msg.to_bytes();

        // Best-effort gossip fan-out (subscribers also have Raft as fallback)
        if let Err(e) = self.gossip.broadcast(self.topic_id, bytes).await {
            tracing::warn!("gossip broadcast failed (Raft fallback available): {}", e);
        }
    }
}
```

## Key Design Decisions

### 1. Log Index as Universal Cursor

The Raft log index is the cursor. This provides:

- Global ordering across all topics
- Resume from any point in history
- Correlation with KV operations (same log)

### 2. Topics as Key Prefixes

Topics map to keys with a reserved prefix:

```
__pubsub/events/{topic_segments...}/{log_index}
```

This enables:

- Prefix filtering in existing LogSubscriber
- Range scans for topic queries
- Integration with existing KV operations

### 3. Retain Event Data or Just Pointers?

Two options:

**Option A: Store event payload inline**

```
__pubsub/events/orders/created/12345 -> {payload, headers, timestamp}
```

- Simple, self-contained
- Larger log entries
- Good for small events (<100KB)

**Option B: Store payload in blobs, pointer in log**

```
__pubsub/events/orders/created/12345 -> {blob_hash, headers, timestamp}
__pubsub/blobs/{hash} -> actual payload (via iroh-blobs)
```

- Efficient for large payloads
- Content-addressed deduplication
- More complex client logic

**Recommendation**: Start with Option A, add blob offloading for >100KB payloads.

### 4. Wildcard Matching

Implement NATS-style wildcards:

- `*` matches exactly one segment: `orders.*` matches `orders.created`, not `orders.us.created`
- `>` matches zero or more segments: `orders.>` matches `orders`, `orders.created`, `orders.us.created`

Wildcard matching happens client-side after prefix filtering:

```rust
// Server sends all entries with prefix "orders/"
// Client filters: "orders.*" pattern against "orders/us/created" -> no match
```

### 5. Consumer Groups via Raft

Consumer group state stored in KV:

```
__pubsub/groups/{group}/cursor      -> u64 (committed cursor)
__pubsub/groups/{group}/members/{node} -> {last_heartbeat, assigned_partitions}
```

Partition assignment uses consistent hashing:

```rust
fn is_assigned(&self, event: &Event) -> bool {
    let partition = hash(&event.topic) % self.total_partitions;
    self.assigned_partitions.contains(&partition)
}
```

## File Structure

```
crates/aspen-pubsub/
    src/
        lib.rs              # Pub re-exports
        topic.rs            # Topic and TopicPattern types
        publisher.rs        # Publisher trait and RaftPublisher impl
        subscriber.rs       # Subscriber trait and LogSubscriber impl
        consumer_group.rs   # Consumer group coordination
        event.rs            # Event type
        cursor.rs           # Cursor type
        error.rs            # PubSubError
        constants.rs        # Tiger Style limits
        tests.rs            # Unit tests
    Cargo.toml
```

## Tiger Style Compliance

- `MAX_TOPIC_SEGMENTS = 16`
- `MAX_SEGMENT_LENGTH = 256`
- `MAX_PAYLOAD_SIZE = 1MB` (larger uses blob offloading)
- `MAX_HEADERS = 32`
- `MAX_HEADER_VALUE_SIZE = 4KB`
- `MAX_CONSUMER_GROUPS = 1000`
- `MAX_CONSUMERS_PER_GROUP = 100`
- `CONSUMER_HEARTBEAT_INTERVAL = 10s`
- `CONSUMER_EXPIRY_TIMEOUT = 30s`

## Testing Strategy

1. **Unit tests**: Topic parsing, wildcard matching, cursor arithmetic
2. **Integration tests**: Single-node publish/subscribe cycle
3. **Madsim simulation**: Multi-node consumer groups, partition rebalancing
4. **Property tests**: Ordering guarantees, no message loss under failures
5. **Chaos tests**: Node failures during publish, consumer group rebalancing

## Future Enhancements

1. **Dead Letter Topics**: Failed messages routed to DLT after max retries
2. **Message Filtering**: Server-side content filtering (SQL predicates)
3. **Priority Queues**: High-priority events processed first
4. **Schema Registry**: Event schema validation and evolution
5. **Metrics**: Per-topic throughput, consumer lag, partition balance

## Conclusion

Aspen's existing infrastructure provides a solid foundation for pub/sub:

- Raft log = ordered event stream with linearizable guarantees
- Log Subscriber = streaming protocol with prefix filtering and replay
- iroh-gossip = efficient fan-out for high-subscriber topics
- Layer primitives = organized key encoding

The pub/sub layer adds:

- Topic/pattern abstraction
- Consumer groups for competing consumers
- Wildcard matching
- Ergonomic API

This approach eliminates the need for NATS/Kafka while providing stronger consistency guarantees.
