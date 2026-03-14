## 1. Crate Scaffolding and Dependencies

- [x] 1.1 Create `crates/aspen-nostr-relay/` with Cargo.toml — deps: `nostr` (core types), `k256` (secp256k1), `tokio`, `tokio-tungstenite`, `aspen-core`, `serde`, `serde_json`, `tracing`
- [x] 1.2 Add `nostr-relay` feature flag to top-level Cargo.toml gating `dep:aspen-nostr-relay` and `hooks`
- [x] 1.3 Add `nostr-relay` to the `full` feature set
- [x] 1.4 Create `NostrRelayConfig` struct with fields: `enabled`, `bind_addr`, `bind_port`, `max_connections`, `max_subscriptions_per_connection`, `max_event_size` with Tiger Style defaults as constants

## 2. Nostr Key Management

- [x] 2.1 Add `NostrIdentity` type wrapping a `k256::ecdsa::SigningKey` / `k256::schnorr` keypair in `crates/aspen-nostr-relay/src/keys.rs`
- [x] 2.2 Implement `NostrIdentity::generate()`, `from_secret_bytes()`, `secret_bytes()`, `public_key_hex()` for persistence
- [x] 2.3 Implement NIP-01 event signing: compute event ID (SHA-256 of serialized array), sign with Schnorr per BIP-340, set `id`/`pubkey`/`sig` fields
- [x] 2.4 Add Nostr key storage to cluster config — optional `nostr_secret_key_hex` field alongside Ed25519 cluster key, loaded/generated on startup when `nostr-relay` is enabled
- [x] 2.5 Write tests: key generation, roundtrip persistence, event signing, signature verification with `nostr` crate's verifier

## 3. Event Storage

- [x] 3.1 Create `crates/aspen-nostr-relay/src/storage.rs` with `NostrEventStore` trait and KV-backed implementation
- [x] 3.2 Implement `store_event()`: write event JSON to `nostr:ev:{id}`, write index keys for kind, author, tags, created_at with big-endian timestamp for lexicographic ordering
- [x] 3.3 Implement `query_events(filters)`: for each NIP-01 filter, scan relevant indexes, intersect results for AND conditions, union across filters for OR
- [x] 3.4 Implement `get_event(id)`: direct KV lookup by event ID
- [x] 3.5 Implement replaceable event logic: for kinds 0, 3, 10000-19999, keep only latest per (pubkey, kind); for kinds 30000-39999, keep only latest per (pubkey, kind, d-tag)
- [x] 3.6 Implement event eviction: when stored count exceeds MAX_STORED_EVENTS, delete oldest events by `created_at`
- [x] 3.7 Write tests: store/retrieve, filter queries (by kind, author, tag, time range, limit), replaceable event dedup, eviction

## 4. NIP-01 Protocol Handler

- [x] 4.1 Create `crates/aspen-nostr-relay/src/connection.rs` — WebSocket connection handler that reads NIP-01 JSON messages and dispatches to EVENT/REQ/CLOSE handlers
- [x] 4.2 Implement EVENT handler: validate event (JSON structure, ID hash, Schnorr signature), store via `NostrEventStore`, broadcast to subscriptions, respond with OK
- [x] 4.3 Implement REQ handler: parse filters, query stored events, send matching events as `["EVENT", sub_id, event]`, send `["EOSE", sub_id]`, register subscription for real-time push
- [x] 4.4 Implement CLOSE handler: remove subscription by ID, release resources
- [x] 4.5 Implement filter matching logic in `crates/aspen-nostr-relay/src/filters.rs` — match event against NIP-01 filter (ids, authors, kinds, tags, since, until)
- [x] 4.6 Write tests: EVENT accept/reject, REQ with stored events + EOSE, CLOSE removes subscription, filter matching (AND within filter, OR across filters)

## 5. Subscription and Fan-out

