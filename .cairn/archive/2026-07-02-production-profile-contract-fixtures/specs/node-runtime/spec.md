# Node Runtime Delta: Production profile contract fixtures

### Requirement: Production profile contracts have positive and negative fixtures
r[molten.prod_ops.profile_contract_fixtures.positive_negative] Production profile Nickel contracts MUST be covered by positive fixtures for reviewed valid profiles and negative fixtures for malformed refs, missing evidence arrays, unsafe paths, vocabulary typos, invalid resource limits, cross-field invariant failures, and metadata errors.

#### Scenario: Reviewed profile fixture exports
- GIVEN the checked-in production profile fixture represents the reviewed valid profile
- WHEN fixture validation runs
- THEN Nickel export succeeds and the exported JSON matches the reviewed profile expectation

#### Scenario: Invalid profile fixtures fail
- GIVEN negative fixtures that each violate one production profile contract or invariant
- WHEN fixture validation runs
- THEN each negative fixture fails Nickel export and reports the expected failure class

### Requirement: Profile fixture validation is deterministic
r[molten.prod_ops.profile_contract_fixtures.validation_gate] Production profile fixture validation MUST run without live network, production credentials, mutable state roots, or ambient filesystem assumptions beyond reading source-controlled fixture files.

#### Scenario: Fixture gate runs locally
- GIVEN the repository checkout contains the profile contract and fixture files
- WHEN the profile fixture validation command runs
- THEN it deterministically reports valid positive exports and rejected negative exports using only source-controlled inputs

#### Scenario: Fixture regression blocks profile evidence update
- GIVEN a profile contract edit accidentally accepts an invalid fixture or changes the valid export unexpectedly
- WHEN the profile fixture validation command runs
- THEN validation fails before production readiness receipt expectations are updated

### Requirement: Profile fixtures are static-contract evidence only
r[molten.prod_ops.profile_contract_fixtures.evidence_boundary] Production profile fixture results MUST NOT replace runtime startup receipts, source-gate freshness checks, adapter conformance evidence, resource-pressure observations, or production drill receipts.

#### Scenario: Fixture pass does not grant runtime trust
- GIVEN all profile contract fixtures pass
- WHEN a production node startup or release gate needs live authority, source-gate, adapter, resource, or drill evidence
- THEN the normal subsystem receipts remain required and fixture results alone are insufficient
