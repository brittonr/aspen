## Context

The relay (`aspen-nostr-relay`) currently accepts EVENT from any WebSocket client after signature verification. There's no concept of "who connected" — the relay validates the event's cryptographic signature but doesn't know or care about the connection's identity. The `handle_message` function handles `ClientMessage::Auth` with a `"unsupported message type"` notice.

The Forge already has a challenge-response auth system (`NostrAuthService`) that works over the Aspen RPC layer. NIP-42 is a different protocol that operates on the WebSocket connection itself — it's the Nostr-native way for relays to authenticate clients.

The `nostr` crate v0.44 ships `ClientMessage::Auth(Box<Event>)` and `RelayMessage::Auth { challenge }` ready to use.

## Goals

- Implement NIP-42 per the spec: relay sends `["AUTH", <challenge>]`, client responds with a signed kind 22242 event
- Gate EVENT writes behind authentication when configured
- Keep reads (REQ, CLOSE) open regardless of auth state
- Bridge/plugin writes via `NostrRelayService::publish()` bypass NIP-42 — they're internal, already trusted
- Configurable policy so operators choose their security posture

## Non-Goals

- NIP-42 restricted REQ (filtering reads by auth) — reads stay open
- Integrating NIP-42 auth with the Forge `NostrAuthService` or capability tokens — they're different layers (WebSocket vs RPC)
- Rate limiting or reputation scoring based on npub — separate concern
- NIP-65 relay list management
- Paid relay features (NIP-11 `limitation.payment_required`)

## Decisions

### 1. Per-connection auth state in the connection handler

**Decision**: Add an `auth_state: Option<PublicKey>` field to `handle_connection`'s local state (not the `SubscriptionRegistry`). When a client completes NIP-42, the handler stores their verified npub. The `handle_event` function checks this before accepting writes.

**Rationale**: Auth state is connection-scoped — it doesn't need to survive reconnection and doesn't belong in the registry (which tracks subscriptions, not identity). Keeping it local to the connection task avoids shared mutable state and lock contention.

**Rejected**: Storing auth state in `SubscriptionRegistry` — adds complexity, requires cleanup, and couples identity to subscription management.

### 2. Challenge generation: random hex string

**Decision**: Generate a 32-byte random challenge on connection, send it as `["AUTH", <hex>]`. The challenge is stored in the connection handler's local scope and verified when the client responds.

**Rationale**: NIP-42 only requires the challenge to be a unique string. 32 random bytes (64 hex chars) is the same entropy the Forge auth uses and prevents replay attacks. No need for a registry of pending challenges since the connection handler owns its own challenge.

**Rejected**: Structured challenges (timestamps, relay URL encoded) — NIP-42 puts the relay URL in the kind 22242 event, not the challenge itself. The challenge just needs to be unique.

### 3. Kind 22242 event verification

**Decision**: When the client sends `ClientMessage::Auth(event)`, verify:

1. `event.kind == 22242`
2. `event.tags` contains a `relay` tag matching the relay's URL
3. `event.tags` contains a `challenge` tag matching the sent challenge
4. `event.created_at` is within ±60 seconds of now (replay window)
5. `event.verify()` passes (signature is valid)

On success, set `auth_state = Some(event.pubkey)`.

**Rationale**: This follows the NIP-42 spec exactly. The relay URL check prevents challenge reuse across relays. The timestamp window prevents old signed events from being replayed.

### 4. Write policy enum

**Decision**: Add a `WritePolicy` enum to config:

- `Open` — current behavior, any valid signed event accepted (default for backward compatibility)
- `AuthRequired` — must complete NIP-42 before EVENT is accepted
- `ReadOnly` — no external writes, only `publish()` API works

The connection handler receives the policy and checks it in `handle_event`.

**Rationale**: Three modes cover all deployment scenarios. `Open` preserves existing behavior. `AuthRequired` is what operators want for spam protection. `ReadOnly` is what the relay already functionally is (bridge writes only) but made explicit.

**Rejected**: Per-kind policies (e.g., "require auth for kind 1 but not kind 0") — over-engineering. If needed later, it's additive.

### 5. Relay URL in config

**Decision**: Add an optional `relay_url: Option<String>` field to `NostrRelayConfig`. Used in the NIP-42 challenge verification (the kind 22242 event must contain a `relay` tag matching this URL). If not set, the relay URL check is skipped.

**Rationale**: The relay doesn't know its own public URL (it binds `127.0.0.1:4869` behind a proxy). Making it configurable is the only clean option. Skipping the check when unset is pragmatic for development.

### 6. Send AUTH challenge on connect

**Decision**: Send the `["AUTH", <challenge>]` message immediately after WebSocket handshake, before the client sends anything.

**Rationale**: NIP-42 says the relay MAY send AUTH at any time, but sending on connect is simplest — the client knows auth is available and can respond before or after REQ. Sending lazily (only when a restricted event is rejected) adds round-trip latency and client-side retry logic.

## Risks / Trade-offs

**[Backward compatibility with non-NIP-42 clients]** → Clients that don't understand AUTH will ignore the `["AUTH", ...]` message (unknown message types are ignored per NIP-01). Reads work without auth. Writes only break if `write_policy = AuthRequired`, which is opt-in. Default is `Open`.

**[Challenge replay across restarts]** → Challenges are ephemeral (in-memory per connection). A restart clears all challenges. A client holding a signed kind 22242 event from a previous connection can't replay it because the challenge won't match. No risk.

**[Clock skew on created_at validation]** → The ±60s window is generous. Nodes with wildly wrong clocks could reject valid auth events. Mitigation: the 60s window is a constant in `constants.rs`, tunable if needed.

**[Relay URL unknown behind proxy]** → If `relay_url` isn't configured, the relay URL tag check is skipped. This weakens cross-relay replay protection. Acceptable for development; operators running public relays should set it.
