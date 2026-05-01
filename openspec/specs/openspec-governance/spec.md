## Purpose

Govern OpenSpec authoring, validation, and archival workflows so active changes remain reviewable, traceable, and deterministic before implementation and archive.
## Requirements
### Requirement: Delta specs are structurally consistent
OpenSpec delta specs SHALL be deterministic, ID-bearing, and consistent with main specs for modifications.

ID: openspec-governance.delta-spec-consistency

#### Scenario: Added requirements have IDs
ID: openspec-governance.delta-spec-consistency.added-requirements-have-ids

- **GIVEN** a delta spec adds a requirement or scenario
- **WHEN** validation runs
- **THEN** every added requirement and scenario SHALL include an `ID:` line

#### Scenario: Modified requirement exists
ID: openspec-governance.delta-spec-consistency.modified-requirement-exists

- **GIVEN** a delta spec modifies a requirement
- **WHEN** validation compares it with the main spec
- **THEN** the target requirement SHALL exist by ID or heading

#### Scenario: Missing modified target fails
ID: openspec-governance.delta-spec-consistency.missing-modified-target-fails

- **GIVEN** a modified requirement has no matching main spec requirement
- **WHEN** validation runs
- **THEN** it SHALL fail with the missing requirement identifier

#### Scenario: Removed requirement exists
ID: openspec-governance.delta-spec-consistency.removed-requirement-exists

- **GIVEN** a delta spec removes a requirement by ID or heading
- **WHEN** validation compares it with the main spec
- **THEN** the target requirement SHALL exist or validation SHALL fail with the missing removal target

#### Scenario: Migration note permits intentional repair
ID: openspec-governance.delta-spec-consistency.migration-note-permits-repair

- **GIVEN** a modified requirement repairs or migrates a legacy spec entry that is missing from the current main spec
- **WHEN** the delta includes an explicit `Migration note:` with rationale
- **THEN** validation MAY accept the modification while preserving the rationale in review output

#### Scenario: Conflicting feature contracts warn
ID: openspec-governance.delta-spec-consistency.conflicting-feature-contracts-warn

- **GIVEN** proposal, design, and delta specs use conflicting feature-contract language for the same capability
- **WHEN** validation runs
- **THEN** it SHALL emit a warning naming the conflicting artifacts and phrases

### Requirement: Drain completion proves archive cleanliness
OpenSpec drain completion SHALL include durable evidence that active changes are empty and archived paths are consistent.

ID: openspec-governance.drain-archive-cleanliness

#### Scenario: Clean drain passes
ID: openspec-governance.drain-archive-cleanliness.clean-drain-passes

- **GIVEN** all completed changes have been archived
- **WHEN** drain completion audit runs
- **THEN** no active change directories SHALL remain outside `archive` or `_done`
- **AND** `.drain-state.md` SHALL be absent

#### Scenario: Leftover active path fails
ID: openspec-governance.drain-archive-cleanliness.leftover-active-path-fails

- **GIVEN** an active change directory remains after archive is claimed
- **WHEN** drain completion audit runs
- **THEN** it SHALL fail and print the remaining path

#### Scenario: Archived verification uses archive paths
ID: openspec-governance.drain-archive-cleanliness.archive-paths-used

- **GIVEN** a change has moved to `openspec/changes/archive/<date>-<name>`
- **WHEN** final preflight runs
- **THEN** task coverage and changed-file entries SHALL use current archive-relative paths

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
