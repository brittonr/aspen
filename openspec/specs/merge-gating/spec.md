## ADDED Requirements

### Requirement: Branch protection configuration

The Forge SHALL support branch protection rules stored in the KV store at `forge:protection:{repo_hex}:{ref_pattern}`. Each rule SHALL specify a ref pattern, required approval count, required CI contexts, and whether to dismiss stale approvals.

#### Scenario: Create branch protection rule

- **WHEN** an administrator creates a protection rule for ref pattern `heads/main` on repo `R` with `required_approvals: 2` and `required_ci_contexts: ["ci/pipeline"]`
- **THEN** the rule SHALL be stored at `forge:protection:{R_hex}:heads/main`

#### Scenario: Update existing rule

- **WHEN** an administrator updates the protection rule to add `"ci/deploy"` to required contexts
- **THEN** the stored rule SHALL reflect the updated contexts list

#### Scenario: Delete protection rule

- **WHEN** an administrator removes the protection rule for `heads/main`
- **THEN** the key SHALL be deleted from the KV store
- **AND** merges to `heads/main` SHALL no longer be gated

### Requirement: Merge gating on CI status

The Forge SHALL reject `Merge` COB operations on protected refs when required CI contexts do not have `Success` status for the patch's head commit.

#### Scenario: Merge blocked by failing CI

- **GIVEN** branch `heads/main` requires CI context `"ci/pipeline"`
- **AND** patch P targets `heads/main` with head commit `C`
- **AND** commit `C` has status `Failure` for context `"ci/pipeline"`
- **WHEN** a user attempts to merge patch P
- **THEN** the merge SHALL be rejected with an error indicating the required CI check failed

#### Scenario: Merge blocked by missing CI status

- **GIVEN** branch `heads/main` requires CI context `"ci/pipeline"`
- **AND** patch P targets `heads/main` with head commit `C`
- **AND** commit `C` has no status entry for context `"ci/pipeline"`
- **WHEN** a user attempts to merge patch P
- **THEN** the merge SHALL be rejected with an error indicating the required CI check is missing

#### Scenario: Merge allowed with passing CI

- **GIVEN** branch `heads/main` requires CI context `"ci/pipeline"`
- **AND** patch P has head commit `C` with status `Success` for `"ci/pipeline"`
- **AND** patch P has the required number of approvals
- **WHEN** a user merges patch P
- **THEN** the merge SHALL succeed

#### Scenario: Unprotected ref allows unconditional merge

- **GIVEN** branch `heads/feature` has no protection rule
- **WHEN** a user merges a patch targeting `heads/feature`
- **THEN** the merge SHALL succeed regardless of CI status or approvals

### Requirement: Merge gating on approvals

The Forge SHALL reject `Merge` COB operations on protected refs when the patch does not have the required number of approvals for its current head commit.

#### Scenario: Merge blocked by insufficient approvals

- **GIVEN** branch `heads/main` requires 2 approvals
- **AND** patch P has 1 approval for its current head commit
- **WHEN** a user attempts to merge patch P
- **THEN** the merge SHALL be rejected with an error indicating insufficient approvals

#### Scenario: Stale approvals dismissed on patch update

- **GIVEN** branch `heads/main` has `dismiss_stale_approvals: true`
- **AND** patch P has 2 approvals for head commit `C1`
- **WHEN** the patch author pushes a new revision with head commit `C2`
- **AND** a user attempts to merge
- **THEN** the merge SHALL be rejected because the approvals are for `C1`, not `C2`

### Requirement: Patch CI status resolution

When resolving a patch's display state, the Forge SHALL query commit statuses for the patch's current head commit and include the aggregated CI result alongside human review status.

#### Scenario: Patch with passing CI

- **GIVEN** patch P has head commit `C`
- **AND** commit `C` has status `Success` for context `"ci/pipeline"`
- **WHEN** the patch is resolved for display
- **THEN** the resolved patch SHALL include CI status `Success` with context `"ci/pipeline"`

#### Scenario: Patch with no CI activity

- **GIVEN** patch P has head commit `C`
- **AND** commit `C` has no commit statuses
- **WHEN** the patch is resolved for display
- **THEN** the resolved patch SHALL indicate no CI statuses are present
