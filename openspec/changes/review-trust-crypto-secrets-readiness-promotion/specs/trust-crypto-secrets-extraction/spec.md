## ADDED Requirements

### Requirement: Package-scoped readiness promotion [r[trust-crypto-secrets-extraction.package-scoped-readiness-promotion]]

The trust/crypto/secrets extraction SHALL promote only real reusable package candidates whose dependency, compatibility, serialization, and owner/public API evidence pass.

#### Scenario: Reusable packages may advance independently [r[trust-crypto-secrets-extraction.package-scoped-readiness-promotion.reusable-packages]]

- GIVEN `aspen-crypto`, default/no-default `aspen-trust`, and `aspen-secrets-core` are the canonical reusable package candidates
- WHEN the fresh readiness review verifies real checker coverage and evidence for those candidates
- THEN those package candidates MAY be marked `extraction-ready-in-workspace`
- AND the aggregate trust/crypto/secrets family SHALL remain `workspace-internal` unless a future change explicitly promotes or excludes all remaining runtime surfaces.

#### Scenario: Runtime secrets crates remain scoped internally [r[trust-crypto-secrets-extraction.package-scoped-readiness-promotion.runtime-scoped]]

- GIVEN `aspen-secrets` and `aspen-secrets-handler` contain runtime services, implementation dependencies, adapters, or compatibility paths
- WHEN the readiness decision is recorded
- THEN those crates SHALL remain runtime or compatibility surfaces
- AND they SHALL NOT be marketed as reusable public API candidates without an explicit future OpenSpec slice.
