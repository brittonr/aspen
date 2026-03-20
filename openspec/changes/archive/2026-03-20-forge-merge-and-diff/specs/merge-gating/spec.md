## MODIFIED Requirements

### Requirement: Merge gating on CI status

The Forge SHALL reject `Merge` COB operations on protected refs when required CI contexts do not have `Success` status for the patch's head commit. The `MergeChecker` SHALL be invoked as part of the `merge_patch()` operation, not only as a standalone check.

#### Scenario: Merge blocked by failing CI

- **GIVEN** branch `heads/main` requires CI context `"ci/pipeline"`
- **AND** patch P targets `heads/main` with head commit `C`
- **AND** commit `C` has status `Failure` for context `"ci/pipeline"`
- **WHEN** `merge_patch(repo_id, patch_id)` is called
- **THEN** the merge SHALL be rejected with `ForgeError::MergeBlocked` indicating the required CI check failed
- **AND** no merge commit SHALL be created

#### Scenario: Merge blocked by missing CI status

- **GIVEN** branch `heads/main` requires CI context `"ci/pipeline"`
- **AND** patch P targets `heads/main` with head commit `C`
- **AND** commit `C` has no status entry for context `"ci/pipeline"`
- **WHEN** `merge_patch(repo_id, patch_id)` is called
- **THEN** the merge SHALL be rejected with `ForgeError::MergeBlocked` indicating the required CI check is missing

#### Scenario: Merge allowed with passing CI

- **GIVEN** branch `heads/main` requires CI context `"ci/pipeline"`
- **AND** patch P has head commit `C` with status `Success` for `"ci/pipeline"`
- **AND** patch P has the required number of approvals
- **WHEN** `merge_patch(repo_id, patch_id)` is called
- **THEN** the merge SHALL succeed and produce a merge commit

#### Scenario: Unprotected ref allows unconditional merge

- **GIVEN** branch `heads/feature` has no protection rule
- **WHEN** `merge_patch(repo_id, patch_id)` is called for a patch targeting `heads/feature`
- **THEN** the merge SHALL succeed regardless of CI status or approvals

### Requirement: Merge gating on approvals

The Forge SHALL reject `Merge` COB operations on protected refs when the patch does not have the required number of approvals for its current head commit. The approval check SHALL be performed within `merge_patch()`.

#### Scenario: Merge blocked by insufficient approvals

- **GIVEN** branch `heads/main` requires 2 approvals
- **AND** patch P has 1 approval for its current head commit
- **WHEN** `merge_patch(repo_id, patch_id)` is called
- **THEN** the merge SHALL be rejected with `ForgeError::MergeBlocked` indicating insufficient approvals

#### Scenario: Stale approvals dismissed on patch update

- **GIVEN** branch `heads/main` has `dismiss_stale_approvals: true`
- **AND** patch P has 2 approvals for head commit `C1`
- **WHEN** the patch author pushes a new revision with head commit `C2`
- **AND** `merge_patch()` is called
- **THEN** the merge SHALL be rejected because the approvals are for `C1`, not `C2`
