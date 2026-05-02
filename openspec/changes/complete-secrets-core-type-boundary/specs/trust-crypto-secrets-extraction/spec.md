## ADDED Requirements

### Requirement: Secrets core owns portable type contracts [r[trust-crypto-secrets-extraction.secrets-core-type-boundary]]

The trust/crypto/secrets extraction SHALL expose portable secrets constants, KV DTO/state types, Transit DTO/key state types, and PKI DTO/state types from a dependency-light core crate instead of requiring consumers to depend on the runtime secrets service shell.

#### Scenario: Core type crate avoids runtime dependencies [r[trust-crypto-secrets-extraction.secrets-core-type-boundary.core-dependency-rail]]

- GIVEN `aspen-secrets-core` is built with no default features
- WHEN its normal dependency graph is inspected
- THEN it SHALL depend only on portable serialization support and SHALL NOT pull Tokio, age, rcgen, x509, Aspen client/transport/trust/auth runtime crates, Iroh, AEAD runtime crates, Redb, or handler/runtime shells.

#### Scenario: Runtime compatibility re-exports remain [r[trust-crypto-secrets-extraction.secrets-core-type-boundary.compatibility-reexports]]

- GIVEN existing runtime code imports secrets constants or KV, Transit, or PKI type modules through `aspen-secrets`
- WHEN `aspen-secrets`, `aspen-secrets-handler`, and the node secrets bundle are checked
- THEN those compatibility paths SHALL compile while the canonical type/state definitions live in `aspen-secrets-core`.
