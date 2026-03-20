## ADDED Requirements

### Requirement: Merge patch operation

The system SHALL provide a `ForgeNode::merge_patch()` method that accepts a `MergeStrategy` parameter, enforces branch protection, performs the tree merge according to the selected strategy, creates the appropriate commit, advances the target ref via Raft consensus, and transitions the patch COB to `Merged` state.

#### Scenario: Successful merge of approved patch

- **GIVEN** patch P targets `heads/main` with head commit CP
- **AND** `heads/main` currently points to commit CM
- **AND** branch protection is satisfied (CI passing, approvals met)
- **AND** the three-way merge of trees (CM, CM, CP) produces no conflicts
- **WHEN** `merge_patch(repo_id, patch_id, MergeCommit)` is called
- **THEN** a merge commit SHALL be created with parents `[CM, CP]`
- **AND** the merge commit's tree SHALL be the result of the three-way merge
- **AND** `heads/main` SHALL be atomically advanced to the merge commit via CAS
- **AND** the patch COB SHALL transition to `Merged` state with the merge commit hash

#### Scenario: Merge rejected by branch protection

- **GIVEN** patch P targets a protected branch
- **AND** required CI contexts are not passing
- **WHEN** `merge_patch(repo_id, patch_id, MergeCommit)` is called
- **THEN** the operation SHALL fail with `ForgeError::MergeBlocked`
- **AND** no merge commit SHALL be created
- **AND** the ref SHALL NOT be updated
- **AND** the patch state SHALL remain `Open`

#### Scenario: Merge rejected due to conflicts

- **GIVEN** patch P modifies file `"config.toml"` and the target branch also modified `"config.toml"` since the patch was created
- **AND** the three-way merge produces conflicts
- **WHEN** `merge_patch(repo_id, patch_id, MergeCommit)` is called
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
- **WHEN** `merge_patch(repo_id, patch_id, MergeCommit)` is called
- **THEN** the system SHALL detect that a fast-forward is possible
- **AND** SHALL advance the ref directly to the patch's head commit without creating a merge commit
- **AND** the patch COB SHALL transition to `Merged` with the patch's head commit hash

### Requirement: Merge commit message

The system SHALL generate a default merge commit message in the format `"Merge patch '<title>'"` where `<title>` is the patch's title. Callers MAY provide a custom message to override the default. For squash merges without a custom message, the commit message SHALL contain the patch title.

#### Scenario: Default merge message

- **GIVEN** patch P has title `"Fix OOM in batch processor"`
- **WHEN** `merge_patch()` is called without a custom message
- **THEN** the merge commit message SHALL be `"Merge patch 'Fix OOM in batch processor'"`

#### Scenario: Custom merge message

- **GIVEN** patch P has title `"Fix OOM"`
- **WHEN** `merge_patch()` is called with custom message `"Land the OOM fix for v2.1"`
- **THEN** the merge commit message SHALL be `"Land the OOM fix for v2.1"`

## ADDED Requirements

### Requirement: RPC handler delegates to ForgeNode::merge_patch

The `ForgeMergePatch` RPC handler SHALL call `ForgeNode::merge_patch()` (the full workflow) instead of `cobs.merge_patch()` (COB transition only). The RPC request SHALL accept an optional `strategy` string and optional `message` string, replacing the previous `merge_commit` field.

#### Scenario: RPC merge executes full workflow

- **WHEN** `ForgeMergePatch { repo_id, patch_id, strategy: Some("merge"), message: None }` is received
- **THEN** the handler SHALL call `ForgeNode::merge_patch()` with `MergeStrategy::MergeCommit`
- **AND** the response SHALL include the merge commit hash on success

#### Scenario: RPC merge with squash strategy

- **WHEN** `ForgeMergePatch { repo_id, patch_id, strategy: Some("squash"), message: Some("feat: auth") }` is received
- **THEN** the handler SHALL call `ForgeNode::merge_patch()` with `MergeStrategy::Squash` and the custom message

#### Scenario: RPC merge with no strategy defaults to merge commit

- **WHEN** `ForgeMergePatch { repo_id, patch_id, strategy: None, message: None }` is received
- **THEN** the handler SHALL use `MergeStrategy::MergeCommit`

### Requirement: CLI merge without pre-computed commit

The `aspen-cli patch merge` command SHALL NOT require a `--merge-commit` argument. The server SHALL compute the merge commit. The CLI SHALL accept an optional `--strategy` flag.

#### Scenario: CLI merge with default strategy

- **WHEN** `aspen-cli patch merge --repo <id> <patch-id>` is run
- **THEN** the CLI SHALL send `ForgeMergePatch` with `strategy: None`

#### Scenario: CLI merge with squash

- **WHEN** `aspen-cli patch merge --repo <id> <patch-id> --strategy squash` is run
- **THEN** the CLI SHALL send `ForgeMergePatch` with `strategy: Some("squash")`

#### Scenario: CLI merge with custom message

- **WHEN** `aspen-cli patch merge --repo <id> <patch-id> --message "ship it"` is run
- **THEN** the CLI SHALL send `ForgeMergePatch` with `message: Some("ship it")`
