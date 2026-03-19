## Why

Every user-facing identity in Forge is a raw ed25519 public key — hex strings like `0ca308066...`. There's no profile, no display name, no way to know who you're collaborating with. Users should be able to use their existing Nostr identity (npub/nsec) to interact with Forge. Their profile, name, and social graph come with them from the Nostr ecosystem.

## What Changes

- Each Nostr npub gets an assigned ed25519 keypair managed by the cluster. The user authenticates with their nsec, and the cluster handles all ed25519 signing internally. No changes to SignedObject, delegate verification, or any crypto paths.
- The `Author` struct gains an `npub` field so commits and COB changes carry the Nostr identity alongside the ed25519 key.
- The web UI resolves authors to Nostr profiles (kind 0 metadata) from the embedded relay — display names and NIP-05 identifiers replace hex keys.
- A Nostr auth flow (NIP-42 or challenge-response) lets users prove npub ownership when connecting via CLI, web, or git-remote-aspen.
- ForgeNode operations accept a user context (npub + assigned ed25519 key) so different users on the same cluster node sign with different keys.

## Capabilities

### New Capabilities

- `nostr-identity-mapping`: npub → ed25519 keypair mapping stored in KV, with challenge-response authentication to prove npub ownership
- `nostr-profile-resolution`: Web UI resolves ed25519 author keys to Nostr profiles (display name, NIP-05) via the embedded relay's kind 0 events
- `nostr-user-context`: ForgeNode operations accept a per-user signing context instead of using the node's single key for everything

### Modified Capabilities

## Impact

- `crates/aspen-forge/src/identity/mod.rs` — Author struct gains `npub` field
- `crates/aspen-forge/src/types.rs` — SignedObject unchanged (still ed25519), but callers pass user-specific keys
- `crates/aspen-forge/src/node.rs` — ForgeNode methods accept a user context for signing
- `crates/aspen-forge/src/cob/store/mod.rs` — CobStore operations accept per-user signing key
- `crates/aspen-forge/src/git/store.rs` — GitBlobStore commit/tree operations accept per-user key
- `crates/aspen-forge-handler/src/executor.rs` — RPC handlers extract user identity from auth context
- `crates/aspen-forge-web/src/templates.rs` — Author rendering resolves npub → profile
- `crates/aspen-nostr-relay/` — Used for profile metadata storage/retrieval
- `crates/aspen-auth/` — Token system extended to carry npub identity
- New crate or module for the npub → ed25519 mapping store and auth challenge flow
