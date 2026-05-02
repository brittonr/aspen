## Context

The previous trust/crypto/secrets blocker made `aspen-crypto` transport-free by default. Fresh dependency inspection showed the next runtime leak was `aspen-secrets -> aspen-auth -> aspen-core-shell`, not direct Redb storage in `aspen-secrets`.

## Decision

Keep portable token data on `aspen-auth-core` and gate runtime auth helpers behind `auth-runtime`.

### Rationale

`CapabilityToken` is already owned by `aspen-auth-core`, so default token parsing does not need the runtime `aspen-auth` shell. `TokenVerifier` and `TokenBuilder` remain useful compatibility helpers for node bootstrap, but they construct runtime auth services and should be opt-in.

### Alternatives

- Split `MountRegistry` storage first: rejected for this slice because it touches more runtime callsites and does not address the observed auth shell dependency.
- Remove verifier/builder helpers outright: rejected because the root node still uses the verifier helper and compatibility can be preserved with an explicit runtime feature.

## Validation

Validate that no-default `aspen-secrets` builds/tests without `aspen-auth`, `aspen-core-shell`, Redb, concrete `iroh`, or `aspen-redb-storage` in the normal dependency tree; separately check `auth-runtime` and node runtime compatibility.