- [x] 5.1 Create `crates/aspen-nostr-relay/src/subscriptions.rs` with `SubscriptionRegistry` — maps (connection_id, sub_id) to active filter sets
- [x] 5.2 Implement subscription add/remove/replace with per-connection limit enforcement (MAX_SUBSCRIPTIONS_PER_CONNECTION)
- [x] 5.3 Implement broadcast fan-out: `tokio::sync::broadcast` channel, each connection task receives events and checks against its active filters
- [x] 5.4 Write tests: subscribe/unsubscribe, replacement, limit enforcement, fan-out delivers to matching subscriptions only

## 6. WebSocket Listener and Relay Service

- [x] 6.1 Create `crates/aspen-nostr-relay/src/relay.rs` with `NostrRelayService` — owns the TCP listener, event store, subscription registry, broadcast channel, and Nostr identity
- [x] 6.2 Implement `NostrRelayService::run()`: bind TCP listener, accept connections, spawn per-connection tasks with WebSocket upgrade via `tokio-tungstenite`
- [x] 6.3 Implement NIP-11: respond with relay info JSON when HTTP GET has `Accept: application/nostr+json` header — include name, description, pubkey, supported_nips, software
- [x] 6.4 Implement connection limit enforcement: track active count with `AtomicU32`, reject when at MAX_NOSTR_CONNECTIONS
- [x] 6.5 Implement graceful shutdown: cancel token propagated to all connection tasks, drain active connections on node shutdown
- [x] 6.6 Write integration test: start relay, connect via WebSocket, publish event, subscribe, receive event

## 7. Plugin Host Function

- [x] 7.1 Add `nostr_publish: bool` field to `PluginPermissions` in `crates/aspen-plugin-api/src/manifest.rs`, defaulting to `false`
- [ ] 7.2 Define `nostr_publish_event` host function signature in plugin API
- [ ] 7.3 Implement host-side `nostr_publish_event`: validate event JSON, verify signature, pass to `NostrRelayService::publish()`, return event ID or error
- [ ] 7.4 Wire permission check: reject calls when `nostr_publish` is `false` or relay feature is not active
- [ ] 7.5 Write tests: permission denied without `nostr_publish`, successful publish flows through to relay, invalid event rejected at host boundary

## 8. Node Integration

- [x] 8.1 Wire `NostrRelayService` startup into `aspen-node` main — create service from config, start listener, store handle for shutdown
- [x] 8.2 Wire Nostr key loading/generation into node bootstrap — load from config or generate on first run, pass to relay service
- [x] 8.3 Add `[nostr_relay]` config section parsing to node config
- [x] 8.4 Wire graceful shutdown of relay service into node shutdown sequence
- [x] 8.5 Feature-gate all integration code behind `#[cfg(feature = "nostr-relay")]`

## 9. Forge Bridge Plugin

- [ ] 9.1 Create `plugins/nostr-forge-bridge/` WASM plugin project with manifest declaring `hooks` and `nostr_publish` permissions, KV read access for `forge:` prefix
- [ ] 9.2 Implement hook handler for repo creation: read `RepoIdentity` from KV, build `kind:30617` event with `d`, `name`, `description`, `clone`, `maintainers` tags, sign with cluster key, call `nostr_publish_event`
- [ ] 9.3 Implement hook handler for ref updates: read updated refs, build `kind:30618` event with `d` and `refs/*` tags, sign, publish
- [ ] 9.4 Implement hook handler for issue creation: read issue data, build `kind:1621` event with `a`, `p`, `subject`, `t` tags, sign, publish
- [ ] 9.5 Implement hook handler for patch submission: read patch data, build `kind:1617` event with patch content, `a`, `p`, `commit` tags, sign, publish
- [ ] 9.6 Write tests: mock hook events → verify correct NIP-34 event structure and tags for each event type

## 10. End-to-End Testing

- [ ] 10.1 Integration test: start aspen-node with `nostr-relay` enabled, connect Nostr client, verify NIP-11 relay info
- [ ] 10.2 Integration test: publish event via WebSocket client, query it back via REQ subscription
- [ ] 10.3 Integration test: install forge-bridge plugin, create Forge repo, verify `kind:30617` event appears on relay
- [ ] 10.4 Integration test: push to Forge repo, verify `kind:30618` state event appears with correct ref tags
- [ ] 10.5 Test resource bounds: exceed connection limit, subscription limit, event size limit — verify clean rejection
