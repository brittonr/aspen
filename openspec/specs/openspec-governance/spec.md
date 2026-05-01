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

### Requirement: Designs map requirements to verification rails
Design artifacts SHALL include a verification strategy that maps changed requirements to concrete checks before tasks are approved.

ID: openspec-governance.design-verification-strategy

#### Scenario: Design includes mapped verification strategy
ID: openspec-governance.design-verification-strategy.mapped-strategy-present

- **GIVEN** a change has delta specs
- **WHEN** the design gate runs
- **THEN** `design.md` SHALL include `## Verification Strategy`
- **AND** the strategy SHALL cite relevant requirement or scenario IDs

#### Scenario: Missing strategy fails design gate
ID: openspec-governance.design-verification-strategy.missing-strategy-fails

- **GIVEN** a change modifies specs but omits verification strategy
- **WHEN** the design gate runs
- **THEN** the gate SHALL fail with a design-stage finding

#### Scenario: Negative paths are planned
ID: openspec-governance.design-verification-strategy.negative-paths-planned

- **GIVEN** a delta spec defines rejection, failure, timeout, malformed, or unauthorized behavior
- **WHEN** design verification is reviewed
- **THEN** the strategy SHALL name a negative check or explicitly defer it with rationale

### Requirement: Drain verification cycle is explicit
OpenSpec drain completion SHALL record build, test, and format verification scope for implementation changes.

ID: openspec-governance.drain-verification-cycle

#### Scenario: Full verification cycle recorded
ID: openspec-governance.drain-verification-cycle.full-cycle-recorded

- **GIVEN** a drain implements code changes
- **WHEN** verification is finalized
- **THEN** `cargo build`, `cargo nextest run`, and `nix fmt` or project equivalents SHALL have status and artifact entries

#### Scenario: Scoped alternative recorded
ID: openspec-governance.drain-verification-cycle.scoped-alternative-recorded

- **GIVEN** full verification is too expensive or blocked
- **WHEN** scoped verification is used
- **THEN** the verification matrix SHALL include rationale, command, status, and next best check

#### Scenario: Doc-only bypass is explicit
ID: openspec-governance.drain-verification-cycle.doc-only-bypass-explicit

- **GIVEN** a drain changes only documentation or OpenSpec artifacts and no source code
- **WHEN** full build, nextest, or format checks are skipped
- **THEN** the verification matrix SHALL mark the rail as doc-only with rationale and changed-file evidence

#### Scenario: Final verification tasks require matrix first
ID: openspec-governance.drain-verification-cycle.final-verification-requires-matrix-first

- **GIVEN** a drain change is about to check the final verification task complete
- **WHEN** the verification matrix is missing or incomplete
- **THEN** preflight or done-review SHALL fail before completion is claimed

#### Scenario: Missing verification cycle fails
ID: openspec-governance.drain-verification-cycle.missing-cycle-fails

- **GIVEN** implementation tasks are checked
- **WHEN** no full, scoped, or explicit doc-only verification matrix exists
- **THEN** preflight or done-review SHALL fail

### Requirement: Proposals trace verification to changed requirements
Proposal verification sections SHALL identify how changed capabilities are expected to be verified.

ID: openspec-governance.proposal-verification-traceability

#### Scenario: Proposal cites changed requirement IDs
ID: openspec-governance.proposal-verification-traceability.cites-requirement-ids

- **GIVEN** a proposal has delta specs with requirement IDs
- **WHEN** proposal gate runs
- **THEN** the proposal verification section SHALL cite those IDs or explicitly defer them to design with rationale

#### Scenario: Missing positive verification fails
ID: openspec-governance.proposal-verification-traceability.missing-positive-verification-fails

- **GIVEN** an added requirement has no verification expectation
- **WHEN** proposal gate runs
- **THEN** it SHALL fail with the requirement ID

#### Scenario: Negative behavior needs verification expectation
ID: openspec-governance.proposal-verification-traceability.negative-behavior-needs-verification

- **GIVEN** a proposal or delta spec includes failure, rejection, unauthorized, malformed, or timeout behavior
- **WHEN** proposal verification is checked
- **THEN** at least one negative-path expectation SHALL be listed or explicitly deferred

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

### Requirement: Verification indexes are fresh
Verification indexes SHALL cite artifacts and paths that reflect the final reviewed state of the change.

ID: openspec-governance.verification-index-freshness

#### Scenario: Fresh verification index passes
ID: openspec-governance.verification-index-freshness.fresh-index-passes

- **GIVEN** `verification.md` cites current tracked artifacts and final preflight output
- **WHEN** preflight runs
- **THEN** it SHALL pass freshness checks

#### Scenario: Stale active path fails after archive
ID: openspec-governance.verification-index-freshness.stale-active-path-fails

- **GIVEN** an archived change's task coverage cites `openspec/changes/<name>` instead of the archive path
- **WHEN** preflight runs
- **THEN** it SHALL fail with the stale path

#### Scenario: Saved diff artifact may contain historical paths
ID: openspec-governance.verification-index-freshness.saved-diff-may-contain-historical-paths

- **GIVEN** a saved diff artifact contains old active change paths as historical diff context
- **WHEN** freshness validation scans archived evidence
- **THEN** the stale path SHALL be allowed only inside the saved diff artifact and not in current task coverage or verification artifact pointers

#### Scenario: Placeholder preflight fails
ID: openspec-governance.verification-index-freshness.placeholder-preflight-fails

- **GIVEN** a cited preflight artifact still contains placeholder text
- **WHEN** preflight runs
- **THEN** it SHALL fail before completion is claimed

#### Scenario: Evidence generated before final diff fails
ID: openspec-governance.verification-index-freshness.generated-before-final-diff-fails

- **GIVEN** a verification index cites a final-diff or preflight artifact captured before the final source or documentation edits
- **WHEN** freshness validation runs
- **THEN** it SHALL fail or require the artifact to be regenerated after staging the final reviewed state
