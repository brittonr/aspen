## Context

Forge operations are signed with ed25519 keys from `iroh::SecretKey`. Each ForgeNode holds a single secret key used for all operations — commits, COB changes, ref updates. `SignedObject<T>` wraps payloads with an ed25519 signature and author public key. The `Author` struct carries a name, email, and optional `iroh::PublicKey`.

The embedded Nostr relay (`aspen-nostr-relay`) runs a NIP-01 relay over WebSocket, stores events in Raft KV, and has its own secp256k1 identity. It's fully functional with 42 passing tests but not connected to Forge operations.

## Goals / Non-Goals

**Goals:**

- Users authenticate with their existing Nostr nsec/npub
- Each npub gets a managed ed25519 keypair — Forge crypto stays ed25519 unchanged
- The web UI shows Nostr display names and NIP-05 identifiers instead of hex keys
- Different users on the same cluster create distinct commits/issues/patches

**Non-Goals:**

- Changing SignedObject or delegate verification to secp256k1
- Requiring Nostr identity (anonymous/unlinked ed25519 usage stays supported)
- Federating Nostr profiles across relays (local relay only for now)
- NIP-34 event publishing (future work, separate change)

## Decisions

**Keypair mapping is stored in Raft KV, encrypted with the cluster's key.**

Key: `_identity:npub:{npub_hex}` → Value: encrypted ed25519 secret key bytes + metadata (created_at, last_used). The cluster's iroh secret key derives an encryption key via BLAKE3 keyed hash. Only nodes in the cluster can decrypt the stored ed25519 keys.

Alternative: derive ed25519 from secp256k1 deterministically. Rejected — there's no standard for cross-curve derivation, and it would mean losing the ed25519 key if the user changes their Nostr key.

**Authentication uses a simple challenge-response, not NIP-42.**

NIP-42 is designed for relay authentication over WebSocket. Our auth happens over iroh QUIC (for CLI/git) or HTTP (for web). The flow: client sends npub, server returns a random challenge, client signs the challenge with their nsec (secp256k1 Schnorr), server verifies, server issues a capability token bound to the npub. The token carries the npub and the assigned ed25519 public key.

Alternative: NIP-42 over the Nostr relay WebSocket. Rejected for CLI/git flows where there's no WebSocket connection.

**ForgeNode gains a `UserContext` parameter, not a permanent key swap.**

Current: `ForgeNode::new(blobs, kv, secret_key)` — one key for the node.
New: ForgeNode keeps the node key as default. Individual operations (`commit`, `create_issue`, `set_ref`, etc.) accept an optional `UserContext { npub, signing_key }`. If absent, the node key is used (backward compatible).

This avoids refactoring ForgeNode's construction. The RPC handler layer resolves the authenticated npub to a UserContext before calling ForgeNode methods.

**Profile resolution is lazy and cached.**

When the web UI needs to display an author, it checks: ed25519 key → npub mapping (KV lookup) → kind 0 profile event (relay query). Results are cached in-memory with a 5-minute TTL. Missing profiles fall back to showing the npub in `npub1...` bech32 format, or the ed25519 hex if no npub is linked.

**The web UI login flow uses a Nostr browser extension (NIP-07) or manual nsec entry.**

NIP-07 defines `window.nostr.signEvent()` which browser extensions (nos2x, Alby) implement. The web UI calls it to sign the challenge without the user exposing their nsec. Fallback: paste nsec directly (with a warning). The signed challenge is sent as a cookie/header on subsequent requests.

## Risks / Trade-offs

**[Stored secret keys are a high-value target]** → Encrypted at rest with cluster key. Only accessible to cluster members. Key rotation possible by re-encrypting with a new cluster key. A compromise of the cluster key exposes all managed ed25519 keys — but the cluster key already gates access to all KV data anyway.

**[Users can't export their ed25519 key]** → By design. The ed25519 key is a managed credential, not a portable identity. The npub IS the portable identity. If a user moves to a different cluster, they authenticate with the same npub and get a new ed25519 key on that cluster.

**[Profile data depends on relay availability]** → Local relay is co-located with the cluster, so it's available when the cluster is. Users need to publish their kind 0 profile to the cluster's relay (or the cluster fetches it from the user's configured relays on first auth).
