## Why

Aspen has no outward-facing protocol that external ecosystems can consume. Forge repos, CI pipelines, issues, and KV state are only visible to iroh-connected clients. The Nostr protocol (NIP-01) provides a simple, widely-adopted event format with an existing client ecosystem. By running a Nostr relay inside Aspen, the cluster exposes a read (and eventually write) view of its state to any Nostr client — without compromising the iroh-only internal transport.

The infrastructure already exists: `iroh-relay` runs an HTTP server on each node, `iroh-proxy-utils` handles WebSocket upgrades, and the WASM plugin system supports KV access and hook subscriptions. The gap is a native relay engine for NIP-01 framing/subscriptions and a plugin host function to let WASM plugins emit Nostr events.

## What Changes

- New crate `aspen-nostr-relay`: NIP-01 relay engine with WebSocket transport, event storage over KV, subscription registry, and filter matching.
- New feature flag `nostr-relay` on `aspen-node` gating the relay functionality.
- New WASM plugin host function `nostr_publish_event` allowing plugins to emit Nostr events into the relay.
- New `nostr_publish` permission in `PluginPermissions`.
- New secp256k1 cluster Nostr key for signing bridge-generated events, stored alongside `ClusterIdentity`.
- First bridge plugin (`nostr-forge-bridge`): subscribes to Forge hook events, translates to NIP-34 events (repo announcements, state, patches, issues), publishes via `nostr_publish_event`.
- WebSocket endpoint on the existing relay server HTTP port (or configurable standalone port) accepting Nostr client connections.
- KV storage schema for Nostr events with indexes by kind, author, tag, and creation time.

## Capabilities

### New Capabilities

- `nostr-relay-engine`: NIP-01 compliant relay engine — WebSocket transport, event storage/retrieval over KV, subscription registry with filter matching, event fan-out to connected clients, relay information document (NIP-11).
- `nostr-plugin-bridge`: WASM plugin host function (`nostr_publish_event`) and permission (`nostr_publish`) enabling plugins to emit Nostr events into the relay engine.
- `nostr-forge-bridge`: WASM plugin that subscribes to Forge hook events and translates them to NIP-34 Nostr events (kind 30617 repo announcements, kind 30618 state, kind 1617 patches, kind 1621 issues).
- `nostr-key-management`: Cluster-level secp256k1 keypair generation, storage, and signing for Nostr event authorship.

### Modified Capabilities

(none — all changes are additive behind a feature flag)

## Impact

- **New crate**: `crates/aspen-nostr-relay/` (~3,000-5,000 lines estimated)
- **New dependencies**: `nostr` crate (core event types, NIP encoding), `k256` or `secp256k1` (signing), `tokio-tungstenite` (WebSocket)
- **Modified crates**:
  - `aspen-plugin-api`: new `nostr_publish` permission field, new host function signature
  - `aspen-cluster`: relay server integration point for Nostr WebSocket handler
  - `aspen-federation` (or new module): secp256k1 key management for Nostr identity
- **New WASM plugin**: `nostr-forge-bridge` (separate build artifact, installable at runtime)
- **Config**: new `[nostr_relay]` config section (enabled, port, max connections, max subscriptions per connection, max event size)
- **Feature flag**: `nostr-relay` in top-level Cargo.toml, gating all Nostr relay functionality
- **No breaking changes**: entirely additive, disabled by default
