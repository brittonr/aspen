## Context

Aspen clusters communicate exclusively over iroh QUIC. External ecosystems (Nostr clients, git frontends, social tools) cannot observe Aspen state without an iroh-connected client. The Nostr protocol (NIP-01) defines a simple relay model: clients connect via WebSocket, subscribe to event filters, and receive signed events in real-time. By embedding a NIP-01 relay inside Aspen, the cluster gains outward visibility without changing its internal transport.

The infrastructure for this already exists in the codebase:

- **iroh-relay** (`relay-server` feature): runs an HTTP server on each node for NAT traversal relay traffic. This server is built on hyper and already listens on a TCP port.
- **iroh-proxy-utils** (`proxy` feature): handles WebSocket upgrades via `UpstreamProxy::handle_upgrade_request`, with `SUPPORTED_UPGRADE_PROTOCOLS = ["websocket"]`.
- **WASM plugin system**: sandboxed plugins with KV read/write, hook subscriptions, timers — the dispatch model for event translation logic.
- **Hook system**: pub/sub event stream with topics for KV writes, cluster events, blob operations — the trigger mechanism for generating Nostr events from Aspen state changes.

## Goals / Non-Goals

**Goals:**

- NIP-01 compliant relay serving events over WebSocket to standard Nostr clients
- Event storage in the existing Raft-backed KV store with indexed queries
- WASM plugin bridge translating Forge events to NIP-34 Nostr events
- Single new host function (`nostr_publish_event`) for plugins to emit events
- Feature-gated, disabled by default, zero overhead when off
- Reuse existing HTTP/WebSocket infrastructure (iroh-relay or standalone listener)

**Non-Goals:**

- Full Nostr social features (zaps, DMs, marketplace, calendars)
- Replacing iroh as the internal transport — Nostr relay is edge-only
- Write path from Nostr clients into Aspen (accepting external patches, issues) — future work
- Running a general-purpose public Nostr relay — this serves Aspen-generated events
- NIP-42 relay authentication — defer to a later phase
- Nostr user identity as Aspen's auth primitive — separate concern explored independently

## Decisions

### 1. WebSocket transport: standalone TCP listener, not embedded in iroh-relay

**Decision**: Run a dedicated `tokio::net::TcpListener` on a configurable port (default 4869) for Nostr WebSocket connections. Do not embed into the iroh-relay HTTP server.

**Rationale**: The iroh-relay server's `ServerConfig` is structured around the relay protocol. Injecting Nostr-specific WebSocket handling requires forking or patching the relay server's hyper service chain. A standalone listener is isolated, independently configurable (bind address, TLS, rate limits), and can be enabled without the `relay-server` feature.

**Alternatives considered**:

- Embed in iroh-relay HTTP server: tighter integration but couples Nostr to relay lifecycle and config. The relay server's access control (cluster-members-only) conflicts with Nostr's open-access model.
- Use iroh-proxy-utils to tunnel WebSocket over QUIC: adds latency, requires a downstream proxy process, and Nostr clients still need a TCP endpoint to connect to.

### 2. Event storage: KV store with secondary indexes

**Decision**: Store events in the Raft-backed KV store using a structured key scheme with secondary index keys for filter queries.

```
nostr:ev:{event_id}                              → event JSON
nostr:ki:{kind}:{created_at_be}:{event_id}       → ""
nostr:au:{author_hex}:{created_at_be}:{event_id} → ""
nostr:tg:{tag_name}:{tag_value}:{event_id}       → ""
```

`created_at_be` is the creation timestamp as 8-byte big-endian for lexicographic ordering.

**Rationale**: The KV store already provides prefix scans, Raft-replicated durability, and the plugin system has KV access. No new storage backend needed. Index keys are structured so NIP-01 filter queries map directly to prefix scans with set intersection.

**Alternatives considered**:

- SQLite/LMDB sidecar: better query performance for complex filters but adds a storage backend outside Raft consensus, breaking the single-source-of-truth model.
- In-memory only: fast but events lost on restart, no cross-node consistency.

### 3. Plugin integration: single host function, not a new plugin type

**Decision**: Add one host function `nostr_publish_event(event_json: &str) -> Result<event_id, error>` gated by a `nostr_publish: bool` permission. The native relay engine handles storage, indexing, and subscription fan-out.

