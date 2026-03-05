## 1. Cursor sentinel and RouterBuilder escape hatch

- [x] 1.1 Add `Cursor::EPHEMERAL` constant (`u64::MAX - 1`) and `is_ephemeral()` method to `crates/aspen-hooks/src/pubsub/cursor.rs`
- [x] 1.2 Write unit tests: `Cursor::EPHEMERAL.is_ephemeral()` returns true, `Cursor::from_index(42).is_ephemeral()` returns false, `Cursor::EPHEMERAL` does not collide with `Cursor::BEGINNING` or `Cursor::LATEST`
- [x] 1.3 Add `custom<P: iroh::protocol::ProtocolHandler>(mut self, alpn: &[u8], handler: P) -> Self` method to `RouterBuilder` in `crates/aspen-cluster/src/router_builder.rs`. Same pattern as existing methods: `self.builder.accept(alpn, handler)`, log with `tracing::info!`, return `self`

## 2. EphemeralBroker

- [x] 2.1 Create `crates/aspen-hooks/src/pubsub/ephemeral/mod.rs` with `pub mod broker; pub mod publisher;` and re-exports
- [x] 2.2 Add `pub mod ephemeral;` to `crates/aspen-hooks/src/pubsub/mod.rs` and re-export `EphemeralBroker` and `EphemeralPublisher`
- [x] 2.3 Create `crates/aspen-hooks/src/pubsub/ephemeral/broker.rs` with `EphemeralBroker` struct: `RwLock<Vec<ActiveSubscription>>` + `AtomicU64` for ID generation. `ActiveSubscription` holds `id: u64`, `pattern: TopicPattern`, `sender: mpsc::Sender<Event>`
- [x] 2.4 Implement `EphemeralBroker::new()`, `subscribe(pattern: TopicPattern, buffer_size: usize) -> (u64, mpsc::Receiver<Event>)`, `unsubscribe(id: u64)`, `publish(topic: &Topic, event: Event)`. Publish takes read lock, iterates subscriptions, uses `try_send` (drops on full buffer). Subscribe/unsubscribe take write lock
- [x] 2.5 Add Tiger Style constants to `crates/aspen-hooks/src/pubsub/ephemeral/mod.rs` or `broker.rs`: `MAX_EPHEMERAL_SUBSCRIPTIONS` (1024), `DEFAULT_EPHEMERAL_BUFFER_SIZE` (256)
- [x] 2.6 Write unit tests: publish with no subscribers (no panic), subscribe + publish matching topic (event received), subscribe + publish non-matching topic (no event), wildcard `*` and `>` patterns route correctly, full buffer drops event without blocking, unsubscribe stops delivery, multiple subscribers on same topic all receive

## 3. EphemeralPublisher

- [x] 3.1 Create `crates/aspen-hooks/src/pubsub/ephemeral/publisher.rs` with `EphemeralPublisher` wrapping `Arc<EphemeralBroker>`. Implement `Publisher` trait: `publish()` calls `broker.publish()` and returns `Ok(Cursor::EPHEMERAL)`, `publish_batch()` iterates and publishes individually
- [x] 3.2 Write unit tests: `publish()` returns `Cursor::EPHEMERAL`, `publish_batch()` delivers all events to subscriber, empty batch returns `Cursor::BEGINNING`

## 4. Wire types and ALPN constant

NOTE: Moved to aspen-hooks (not aspen-transport) to avoid circular dependency — handler needs EphemeralBroker from hooks, and hooks already depends on transport.

- [x] 4.1 Create `crates/aspen-hooks/src/pubsub/ephemeral/wire.rs` with postcard-serializable types: `EphemeralSubscribeRequest`, `EphemeralWireEvent`. Length-prefixed read/write helpers: `read_frame`, `write_frame` (4-byte BE + postcard)
- [x] 4.2 Add `pub mod wire; pub mod handler;` to ephemeral/mod.rs, re-export `EPHEMERAL_ALPN` and `EphemeralProtocolHandler` from pubsub/mod.rs
- [x] 4.3 Wire types include topic, timestamp_ms, payload, headers. Subscribe request includes pattern and buffer_size
- [x] 4.4 Unit tests: postcard round-trip for both types, length-prefix encoding verification

## 5. EphemeralProtocolHandler

- [x] 5.1 Create `crates/aspen-hooks/src/pubsub/ephemeral/handler.rs` with `EphemeralProtocolHandler` holding `Arc<EphemeralBroker>` and `Arc<Semaphore>`
- [x] 5.2 Implement `iroh::protocol::ProtocolHandler`: accept connection, accept bi-stream, read subscribe request, validate pattern, register with broker, stream events via write_frame, unsubscribe on disconnect
- [x] 5.3 Implement `shutdown()` to close the connection semaphore
- [x] 5.4 `MAX_EPHEMERAL_CONNECTIONS: u32 = 200` in handler.rs, `EPHEMERAL_ALPN` constant in handler.rs

## 6. Integration test

- [x] 6.1 Write integration test in `crates/aspen-hooks/tests/ephemeral_pubsub_integration.rs`: two tests — end-to-end (subscribe, publish, receive, non-matching filtered, disconnect cleanup) and multiple subscribers (two clients, both receive, one disconnects, other still works)

## 7. Bootstrap wiring

- [x] 7.1 Added `ephemeral_broker: Option<Arc<EphemeralBroker>>` field to `Node` struct in `src/node/mod.rs`. Broker is created during `spawn_router()` and `spawn_router_with_blobs()`, stored on Node, and exposed via `ephemeral_broker()` accessor
- [x] 7.2 Registered `EphemeralProtocolHandler` with `EPHEMERAL_ALPN` in both `spawn_router()` and `spawn_router_with_blobs()` in `src/node/mod.rs`
- [x] 7.3 `cargo check --features "jobs,docs,blob,hooks,automerge,federation"` compiles clean
- [x] 7.4 `cargo nextest run -p aspen-hooks -E 'test(/ephemeral/)'` — 21/21 tests pass. Full suite: 136/136 pass, zero regressions
