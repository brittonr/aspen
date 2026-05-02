# trust-crypto-secrets-extraction Specification

## Purpose

Defines reusable extraction boundaries for trust, crypto, and secrets helpers so portable cryptographic/state-machine surfaces stay separate from Aspen runtime adapters.

## Requirements

### Requirement: Transport-free crypto default [r[trust-crypto-secrets-extraction.transport-free-crypto-default]]

The trust/crypto/secrets extraction SHALL keep reusable default crypto helpers free of concrete Aspen runtime, transport endpoint, Redb, handler, and node bootstrap dependencies.

#### Scenario: Aspen crypto default excludes runtime identity lifecycle [r[trust-crypto-secrets-extraction.transport-free-crypto-default.aspen-crypto-default]]

- GIVEN `aspen-crypto` is built with no default features
- WHEN its normal dependency graph is inspected
- THEN it SHALL expose cookie/hash helper behavior without normal dependencies on concrete `iroh`, `iroh-base`, Tokio, or randomness crates
- AND node identity lifecycle helpers SHALL require an explicit feature.

#### Scenario: Identity compatibility is explicit [r[trust-crypto-secrets-extraction.transport-free-crypto-default.identity-feature]]

- GIVEN a runtime consumer needs node identity key lifecycle helpers
- WHEN it enables `aspen-crypto/identity`
- THEN it MAY use `iroh-base` key types, randomness, and Tokio file lifecycle helpers without pulling concrete `iroh` endpoint/runtime dependencies through the default crypto surface.

### Requirement: Explicit secrets auth runtime boundary [r[trust-crypto-secrets-extraction.secrets-auth-runtime-boundary]]

The trust/crypto/secrets extraction SHALL keep reusable `aspen-secrets` token parsing independent from the runtime `aspen-auth` shell unless an explicit runtime-auth feature is enabled.

#### Scenario: Secrets default uses portable token data [r[trust-crypto-secrets-extraction.secrets-auth-runtime-boundary.default-token-data]]

- GIVEN `aspen-secrets` is built with no default features
- WHEN a prebuilt capability token is parsed from secrets config
- THEN it SHALL use the portable `aspen-auth-core::CapabilityToken` data type without requiring runtime `aspen-auth` verifier or builder services.

#### Scenario: Runtime auth helpers are opt-in [r[trust-crypto-secrets-extraction.secrets-auth-runtime-boundary.runtime-helper-feature]]

- GIVEN a node bootstrap/runtime consumer needs `TokenVerifier` or `TokenBuilder` helpers
- WHEN it enables `aspen-secrets/auth-runtime`
- THEN it MAY construct those helpers through the runtime `aspen-auth` shell without making that shell part of the reusable default secrets boundary.

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

### Requirement: Secrets handler runtime adapter is explicit [r[trust-crypto-secrets-extraction.secrets-handler-runtime-adapter-boundary]]

The trust/crypto/secrets extraction SHALL keep `aspen-secrets-handler` default builds free of broad Aspen runtime-context and Redb/Raft storage dependencies; only the node/runtime factory adapter MAY enable the full runtime context graph.

#### Scenario: Portable handler default avoids runtime storage [r[trust-crypto-secrets-extraction.secrets-handler-runtime-adapter-boundary.default-portable]]

- GIVEN `aspen-secrets-handler` is built with no default features
- WHEN its normal dependency graph is inspected
- THEN it SHALL compile without normal dependencies on `aspen-core`, `aspen-core-shell`, `aspen-raft`, `aspen-redb-storage`, or `redb`
- AND its executor SHALL use `aspen_traits::KeyValueStore` plus `aspen-kv-types` request types instead of `aspen_core` compatibility re-exports.

#### Scenario: Runtime factory adapter remains opt-in compatible [r[trust-crypto-secrets-extraction.secrets-handler-runtime-adapter-boundary.runtime-adapter]]

- GIVEN Aspen node/runtime handler registration needs `ClientProtocolContext` and `HandlerFactory`
- WHEN `aspen-secrets-handler/runtime-adapter` or the aggregate `aspen-rpc-handlers/secrets` bundle is enabled
- THEN the secrets handler factory SHALL compile with the runtime context and preserve node secrets compatibility.
