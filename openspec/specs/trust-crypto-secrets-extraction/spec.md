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
