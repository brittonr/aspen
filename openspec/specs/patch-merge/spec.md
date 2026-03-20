## ADDED Requirements

### Requirement: Merge patch operation

The system SHALL provide a `ForgeNode::merge_patch()` method that enforces branch protection, performs the tree merge, creates a merge commit, advances the target ref via Raft consensus, and transitions the patch COB to `Merged` state.

#### Scenario: Successful merge of approved patch

- **GIVEN** patch P targets `heads/main` with head commit CP
- **AND** `heads/main` currently points to commit CM
- **AND** branch protection is satisfied (CI passing, approvals met)
- **AND** the three-way merge of trees (CM, CM, CP) produces no conflicts
- **WHEN** `merge_patch(repo_id, patch_id)` is called
- **THEN** a merge commit SHALL be created with parents `[CM, CP]`
- **AND** the merge commit's tree SHALL be the result of the three-way merge
- **AND** `heads/main` SHALL be atomically advanced to the merge commit via CAS
- **AND** the patch COB SHALL transition to `Merged` state with the merge commit hash

#### Scenario: Merge rejected by branch protection

- **GIVEN** patch P targets a protected branch
- **AND** required CI contexts are not passing
- **WHEN** `merge_patch(repo_id, patch_id)` is called
- **THEN** the operation SHALL fail with `ForgeError::MergeBlocked`
- **AND** no merge commit SHALL be created
- **AND** the ref SHALL NOT be updated
- **AND** the patch state SHALL remain `Open`

#### Scenario: Merge rejected due to conflicts

- **GIVEN** patch P modifies file `"config.toml"` and the target branch also modified `"config.toml"` since the patch was created
- **AND** the three-way merge produces conflicts
- **WHEN** `merge_patch(repo_id, patch_id)` is called
- **THEN** the operation SHALL fail with `ForgeError::MergeConflicts`
- **AND** the error SHALL include the list of `MergeConflict` entries
- **AND** no merge commit SHALL be created

#### Scenario: CAS failure triggers retry

- **GIVEN** patch P is ready to merge
- **AND** another push advances `heads/main` between the merge check and the CAS write
- **WHEN** `merge_patch()` attempts to advance the ref
- **THEN** the CAS SHALL fail
- **AND** the operation SHALL retry the merge with the new base (up to 3 attempts)

#### Scenario: Fast-forward merge when possible

- **GIVEN** patch P's base commit equals the current target ref (no divergence)
- **WHEN** `merge_patch(repo_id, patch_id)` is called
- **THEN** the system SHALL detect that a fast-forward is possible
- **AND** SHALL advance the ref directly to the patch's head commit without creating a merge commit
- **AND** the patch COB SHALL transition to `Merged` with the patch's head commit hash

### Requirement: Patch merged state

The `PatchState` enum SHALL include a `Merged` variant recording the merge commit hash, the public key of the user who merged, and a timestamp. `Merged` SHALL be a terminal state with no further transitions.

#### Scenario: Patch transitions to merged

- **WHEN** a patch is successfully merged
- **THEN** the patch state SHALL be `Merged`
- **AND** the state SHALL contain the merge commit's BLAKE3 hash
- **AND** resolving the patch SHALL show state `Merged` with the merge details

#### Scenario: No operations on merged patch

- **GIVEN** patch P is in state `Merged`
- **WHEN** a user attempts to add a comment, push a revision, or close the patch
- **THEN** the operation SHALL fail with an error indicating the patch is already merged

### Requirement: Merge commit message

The system SHALL generate a default merge commit message in the format `"Merge patch '<title>'"` where `<title>` is the patch's title. Callers MAY provide a custom message to override the default.

#### Scenario: Default merge message

- **GIVEN** patch P has title `"Fix OOM in batch processor"`
- **WHEN** `merge_patch()` is called without a custom message
- **THEN** the merge commit message SHALL be `"Merge patch 'Fix OOM in batch processor'"`

#### Scenario: Custom merge message

- **GIVEN** patch P has title `"Fix OOM"`
- **WHEN** `merge_patch()` is called with custom message `"Land the OOM fix for v2.1"`
- **THEN** the merge commit message SHALL be `"Land the OOM fix for v2.1"`

### Requirement: Merge emits gossip announcement

After a successful merge, the system SHALL broadcast a `RefUpdate` gossip announcement for the target branch, allowing other nodes to sync the new merge commit.

#### Scenario: Gossip after merge

- **GIVEN** gossip is enabled on the forge node
- **WHEN** a patch is successfully merged to `heads/main`
- **THEN** a `RefUpdate` announcement SHALL be broadcast with the new commit hash
- **AND** subscribed peers SHALL receive the announcement
