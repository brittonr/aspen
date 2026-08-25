# Node Runtime Delta

## ADDED Requirements

### Requirement: Production profile export requires a candidate source input

r[molten.prod_ops.production_profile.candidate_input] The production node profile MUST require one explicit canonical non-placeholder candidate source reference at export. It MUST bind that same reference as the profile source-gate input.

#### Scenario: Reviewed candidate exports

- GIVEN an operator supplies a canonical non-placeholder candidate source reference
- WHEN Nickel exports the production node profile
- THEN the export records that reference at the root and as the only source-gate input.

#### Scenario: Missing candidate denies export

- GIVEN no candidate source reference
- WHEN Nickel exports the production node profile
- THEN evaluation fails before a profile artifact is produced.

#### Scenario: Placeholder candidate denies export

- GIVEN an all-zero or repeated dummy candidate source reference
- WHEN Nickel exports the production node profile
- THEN contract validation fails before a profile artifact is produced.

#### Scenario: Candidate and source-gate mismatch denies export

- GIVEN valid candidate and source-gate references that differ
- WHEN Nickel exports the production node profile
- THEN contract validation fails before a profile artifact is produced.

### Requirement: Candidate fixtures remain explicit non-claims

r[molten.prod_ops.production_profile.candidate_fixture_non_claim] Positive profile fixtures MUST name their deterministic candidate reference explicitly and MUST NOT serve as release evidence.

#### Scenario: Fixture profile exports

- GIVEN the positive fixture supplies its named non-placeholder test reference
- WHEN the fixture rail runs
- THEN the profile exports for conformance testing without claiming candidate readiness.
