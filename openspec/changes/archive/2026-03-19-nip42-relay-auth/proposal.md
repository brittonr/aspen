## Why

The Nostr relay accepts EVENT writes from any WebSocket client without verifying identity. Before opening a write path from external Nostr clients into Forge (NIP-34 patches, issues), the relay needs a gating mechanism. NIP-42 is the standard protocol for relay-to-client authentication — the relay sends a challenge, the client signs it with their nsec, and the relay verifies before allowing writes. Without this, the relay is open to spam and unauthorized event injection.

## What Changes

- Implement NIP-42 authentication in the relay's WebSocket connection handler. On connect, the relay sends an `AUTH` challenge. The client responds with a signed kind 22242 event proving npub ownership.
- Add per-connection auth state tracking to the subscription registry and connection handler. Each connection knows whether it's authenticated and which npub it proved.
- Add a configurable write policy to `NostrRelayConfig`: `open` (current behavior, anyone can write), `auth-required` (must complete NIP-42 before EVENT is accepted), or `read-only` (no external writes at all, only bridge/publish API).
- Reject EVENT submissions from unauthenticated connections when policy is `auth-required`, responding with `OK false "auth-required: ..."` per NIP-42 spec.
- Update `supported_nips` in the NIP-11 relay info document to include 42.
- Reads (REQ/CLOSE) remain open regardless of auth policy — authentication gates writes, not reads.

## Capabilities

### New Capabilities

- `nip42-auth`: NIP-42 challenge-response authentication on the relay WebSocket — challenge generation, kind 22242 event verification, per-connection auth state, and write gating based on configurable policy.

### Modified Capabilities

- `nostr-relay-engine`: Update relay info document to advertise NIP-42 support; connection handler dispatches AUTH messages instead of returning "unsupported".

## Impact

- `crates/aspen-nostr-relay/src/connection.rs`: New AUTH message handler, per-connection auth state, EVENT gating logic.
- `crates/aspen-nostr-relay/src/relay.rs`: Pass write policy and relay URL to connection handler; update NIP-11 document.
- `crates/aspen-nostr-relay/src/config.rs`: New `write_policy` field on `NostrRelayConfig`.
- `crates/aspen-nostr-relay/src/constants.rs`: New constants for auth challenge TTL, kind 22242.
- No new crate dependencies — the `nostr` crate v0.44 already provides `ClientMessage::Auth`, `RelayMessage::Auth`, and kind 22242 event types.
