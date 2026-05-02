## ADDED Requirements

### Requirement: Secrets storage uses lightweight KV traits [r[trust-crypto-secrets-extraction.secrets-kv-traits-boundary]]

The trust/crypto/secrets extraction SHALL keep reusable `aspen-secrets` Aspen-backed storage adapter signatures on the lightweight KV trait/type crates instead of the broad `aspen-core` compatibility surface.

#### Scenario: Secrets storage signatures avoid aspen-core [r[trust-crypto-secrets-extraction.secrets-kv-traits-boundary.no-aspen-core-signatures]]

- GIVEN `aspen-secrets` exposes `AspenSecretsBackend`, `MountRegistry`, or SOPS runtime KV manager APIs
- WHEN those APIs accept or store a KV backend
- THEN they SHALL use `aspen_traits::KeyValueStore` and `aspen-kv-types` operation types rather than naming `aspen_core::KeyValueStore` or `aspen_core` request/result compatibility re-exports.

#### Scenario: Runtime compatibility remains intact [r[trust-crypto-secrets-extraction.secrets-kv-traits-boundary.runtime-compatibility]]

- GIVEN existing runtime stores implement the Aspen KV trait through compatibility re-exports
- WHEN `aspen-secrets-handler` and node secrets bootstrap are checked
- THEN they SHALL continue to compile without adding `aspen-core` back to the `aspen-secrets` no-default dependency boundary.
