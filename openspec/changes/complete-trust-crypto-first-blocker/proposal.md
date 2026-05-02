# Complete Trust/Crypto/Secrets First Blocker

## Summary

Narrow the `aspen-crypto` reusable default boundary so cookie/hash helpers do not pull
Iroh endpoint/runtime dependencies, keep node identity lifecycle behind an explicit
feature, and record trust/crypto/secrets evidence for the current workspace-internal
state.

## Why

The trust/crypto/secrets manifest already records `aspen-trust` as the first pure trust
surface and `aspen-secrets --no-default-features` as buildable. The remaining first
blocker was `aspen-crypto`: its default crate mixed transport-free cookie helpers with
Iroh/tokio identity lifecycle utilities.

## Scope

- `aspen-crypto` default features expose transport-free cookie/hash helpers only.
- `aspen-crypto/identity` enables `iroh-base`, randomness, and Tokio file lifecycle
  support for node identity keys.
- `aspen-secrets` keeps identity re-exports by opting into `aspen-crypto/identity` and
  uses `iroh-base` key types directly instead of concrete `iroh`.
- Test-only `aspen-secrets` KV mocks are updated to the split KV capability traits so
  no-default feature tests are usable evidence.

## Non-goals

- Publishing or repo-splitting trust/crypto/secrets crates.
- Removing runtime trust/share exchange or secrets handler adapters.
- Promoting the aggregate family beyond workspace-internal while publication policy and
  aggregate split boundaries remain open.
