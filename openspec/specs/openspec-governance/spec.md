# openspec-governance Specification

## Purpose

Defines governance requirements for OpenSpec-driven changes, including durable task evidence, validation guardrails, and archive-ready verification records.
## Requirements
### Requirement: Checked tasks have durable evidence
Every checked OpenSpec task SHALL cite durable evidence that exists in the repository and supports the task claim.

ID: openspec-governance.task-evidence-coverage

#### Scenario: Checked task cites valid evidence
ID: openspec-governance.task-evidence-coverage.checked-task-cites-valid-evidence

- **GIVEN** a checked task appears in `tasks.md`
- **WHEN** preflight reads `verification.md`
- **THEN** the task SHALL appear verbatim under task coverage
- **AND** at least one cited evidence path SHALL exist, be repo-relative, and be tracked
- **AND** cited evidence SHALL be either under the current change directory or a currently changed source/documentation file listed as implementation evidence

#### Scenario: Missing evidence fails
ID: openspec-governance.task-evidence-coverage.missing-evidence-fails

- **GIVEN** a checked task has no matching evidence line
- **WHEN** preflight runs
- **THEN** it SHALL fail with the task text and missing coverage reason
- **AND** the failure output SHALL include concrete remediation guidance

#### Scenario: Invalid evidence output is actionable
ID: openspec-governance.task-evidence-coverage.invalid-evidence-output-actionable

- **GIVEN** a checked task cites an invalid evidence path or invalid evidence content
- **WHEN** preflight runs
- **THEN** it SHALL identify the task text, evidence path or reason, and concrete remediation guidance

#### Scenario: Placeholder evidence fails
ID: openspec-governance.task-evidence-coverage.placeholder-evidence-fails

- **GIVEN** a checked task cites an empty or placeholder evidence artifact
- **WHEN** preflight runs
- **THEN** it SHALL fail before archive or done-review can pass

### Requirement: Ambiguous review states use oracle checkpoints
When deterministic evidence cannot resolve a review-critical question, the workflow SHALL record an explicit checkpoint instead of claiming completion.

ID: openspec-governance.review-oracle-checkpoints

#### Scenario: Checkpoint records ambiguity
ID: openspec-governance.review-oracle-checkpoints.records-ambiguity

- **GIVEN** a reviewer cannot verify a design or task claim from supplied evidence
- **WHEN** an oracle checkpoint is created
- **THEN** it SHALL record the question, inspected evidence, decision, owner, and next action

#### Scenario: Missing checkpoint is flagged
ID: openspec-governance.review-oracle-checkpoints.missing-checkpoint-flagged

- **GIVEN** a completion claim depends on evidence that is not available
- **WHEN** done-review runs
- **THEN** it SHALL flag missing oracle checkpoint or require the claim to be withdrawn

#### Scenario: Concrete blocker is acceptable
ID: openspec-governance.review-oracle-checkpoints.concrete-blocker-acceptable

- **GIVEN** no oracle decision exists
- **WHEN** the agent reports a concrete blocker and stops without claiming completion
- **THEN** review SHALL accept the stop condition as non-overclaiming
