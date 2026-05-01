## ADDED Requirements

### Requirement: Completion claims require evidence
Agent completion summaries SHALL not claim clean status, empty queues, completed archives, or successful checks unless command evidence exists.

ID: openspec-governance.completion-claim-evidence

#### Scenario: Evidence-backed claim passes
ID: openspec-governance.completion-claim-evidence.evidence-backed-claim-passes

- **GIVEN** a final response claims git status is clean
- **WHEN** done-review inspects the turn
- **THEN** recent `git status --short` or equivalent evidence SHALL be present

#### Scenario: Unsupported claim fails review
ID: openspec-governance.completion-claim-evidence.unsupported-claim-fails

- **GIVEN** a final response claims an OpenSpec queue is empty without evidence
- **WHEN** done-review runs
- **THEN** it SHALL produce a blocker or major finding

#### Scenario: Uncertain summary is allowed
ID: openspec-governance.completion-claim-evidence.uncertain-summary-allowed

- **GIVEN** evidence is unavailable
- **WHEN** the final response avoids strong completion claims and names the blocker
- **THEN** done-review SHALL NOT fail for overclaiming
