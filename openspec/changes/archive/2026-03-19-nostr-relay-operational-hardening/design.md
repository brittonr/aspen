## Context

The Nostr relay (`aspen-nostr-relay`) is functional — NIP-01, NIP-11, NIP-42, KV-backed storage, subscription fan-out, 42+ passing tests. But it has gaps that will surface under real usage: a non-atomic event counter, unoptimized time-range scans, zero rate limiting, no iroh transport integration, and a missing NIP-11 content negotiation path.

The relay currently binds a raw TCP socket and runs `tokio-tungstenite` directly. Every other Aspen service (Raft, Client RPC, Forge web, Nix cache, DAG sync, net tunnels) communicates over iroh QUIC with ALPN-based routing. The Nostr relay is the only component that bypasses this.

## Goals / Non-Goals

**Goals:**

- Atomic event count that stays accurate under concurrent writes
- Timestamp-bounded KV scans that skip irrelevant data at the storage layer
- Per-IP and per-pubkey rate limiting with token-bucket semantics
- Iroh QUIC transport for Nostr connections alongside the existing TCP listener
- NIP-11 relay info served via HTTP content negotiation before WebSocket upgrade

**Non-Goals:**

- Replacing the TCP WebSocket listener (keep it for external Nostr clients that don't speak iroh)
- Adding new NIP support (NIP-09 deletion, NIP-45 COUNT, NIP-50 search — those are separate work)
- Changing the event storage schema or index key format
- Cross-relay federation or external relay connectivity

## Decisions

**Event count uses a CAS retry loop, not a separate atomic counter.**

The current `increment_count`/`decrement_count` do `read(count) → write(count+1)`, which races under concurrent publishes. The fix: use the KV store's compare-and-swap operation. Read the current count and its version, compute the new value, write with the expected version. If the CAS fails (another writer updated it), re-read and retry. Cap retries at 5 to bound latency.

Alternative: track count in-memory with `AtomicU32` and reconcile periodically. Rejected — the count gates eviction, so an inaccurate in-memory counter could allow unbounded storage growth or premature eviction after a restart.

**Timestamp-bounded scans narrow the KV prefix range using `since`/`until`.**

Kind and author index keys already embed a zero-padded 16-digit timestamp: `nostr:ki:{kind}:{timestamp}:{event_id}`. A filter with `since: 1700000000` can start the scan at `nostr:ki:{kind}:0001700000000` instead of the kind prefix root. Similarly, `until` bounds the scan endpoint. This is a prefix trick — no schema change, just smarter scan bounds.

For author indexes (`nostr:au:{author}:{timestamp}:{event_id}`), the same approach works when the filter specifies both an author and a time range.

Tag indexes don't include a timestamp component and can't benefit from this optimization. They continue with a full prefix scan followed by in-memory timestamp filtering.

**Rate limiting uses a per-key token bucket stored in a `DashMap`.**

Two independent rate limit dimensions:

- Per-IP: limits total EVENT submissions from a source address
- Per-pubkey: limits EVENT submissions from a specific Nostr identity

Each bucket tracks tokens (f64), last refill timestamp, and config (burst capacity, sustained rate). On each EVENT, deduct a token from both the IP and pubkey buckets. If either is empty, reject with `["OK", <id>, false, "rate-limited: ..."]`. Stale buckets are evicted on a periodic sweep (every 60s, remove buckets untouched for 5 minutes).

Alternative: leaky bucket. Token bucket was chosen because it allows short bursts (useful for git push → NIP-34 bridge publishing a batch of events) while still enforcing a sustained rate.

Alternative: rate limit at the KV layer. Rejected — rate limiting should happen before event validation and storage to minimize wasted work.

**Iroh transport adds a `NOSTR_WS_ALPN` protocol handler that bridges to the WebSocket handler.**

Register `b"/aspen/nostr-ws/1"` as an ALPN on the iroh endpoint. When an iroh connection arrives on this ALPN, open a bidirectional QUIC stream and adapt it into the same `WebSocketStream<TcpStream>` interface the connection handler expects. This follows the pattern established by `FORGE_WEB_ALPN` (HTTP/3 over iroh for Forge web) and `NIX_CACHE_H3_ALPN` (HTTP/3 over iroh for Nix cache).

The adaptation layer reads/writes WebSocket frames over the QUIC stream. The framing is simple: length-prefixed JSON messages (4-byte big-endian length + UTF-8 payload), since QUIC already provides reliable ordered delivery and encryption — no need for the WebSocket framing overhead.

The TCP listener stays for external Nostr clients. The iroh transport adds an internal path for cluster-to-relay and CLI-to-relay communication that benefits from iroh's NAT traversal and peer authentication.

**NIP-11 content negotiation reads the first HTTP request before deciding to upgrade.**

Replace the direct `accept_async(stream)` with a manual HTTP request parse. Read the first request from the TCP stream. If the `Accept` header contains `application/nostr+json`, respond with the NIP-11 JSON document and close. Otherwise, complete the WebSocket upgrade handshake using `tokio-tungstenite`'s `accept_hdr_async` or by passing the already-read bytes through.

This is a small change to `relay.rs`'s accept loop. The `relay_info_json()` method already produces the correct NIP-11 document.

## Risks / Trade-offs

**[CAS retry loop adds latency under contention]** → With Aspen's event rate (Forge-generated events, not a global firehose), contention on the counter is low. 5 retries with the Raft round-trip (~2-3ms each) means worst case ~15ms added latency. If this becomes a bottleneck, batch count updates.

**[Rate limit state is per-node, not cluster-wide]** → A client could distribute writes across nodes to bypass per-IP limits. Acceptable for now — cluster nodes are behind iroh, not directly addressable. External clients hit a single TCP listener. Cluster-wide rate limiting would require Raft consensus for every rate check, which defeats the purpose.

**[QUIC stream adaptation is not true WebSocket]** → Internal iroh clients use length-prefixed JSON, not WebSocket framing. This means standard Nostr client libraries won't work directly over iroh — only Aspen's own client code. This is by design: external clients use the TCP WebSocket, internal clients use iroh.

**[DashMap rate limit state lost on restart]** → Acceptable. Rate limits are ephemeral; after a restart, clients get a fresh burst allowance. The alternative (persisting rate state in KV) adds Raft round-trips to every EVENT.
