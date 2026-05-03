## ADDED Requirements

### Requirement: Aspen Trust Async Default Policy [r[trust-crypto-secrets-extraction.aspen-trust-async-default-policy]]

`aspen-trust` SHALL make its default/no-default reusable surface explicit by keeping async runtime service APIs behind a named feature while preserving runtime compatibility for consumers that opt in.

#### Scenario: Default trust surface excludes async runtime service dependencies [r[trust-crypto-secrets-extraction.aspen-trust-async-default-policy.default-excludes-async]]

- GIVEN a consumer depends on `aspen-trust` with default or no default features
- WHEN dependency evidence is captured for the trust/crypto/secrets readiness checker
- THEN the normal dependency graph SHALL NOT include `tokio` or `async-trait` through `aspen-trust`
- AND `key_manager` and `reencrypt` SHALL require the explicit `async` feature.

#### Scenario: Runtime trust consumers opt into async service APIs [r[trust-crypto-secrets-extraction.aspen-trust-async-default-policy.runtime-opt-in]]

- GIVEN a runtime consumer needs the trust key manager or reencryption service APIs
- WHEN it enables its trust runtime feature
- THEN it SHALL enable `aspen-trust/async` explicitly and continue to compile.

#### Scenario: Trust checker validates real trust package coverage [r[trust-crypto-secrets-extraction.aspen-trust-async-default-policy.checker-coverage]]

- GIVEN the trust/crypto/secrets family readiness checker runs
- WHEN selected pure trust helper/state surfaces are considered reusable candidates
- THEN it SHALL validate `aspen-trust` as a real package candidate with the documented default dependency policy.
