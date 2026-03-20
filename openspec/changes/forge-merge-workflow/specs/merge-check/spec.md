## ADDED Requirements

### Requirement: Pre-merge check endpoint

The system SHALL provide a `ForgeCheckMerge` RPC that evaluates mergeability of a patch without performing the merge. The check SHALL resolve the patch, evaluate branch protection rules, and attempt a tree merge to detect conflicts.

#### Scenario: Mergeable patch with all checks passing

- **GIVEN** patch P is open, targets `heads/main`
- **AND** branch protection requires 1 approval and CI context `ci/pipeline`
- **AND** patch has 1 approval and CI status is `Success` for `ci/pipeline`
- **AND** three-way merge produces no conflicts
- **WHEN** `ForgeCheckMerge { repo_id, patch_id }` is called
- **THEN** the response SHALL have `mergeable: true`
- **AND** `conflicts` SHALL be empty
- **AND** `available_strategies` SHALL contain `["merge", "fast-forward", "squash"]` or a subset based on branch state

#### Scenario: Patch blocked by missing approval

- **GIVEN** branch protection requires 2 approvals
- **AND** patch has 1 approval
- **WHEN** `ForgeCheckMerge` is called
- **THEN** `mergeable` SHALL be `false`
- **AND** `protection_status` SHALL indicate insufficient approvals

#### Scenario: Patch blocked by failing CI

- **GIVEN** branch protection requires CI context `ci/pipeline`
- **AND** CI status for the patch head is `Failure`
- **WHEN** `ForgeCheckMerge` is called
- **THEN** `mergeable` SHALL be `false`
- **AND** `protection_status` SHALL indicate failing CI context

#### Scenario: Patch has merge conflicts

- **GIVEN** patch and target have both modified the same file with different content
- **WHEN** `ForgeCheckMerge` is called
- **THEN** `mergeable` SHALL be `false`
- **AND** `conflicts` SHALL list the conflicting file paths

#### Scenario: Closed patch is not checkable

- **GIVEN** patch P is in `Closed` state
- **WHEN** `ForgeCheckMerge` is called
- **THEN** `mergeable` SHALL be `false`
- **AND** the response SHALL indicate the patch is not open

### Requirement: Available strategies reflect branch state

The check response SHALL indicate which merge strategies are available based on the current branch state.

#### Scenario: Fast-forward available when target has not diverged

- **GIVEN** `heads/main` points to the patch base commit
- **WHEN** `ForgeCheckMerge` is called
- **THEN** `available_strategies` SHALL include `"fast-forward"`

#### Scenario: Fast-forward unavailable when target has diverged

- **GIVEN** `heads/main` has advanced beyond the patch base
- **WHEN** `ForgeCheckMerge` is called
- **THEN** `available_strategies` SHALL NOT include `"fast-forward"`
- **AND** `available_strategies` SHALL include `"merge"` and `"squash"`

### Requirement: No side effects from check

The `ForgeCheckMerge` RPC SHALL NOT modify any state — no refs, no COB transitions, no objects written to storage.

#### Scenario: Check does not create objects

- **GIVEN** a mergeable patch
- **WHEN** `ForgeCheckMerge` is called
- **THEN** no new git objects (commits, trees) SHALL be written to the blob store
- **AND** no refs SHALL be modified
