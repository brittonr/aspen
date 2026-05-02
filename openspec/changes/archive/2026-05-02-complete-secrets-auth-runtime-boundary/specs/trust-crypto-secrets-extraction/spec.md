## ADDED Requirements

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
