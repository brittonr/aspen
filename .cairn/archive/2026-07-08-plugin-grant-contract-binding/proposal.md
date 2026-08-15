## Why

Plugin capability grant fixtures validate local grant shape, but the authored Nickel contract does not fully prove the grant is bound to the referenced plugin extension contract descriptor. Runtime Rust later checks operation, descriptor, schema, and resource relationships, but authoring-time contracts can catch stale or mismatched grant evidence earlier.

## What Changes

- Add Nickel authoring helpers that validate a plugin capability grant against a supplied plugin extension contract export.
- Require grant operation, descriptor ref, input schema ref, output schema ref, replay class, effect refs, resource scope, and production profile expectations to match the referenced descriptor and contract.
- Tighten revocation and attenuation invariants where those are local grant properties.
- Keep runtime Rust semantic gates authoritative and fail-closed.

## Impact

- **Files**: plugin extension Nickel grant/contract modules, fixtures, generated exports, plugin host grant parser tests, and drift gates.
- **Testing**: valid bound grants export; wrong contract ref, wrong descriptor, schema mismatch, resource over-scope, replay mismatch, revoked-without-evidence, and inverted validity fixtures fail.
