## ADDED Requirements

### Requirement: Merge strategy enum

The system SHALL define a `MergeStrategy` enum with three variants: `MergeCommit`, `FastForwardOnly`, and `Squash`. The default strategy SHALL be `MergeCommit`.

#### Scenario: Parse strategy from string

- **WHEN** the string `"merge"` is parsed
- **THEN** the result SHALL be `MergeStrategy::MergeCommit`

#### Scenario: Parse fast-forward strategy

- **WHEN** the string `"fast-forward"` is parsed
- **THEN** the result SHALL be `MergeStrategy::FastForwardOnly`

#### Scenario: Parse squash strategy

- **WHEN** the string `"squash"` is parsed
- **THEN** the result SHALL be `MergeStrategy::Squash`

#### Scenario: Default strategy

- **WHEN** no strategy is specified (None)
- **THEN** the system SHALL use `MergeStrategy::MergeCommit`

#### Scenario: Invalid strategy string

- **WHEN** an unrecognized strategy string is provided
- **THEN** the system SHALL return an error

### Requirement: Merge commit strategy creates merge commit

When `MergeStrategy::MergeCommit` is used, `ForgeNode::merge_patch()` SHALL create a two-parent merge commit if the target has diverged from the patch base. If the target has not diverged, the system SHALL fast-forward the ref without creating a merge commit.

#### Scenario: Merge commit with diverged branches

- **GIVEN** patch P with base B and head H targets ref `heads/main`
- **AND** `heads/main` currently points to commit M where M ≠ B
- **AND** three-way merge of trees (B, M, H) produces no conflicts
- **WHEN** `merge_patch(repo, patch, MergeCommit)` is called
- **THEN** a merge commit SHALL be created with parents `[M, H]`
- **AND** the merge commit tree SHALL be the three-way merge result
- **AND** `heads/main` SHALL advance to the merge commit via CAS

#### Scenario: Merge commit with no divergence falls back to fast-forward

- **GIVEN** patch P with base B and head H targets ref `heads/main`
- **AND** `heads/main` currently points to B (no divergence)
- **WHEN** `merge_patch(repo, patch, MergeCommit)` is called
- **THEN** `heads/main` SHALL advance directly to H (fast-forward)
- **AND** no merge commit SHALL be created

### Requirement: Fast-forward-only strategy rejects diverged branches

When `MergeStrategy::FastForwardOnly` is used, `ForgeNode::merge_patch()` SHALL advance the ref to the patch head only if the target ref points to an ancestor of the patch head. If the target has diverged, the merge SHALL fail.

#### Scenario: Fast-forward succeeds when target is patch base

- **GIVEN** patch P with base B and head H targets ref `heads/main`
- **AND** `heads/main` currently points to B
- **WHEN** `merge_patch(repo, patch, FastForwardOnly)` is called
- **THEN** `heads/main` SHALL advance to H
- **AND** no merge commit SHALL be created

#### Scenario: Fast-forward fails when target has diverged

- **GIVEN** patch P with base B and head H targets ref `heads/main`
- **AND** `heads/main` currently points to M where M ≠ B
- **WHEN** `merge_patch(repo, patch, FastForwardOnly)` is called
- **THEN** the system SHALL return `ForgeError::FastForwardNotPossible`

### Requirement: Squash strategy creates single-parent commit

When `MergeStrategy::Squash` is used, `ForgeNode::merge_patch()` SHALL create a single-parent commit on the target branch with the merged tree content. The commit message SHALL be the custom message if provided, or a concatenation of all patch commit messages.

#### Scenario: Squash merge with diverged branches

- **GIVEN** patch P with base B and head H targets ref `heads/main`
- **AND** `heads/main` currently points to M
- **AND** three-way merge produces no conflicts
- **WHEN** `merge_patch(repo, patch, Squash)` is called
- **THEN** a commit SHALL be created with single parent M
- **AND** the commit tree SHALL be the three-way merge result
- **AND** `heads/main` SHALL advance to the squash commit via CAS

#### Scenario: Squash merge with no divergence

- **GIVEN** patch P with base B and head H targets ref `heads/main`
- **AND** `heads/main` currently points to B
- **WHEN** `merge_patch(repo, patch, Squash)` is called
- **THEN** a commit SHALL be created with single parent B
- **AND** the commit tree SHALL match H's tree

#### Scenario: Squash merge uses custom message

- **GIVEN** a squash merge with `custom_message = Some("feat: add auth")`
- **WHEN** the squash commit is created
- **THEN** the commit message SHALL be `"feat: add auth"`

#### Scenario: Squash merge auto-generates message

- **GIVEN** a squash merge with no custom message
- **AND** the patch has title "Add authentication"
- **WHEN** the squash commit is created
- **THEN** the commit message SHALL contain the patch title
