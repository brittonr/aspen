# Job DAG delta

## ADDED Requirements

### Requirement: Checked selected sequence profile
r[molten.job_dag.selected_sequence] Explicit selected-sequence-v2 graphs MUST bind supported stages to registry implementation identities and exact integer-sequence schemas, validate direction, required inputs and multiplicity before execution, route only selected outputs, bound fan-in and preserve port-labelled outputs through caching. V1 MUST retain its existing semantics.

#### Scenario: Distinct partition outputs
- GIVEN an admitted v2 partition followed by a consumer of left
- WHEN executed or replayed from cache
- THEN only the left sequence reaches the consumer.

#### Scenario: Invalid wiring
- GIVEN an unknown port, schema mismatch, absent required input or excess single-input connection
- WHEN graph admission runs
- THEN admission denies before stages execute.

### Requirement: Closed authoring boundary
r[molten.job_dag.captured_authoring] Captured graph authoring MUST lower bounded closed values through canonical JobDag parsing and preserve control dependencies, explicit instances and selected-output references. Construction provenance MUST NOT grant execution authority.

#### Scenario: Shared producer
- GIVEN two handles refer to one explicit producer instance
- WHEN lowered
- THEN the graph retains one producer and both declared edges.
