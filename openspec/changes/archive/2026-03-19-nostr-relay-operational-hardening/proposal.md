## Why

The Nostr relay has correctness and architecture gaps that will bite under real load. The event counter does read-then-write without CAS, so concurrent publishes drift the count and eventually break eviction. Storage scans ignore `since`/`until` timestamps already encoded in index keys, loading far more data than necessary. There's no per-IP or per-pubkey rate limiting, so a single client can flood the relay. The relay runs on raw TCP WebSocket instead of iroh QUIC, violating Aspen's "Iroh for ALL communication" rule and sitting outside the cluster's NAT traversal, authentication, and encryption. And the NIP-11 relay info document is never actually served — `run()` calls `accept_async()` without checking the HTTP `Accept` header first.

## What Changes

- Replace the event count's read-then-write with a compare-and-swap loop so concurrent publishes produce an accurate count
- Add timestamp-bounded KV scans: use `since`/`until` to narrow the scan prefix range instead of loading all candidates and filtering in-memory
- Add per-IP and per-pubkey rate limiting with token-bucket counters, configurable limits, and clean rejection messages
- Add an iroh QUIC transport path: register a `NOSTR_WS_ALPN` protocol handler that accepts iroh connections and bridges them into the existing WebSocket handler, mirroring the pattern used by `FORGE_WEB_ALPN` and `NIX_CACHE_H3_ALPN`
- Add HTTP content negotiation before WebSocket upgrade: inspect the `Accept` header and serve the NIP-11 JSON document for `application/nostr+json` requests, only upgrading to WebSocket otherwise

## Capabilities

### New Capabilities

- `nostr-relay-rate-limiting`: Per-IP and per-pubkey token-bucket rate limiting for EVENT submissions, with configurable burst and sustained rates
- `nostr-relay-iroh-transport`: Iroh QUIC transport for Nostr WebSocket connections via ALPN-based protocol routing, bringing the relay into the cluster's networking layer

### Modified Capabilities

- `nostr-relay-engine`: Event count atomicity via CAS, timestamp-bounded index scans, NIP-11 HTTP content negotiation

## Impact

- `crates/aspen-nostr-relay/src/storage.rs` — CAS loop for count, timestamp-bounded scan helpers
- `crates/aspen-nostr-relay/src/relay.rs` — rate limiter integration, NIP-11 content negotiation before WebSocket upgrade
- `crates/aspen-nostr-relay/src/connection.rs` — rate limit checks on EVENT handler
- `crates/aspen-nostr-relay/src/constants.rs` — new constants for rate limits
- `crates/aspen-nostr-relay/src/config.rs` — rate limit config fields
- `crates/aspen-nostr-relay/src/filters.rs` — timestamp-bounded prefix generation
- `crates/aspen-transport/src/constants.rs` — `NOSTR_WS_ALPN` constant
- `crates/aspen-cluster/` — wire iroh protocol handler for Nostr relay
- New: `crates/aspen-nostr-relay/src/rate_limit.rs`
