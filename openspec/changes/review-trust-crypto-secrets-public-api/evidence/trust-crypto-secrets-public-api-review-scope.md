# Trust/Crypto/Secrets Public API Review Scope

## Decision

Do not promote `trust-crypto-secrets` to `extraction-ready-in-workspace` in this review. The previous boundary slices produced reusable sub-surfaces, but the aggregate still needs real checker coverage and explicit public API stability policy before a readiness raise.

## Canonical reusable surfaces

- `aspen-crypto` default/no-default cookie/hash helper surface.
- `aspen-secrets-core` constants plus KV, Transit, and PKI type/state contracts.
- Selected `aspen-trust` pure helper/state surfaces, pending explicit async/default dependency and serialization policy.

## Runtime or compatibility surfaces

- `aspen-crypto/identity`: node identity lifecycle helpers with `iroh-base`, randomness, and Tokio file lifecycle support.
- `aspen-secrets`: service/runtime implementation crate; not lightweight under no-default features.
- `aspen-secrets/auth-runtime`: runtime auth verifier/builder integration.
- `aspen-secrets-handler`: RPC compatibility consumer; runtime factory remains behind `runtime-adapter`.

## Promotion blockers

1. The readiness checker maps `trust-crypto-secrets` to an aggregate pseudo-candidate and warns that direct package dependency checks are deferred.
2. `aspen-trust` default public surface exposes async/Tokio service APIs; policy must decide whether this is acceptable or should be feature-gated.
3. Trust and secrets serialization contracts need explicit ownership and, for stable formats, golden/roundtrip evidence.
4. `aspen-secrets --no-default-features` is still an implementation/service graph, so documentation must not present it as a portable lightweight API.
