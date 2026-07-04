# Testing Harness Delta: Contract invariant negative fixtures

### Requirement: Contract invariants have positive and negative fixtures
r[molten.testing.contract_negative_fixtures.invariant_matrix] Each repository-owned Nickel contract module SHOULD provide positive fixtures for reviewed valid exports and focused negative fixtures for every exported field-domain or cross-field invariant, or record an explicit fixture exemption.

#### Scenario: Valid contract fixture exports
- GIVEN a contract module with reviewed valid source fixtures
- WHEN fixture validation runs
- THEN every positive fixture exports successfully and matches the reviewed generated artifact when one is checked in

#### Scenario: Invalid contract fixture fails
- GIVEN a negative fixture that violates exactly one documented field-domain or cross-field invariant
- WHEN fixture validation runs
- THEN the fixture fails before generated JSON or Preserves evidence is refreshed

### Requirement: Negative fixture failure classes are reviewable
r[molten.testing.contract_negative_fixtures.failure_classes] Negative contract fixtures SHOULD name the expected failure class or invariant so a fixture that fails for the wrong reason remains visible during review.

#### Scenario: Fixture fails for expected invariant
- GIVEN a negative fixture named for a malformed ref, duplicate id, missing evidence, invalid enum, stale metadata, or cross-field contradiction
- WHEN validation reports the failure
- THEN reviewers can identify the intended invariant rather than treating any failure as sufficient
