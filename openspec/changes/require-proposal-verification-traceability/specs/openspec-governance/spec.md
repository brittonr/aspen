## ADDED Requirements

### Requirement: Proposals trace verification to changed requirements
Proposal verification sections SHALL identify how changed capabilities are expected to be verified.

ID: openspec-governance.proposal-verification-traceability

#### Scenario: Proposal cites changed requirement IDs
ID: openspec-governance.proposal-verification-traceability.cites-requirement-ids

- **GIVEN** a proposal has delta specs with requirement IDs
- **WHEN** proposal gate runs
- **THEN** the proposal verification section SHALL cite those IDs or explicitly defer them to design with rationale

#### Scenario: Missing positive verification fails
ID: openspec-governance.proposal-verification-traceability.missing-positive-verification-fails

- **GIVEN** an added requirement has no verification expectation
- **WHEN** proposal gate runs
- **THEN** it SHALL fail with the requirement ID

#### Scenario: Negative behavior needs verification expectation
ID: openspec-governance.proposal-verification-traceability.negative-behavior-needs-verification

- **GIVEN** a proposal or delta spec includes failure, rejection, unauthorized, malformed, or timeout behavior
- **WHEN** proposal verification is checked
- **THEN** at least one negative-path expectation SHALL be listed or explicitly deferred
