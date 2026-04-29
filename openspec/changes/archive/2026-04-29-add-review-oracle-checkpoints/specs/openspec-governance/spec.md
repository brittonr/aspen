## ADDED Requirements

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
