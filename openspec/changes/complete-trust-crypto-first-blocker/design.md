# Design

## Boundary

`aspen-crypto` now separates two surfaces:

- default: transport-free cookie validation and BLAKE3-derived keys/topics;
- `identity`: node identity key lifecycle using `iroh_base::{PublicKey, SecretKey}`,
  `rand`, and Tokio filesystem helpers.

Using `iroh-base` instead of `iroh` keeps identity tied to Aspen's established key type
without pulling concrete endpoint discovery, QUIC runtime, or relay/discovery crates.

## Compatibility

`aspen-secrets::identity::*` remains a compatibility re-export because `aspen-secrets`
opts into `aspen-crypto/identity`. Secrets provider APIs keep returning the same key
types via `iroh-base`, matching `aspen-auth` and existing token/signing code.

## Readiness

This completes the manifest's first blocker but does not make the aggregate family
publishable. The family stays `workspace-internal` until trust/secrets split policy,
license/publication policy, and aggregate candidate modeling are resolved.
