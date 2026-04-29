## ADDED Requirements

### Requirement: OpenSpec tasks are bounded and ordered
OpenSpec task lists SHALL decompose implementation work into bounded, dependency-aware tasks.

ID: openspec-governance.task-size-and-ordering

#### Scenario: Bounded implementation task passes
ID: openspec-governance.task-size-and-ordering.bounded-implementation-task-passes

- **GIVEN** an implementation task covers a small coherent requirement set
- **WHEN** tasks gate runs
- **THEN** it SHALL pass size/order checks

#### Scenario: Oversized implementation task is flagged
ID: openspec-governance.task-size-and-ordering.oversized-implementation-task-flagged

- **GIVEN** one implementation task bundles multiple independent deliverables or many scenario IDs
- **WHEN** tasks gate runs
- **THEN** it SHALL warn or fail with split guidance

#### Scenario: Integration verification task may cover many scenarios
ID: openspec-governance.task-size-and-ordering.integration-verification-may-be-broad

- **GIVEN** a verification task is explicitly labeled as an integration proof
- **WHEN** it covers many requirement or scenario IDs
- **THEN** the tasks gate MAY accept the broad coverage if the task names the integration boundary and required evidence

#### Scenario: Dependency order is explicit
ID: openspec-governance.task-size-and-ordering.dependency-order-explicit

- **GIVEN** a task depends on earlier foundation work
- **WHEN** tasks gate runs
- **THEN** the dependency SHALL be expressed by ordering or an explicit prerequisite note
