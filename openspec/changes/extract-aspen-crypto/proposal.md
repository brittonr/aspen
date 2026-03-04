# Extract `aspen-crypto` Lightweight Crate

## Problem

`aspen-secrets` consolidates all crypto/secrets code but is too heavy for a required dependency — it pulls age, rcgen, x509-cert, chacha20poly1305, ed25519-dalek, etc. This forces downstream crates (`aspen-cluster`, `aspen-auth`, `aspen-dht-discovery`) to maintain their own copies of cookie validation, HMAC key derivation, and identity key management (~80 lines of duplicated code with "see also" comments pointing at `aspen-secrets`).

## Solution

Extract a new `aspen-crypto` crate containing only the lightweight, widely-needed primitives:

- **Cookie**: validation, HMAC key derivation, gossip topic derivation
- **Identity**: `NodeIdentityProvider` — generate, parse hex, load/save, public key access

Dependencies: blake3, hex, iroh, rand, tokio, thiserror (all already in the dep tree). Zero heavy crypto deps.

## Impact

- `aspen-cluster` drops ~80 lines of duplicate cookie/identity code
- `aspen-auth` delegates `derive_hmac_key` to canonical source
- `aspen-secrets` re-exports from `aspen-crypto` (no breaking changes)
- Single source of truth for cookie validation and node identity lifecycle
