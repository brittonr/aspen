# Evidence Gates Delta

## ADDED Requirements

### Requirement: Fabricated Octet gate receipts are test-only
r[molten.octet_source_gate_receipt_validation.spec.test_fixture_isolation] Fabricated passing Octet gate receipts used as test fixtures MUST be compiled only into test targets. Library and binary builds MUST NOT contain a production path that supplies them as source-gate evidence.

#### Scenario: Production build excludes the fabricated receipt
- GIVEN the library and binaries are built without test targets
- WHEN any non-test code refers to the fabricated passing receipt
- THEN the build fails instead of admitting the reference

#### Scenario: Tests keep the fixture
- GIVEN library, binary or integration test targets
- WHEN they use the fabricated receipt to exercise gate consumers
- THEN they compile with the test-only fixture enabled

### Requirement: Unqualified fixture gates are never Octet evidence
r[molten.octet_source_gate_receipt_validation.spec.unqualified_fixture_denied] A fixture source gate used where no authenticated Octet output exists MUST be a distinct record that is not an Octet gate receipt. Every Octet source-gate validation consumer, produced-gate validation, upgrade-plan admission, strict job admission and scan-mode node startup MUST deny it. A composition that binds a fixture gate MUST record the strict Octet source-gate check as failed and MUST mark its receipts as bound to an unqualified fixture gate.

#### Scenario: Consumers deny the fixture gate
- GIVEN an unqualified fixture gate value
- WHEN node startup, remote job admission, upgrade planning or node control validates it as a source gate
- THEN validation denies it and binds no Octet gate receipt ref

#### Scenario: Fixture composition is labelled
- GIVEN a crate-internal fixture composition of node startup or loopback job admission
- WHEN it binds the fixture gate
- THEN its receipt records `strict-octet-source-gate-bound` as failed and adds `unqualified-fixture-source-gate`
