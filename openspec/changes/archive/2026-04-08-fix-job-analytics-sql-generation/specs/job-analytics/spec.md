## ADDED Requirements

### Requirement: Analytics SQL builders generate valid predicate syntax

The job analytics SQL builders SHALL generate syntactically valid SQL for every supported optional-filter combination. Optional predicates SHALL be composed with at most one `WHERE` clause and additional predicates SHALL be joined with `AND`.

#### Scenario: Average duration with status filter only

- **WHEN** the average-duration analytics query is built with `status = Some(...)` and `job_type = None`
- **THEN** the generated SQL SHALL contain `WHERE status = ...`
- **AND** it SHALL NOT contain `FROM jobs AND`

#### Scenario: Average duration with both filters

- **WHEN** the average-duration analytics query is built with both `job_type` and `status`
- **THEN** the generated SQL SHALL contain one `WHERE` clause
- **AND** the `job_type` and `status` predicates SHALL be joined with `AND`

#### Scenario: Queue depth with priority filter

- **WHEN** the queue-depth analytics query is built with `priority = Some(...)`
- **THEN** the generated SQL SHALL contain `WHERE status = 'Queued' AND priority = ...`
- **AND** it SHALL NOT contain a second `WHERE`

### Requirement: Analytics SQL coverage includes optional-filter combinations

The analytics SQL unit tests SHALL cover the optional-filter combinations that affect predicate assembly.

#### Scenario: Average duration filter combinations are tested

- **WHEN** the analytics SQL test suite runs
- **THEN** it SHALL exercise average-duration SQL generation for no filters, status-only, job-type-only, and both filters present

#### Scenario: Queue depth priority filter is tested

- **WHEN** the analytics SQL test suite runs
- **THEN** it SHALL exercise queue-depth SQL generation with and without the optional priority filter
