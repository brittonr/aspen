# Testing Harness Delta: Sans-IO Protocol Fixtures

### Requirement: Sans-IO protocol fixtures cover positive and negative behavior
r[molten.testing.sans_io_positive_negative_fixtures] Molten SHOULD provide focused Sans-IO protocol fixtures for implemented protocol cores, including positive tests for deterministic transitions and negative tests for malformed messages, missing evidence, illegal transitions, hidden ambient inputs, and shell mutation before admission.

#### Scenario: In-memory fixture observes deterministic output
- GIVEN a protocol core fixture with explicit state, input message, limit profile, and admission facts
- WHEN the fixture runs the core twice
- THEN both runs produce identical state deltas, envelope descriptors, effect intents, and diagnostics.

#### Scenario: Hidden ambient dependency fails fixture review
- GIVEN a protocol core attempts to read wall-clock, random, filesystem, process, network, or database state during transition evaluation
- WHEN the Sans-IO fixture or static check evaluates the core boundary
- THEN the check fails before the transition can be accepted as replayable evidence.

#### Scenario: Shell mutation before admission is rejected
- GIVEN a shell writes state or sends a frame before the returned intent passes admission
- WHEN the negative fixture runs
- THEN the fixture fails and reports pre-admission mutation as a boundary violation.