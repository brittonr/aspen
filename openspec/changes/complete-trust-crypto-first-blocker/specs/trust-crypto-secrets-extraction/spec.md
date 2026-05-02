## ADDED Requirements

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
