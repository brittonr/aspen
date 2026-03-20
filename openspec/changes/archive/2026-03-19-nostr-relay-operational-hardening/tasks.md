## 1. Atomic event count via CAS

- [x] 1.1 Add `MAX_CAS_RETRIES: u32 = 5` constant to `crates/aspen-nostr-relay/src/constants.rs`
- [x] 1.2 Replace `increment_count` in `storage.rs` with a CAS loop: read current count + version, compute new count, write with expected version, retry on conflict up to MAX_CAS_RETRIES
- [x] 1.3 Replace `decrement_count` with the same CAS loop pattern
- [x] 1.4 Return `StorageError` if CAS retries are exhausted instead of silently proceeding
- [x] 1.5 Unit test: concurrent store_event calls (spawn 10 tasks, each storing one event) — verify final count equals exactly 10
- [x] 1.6 Unit test: CAS retry exhaustion path — mock a KV store that always returns a stale version, verify StorageError is returned

## 2. Timestamp-bounded index scans

- [x] 2.1 Add `filter_scan_prefix_bounded(filter) -> Vec<(String, Option<String>)>` to `filters.rs` that returns (start_prefix, optional end_prefix) pairs incorporating `since`/`until` timestamps
- [x] 2.2 For kind index scans: when `since` is present, start at `nostr:ki:{kind}:{since_padded}:` instead of `nostr:ki:{kind}:`; when `until` is present, stop scanning at `nostr:ki:{kind}:{until_padded}:\xff`
- [x] 2.3 For author index scans: when `since`/`until` are present alongside `authors`, bound the scan range on `nostr:au:{author}:{timestamp}:` the same way
- [x] 2.4 Update `scan_candidates` in `storage.rs` to use bounded scan prefixes — pass start/end bounds to `ScanRequest` if the KV trait supports range scans, otherwise filter during iteration using early termination on sorted keys
- [x] 2.5 Unit test: store 100 events spanning a wide time range, query with `since`/`until` covering 10 events — verify only 10 are returned and the scan touched fewer keys (instrument or count KV reads)
- [x] 2.6 Unit test: query with `since` only, `until` only, and both — verify correct results in each case

## 3. Rate limiting

- [x] 3.1 Add rate limit constants to `constants.rs`: `MAX_EVENTS_PER_SECOND_PER_IP: u32 = 10`, `MAX_EVENTS_BURST_PER_IP: u32 = 20`, `MAX_EVENTS_PER_SECOND_PER_PUBKEY: u32 = 5`, `MAX_EVENTS_BURST_PER_PUBKEY: u32 = 10`, `RATE_LIMIT_BUCKET_TTL_SECS: u64 = 300`, `RATE_LIMIT_CLEANUP_INTERVAL_SECS: u64 = 60`
- [x] 3.2 Add rate limit config fields to `NostrRelayConfig`: `events_per_second_per_ip`, `events_burst_per_ip`, `events_per_second_per_pubkey`, `events_burst_per_pubkey` with defaults from constants
- [x] 3.3 Create `crates/aspen-nostr-relay/src/rate_limit.rs` with `TokenBucket` struct (tokens: f64, last_refill: Instant, rate: f64, burst: f64) and `try_consume() -> bool` method
- [x] 3.4 Create `RateLimiter` struct wrapping two `DashMap`s (ip_buckets, pubkey_buckets) with `check_ip(addr) -> bool` and `check_pubkey(hex) -> bool` methods
- [x] 3.5 Add `RateLimiter::start_cleanup_task()` that spawns a tokio task sweeping stale buckets every RATE_LIMIT_CLEANUP_INTERVAL_SECS
- [x] 3.6 Wire `RateLimiter` into `NostrRelayService` — create during `new()`, pass to connection handler
- [x] 3.7 Add IP rate check in `handle_message` before event processing — reject with `["OK", id, false, "rate-limited: ..."]` if IP bucket is empty
- [x] 3.8 Add pubkey rate check in `handle_event` after signature verification — reject with `["OK", id, false, "rate-limited: ..."]` if pubkey bucket is empty
- [x] 3.9 Skip rate limit checks when both rates are configured as zero
- [x] 3.10 Unit test: token bucket refill over time — verify tokens accumulate up to burst cap
- [x] 3.11 Unit test: submit events beyond burst limit — verify rejection after burst is exhausted
- [x] 3.12 Unit test: two different pubkeys from the same IP — verify independent pubkey buckets
- [x] 3.13 Unit test: stale bucket cleanup — verify buckets are removed after TTL expiry

## 4. NIP-11 HTTP content negotiation

- [x] 4.1 Replace `accept_async(stream)` in `relay.rs` accept loop with a manual HTTP request read — use `tokio::io::AsyncReadExt` to peek at the first bytes
- [x] 4.2 If the request contains `Accept: application/nostr+json`, write the NIP-11 JSON response with `Content-Type: application/nostr+json` headers and close the connection
- [x] 4.3 If the request contains `Upgrade: websocket`, pass the stream and already-read bytes to `tokio-tungstenite` for WebSocket upgrade
- [x] 4.4 If neither, respond with HTTP 400 and close
- [x] 4.5 Integration test: send an HTTP GET with `Accept: application/nostr+json` to the relay port — verify the response is valid NIP-11 JSON with correct fields
- [x] 4.6 Integration test: send a normal WebSocket upgrade request — verify the connection upgrades and NIP-01 messages work
- [x] 4.7 Integration test: send a plain HTTP GET without NIP-11 accept header — verify HTTP 400 response

## 5. Iroh QUIC transport

- [x] 5.1 Add `NOSTR_WS_ALPN: &[u8] = b"/aspen/nostr-ws/1"` to `crates/aspen-transport/src/constants.rs`
- [x] 5.2 Create `crates/aspen-nostr-relay/src/iroh_transport.rs` with a `NostrProtocolHandler` implementing iroh's `ProtocolHandler` trait
- [x] 5.3 Implement length-prefixed frame codec: `write_frame(stream, json_bytes)` writes 4-byte BE length + payload; `read_frame(stream)` reads length then payload, rejects frames > MAX_EVENT_SIZE
- [x] 5.4 Implement the QUIC stream adapter: on accepting a bidirectional stream, wrap it in a `NostrIrohConnection` that translates between length-prefixed frames and the relay's internal message dispatch (reusing `handle_message` from `connection.rs`)
- [x] 5.5 Wire `NostrProtocolHandler` registration in `aspen-cluster`: when `nostr-relay` feature is enabled, register the handler on the iroh endpoint with `NOSTR_WS_ALPN`
- [x] 5.6 Share the event store, subscription registry, and broadcast channel between TCP and iroh transports — both transports SHALL use the same `RelayInner` instance
- [x] 5.7 Integration test: connect via iroh QUIC, send a length-prefixed EVENT, verify OK response
- [x] 5.8 Integration test: publish event via iroh, subscribe via TCP WebSocket — verify the event is received
- [x] 5.9 Integration test: publish event via TCP WebSocket, subscribe via iroh — verify the event is received
- [x] 5.10 Integration test: send oversized frame via iroh — verify stream is closed with error
