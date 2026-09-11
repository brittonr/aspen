# Runtime spine: turn causality delta

## ADDED Requirements

### Requirement: Committed turns record a validated cause
r[molten.runtime_spine.turn_causality] Molten MUST record a cause from one closed vocabulary on every committed turn record, MUST include the cause in the canonical record identity, MUST fail validation for a record with an absent or unknown cause, and MUST NOT derive a cause from a wall clock.

#### Scenario: Dependency-driven turn names its cause
- GIVEN a turn that runs because an earlier turn released dependent work
- WHEN the trace record is committed
- THEN the record names the releasing turn by ref as its cause.

#### Scenario: Cleanup turn is identified
- GIVEN an owner scope is cleaned up and its outstanding assertions retract
- WHEN the trace record is committed
- THEN the record names `cleanup` as its cause.

#### Scenario: Missing or unknown cause fails validation
- GIVEN a trace record with an absent cause or a cause outside the vocabulary
- WHEN trace validation runs
- THEN validation fails closed and the record does not satisfy replayable trace evidence.

#### Scenario: Cause change is visible to replay
- GIVEN a recorded turn trace and a replay of the same actions with a different cause
- WHEN replay compares canonical refs
- THEN the comparison reports divergence.