**Rationale**: Keeps the plugin sandbox unchanged. Plugins construct events as JSON strings and hand them off. The native code handles WebSocket I/O, subscription matching, and storage — things the WASM sandbox cannot do. One host function is the minimum viable surface.

**Alternatives considered**:

- Extend plugin system with network listener capabilities: massive scope increase, changes the plugin security model fundamentally.
- New "protocol plugin" type with ALPN registration: interesting long-term but requires rethinking the plugin dispatch model from request/response to connection-oriented.
- Native-only (no plugin involvement): works but loses the runtime extensibility — can't add new event bridges without recompiling.

### 4. Nostr key: cluster-level secp256k1 keypair

**Decision**: Generate a secp256k1 keypair per cluster for signing bridge-generated Nostr events. Store alongside `ClusterIdentity` in cluster config. Expose the npub in the relay's NIP-11 information document.

**Rationale**: Bridge-generated events (Forge push → NIP-34 repo state) need a signer. The cluster identity is the natural authority. Using a separate secp256k1 key avoids any entanglement with the Ed25519 identity used for iroh transport. The `k256` crate provides pure-Rust secp256k1 without pulling in Bitcoin dependencies.

**Alternatives considered**:

- Derive secp256k1 from Ed25519 cluster key: mathematically possible but fragile and non-standard. If the derivation is ever broken, both identities are compromised.
- Per-node Nostr keys: events from different nodes would appear as different authors for the same cluster, confusing Nostr clients.
- No signing (unsigned events): NIP-01 requires valid signatures. Not an option.

### 5. NIP scope: NIP-01 + NIP-11 + NIP-34 only

**Decision**: Implement NIP-01 (basic protocol), NIP-11 (relay info document), and NIP-34 (git stuff) for the initial release. No other NIPs.

**Rationale**: NIP-01 is mandatory for any relay. NIP-11 is trivial (one JSON endpoint). NIP-34 is the primary value — it makes Forge repos visible to the Nostr git ecosystem. Every additional NIP adds surface area with diminishing returns for Aspen's use case.

### 6. Subscription fan-out: in-process broadcast channel

**Decision**: Use `tokio::sync::broadcast` for real-time event distribution to connected WebSocket clients. When a new event arrives (from plugin or direct write), broadcast it. Each WebSocket connection task receives from the broadcast and checks its active filters.

**Rationale**: Simple, no external dependencies, bounded memory (broadcast channel has a configurable capacity). The relay serves Aspen-generated events, not a firehose of global traffic — the event rate is low enough that broadcast with per-connection filter checking is sufficient.

**Tiger Style bounds**:

- `MAX_NOSTR_CONNECTIONS`: 256
- `MAX_SUBSCRIPTIONS_PER_CONNECTION`: 16
- `MAX_FILTERS_PER_SUBSCRIPTION`: 8
- `MAX_EVENT_SIZE`: 64 KB
- `BROADCAST_CHANNEL_CAPACITY`: 4096
- `MAX_STORED_EVENTS`: 100,000

## Risks / Trade-offs

**[WebSocket adds a non-iroh listener]** → The Nostr relay is explicitly an edge adapter. It does not participate in consensus, does not carry internal traffic, and is feature-gated off by default. The WebSocket port is read-only and stateless from the relay's perspective — events flow out, not in (initially).

**[KV-based event storage may be slow for complex filter queries]** → NIP-01 filters with multiple conditions require intersecting results from multiple index scans. For the expected event volume (thousands, not millions), this is acceptable. If it becomes a bottleneck, a secondary SQLite index can be added behind the same trait without changing the relay or plugin API.

**[secp256k1 dependency for Nostr signing]** → The `k256` crate is pure Rust, well-audited (RustCrypto project), and adds ~50KB to binary size. It does not pull in Bitcoin-specific dependencies. The `nostr` crate (for event type definitions) is lightweight when used without `nostr-sdk`.

**[WASM plugin must construct valid NIP-01 event JSON]** → If a plugin emits malformed events, the native relay validates before storing/broadcasting. Invalid events are rejected with an error returned to the plugin. The relay never serves unvalidated events.

**[Relay is read-only initially — no write path from Nostr]** → This is intentional. Accepting external events (e.g., NIP-34 patches from Nostr clients → Forge) requires authorization, input validation, and a mapping layer that doesn't exist yet. The read-only bridge proves value before investing in the write path.
