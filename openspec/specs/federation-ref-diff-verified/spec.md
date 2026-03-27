## ADDED Requirements

### Requirement: Verus spec for compute_ref_diff

The `compute_ref_diff` function SHALL have a Verus formal specification in `verus/ref_diff_spec.rs` proving partition and completeness properties.

#### Scenario: Output categories partition the ref namespace

- **WHEN** `compute_ref_diff` is called with any local and remote ref sets
- **THEN** every ref name in the union of local and remote keys SHALL appear in exactly one output category (to_pull, to_push, in_sync, or conflicts)

#### Scenario: Pull category contains remote-only refs

- **WHEN** a ref exists in remote but not in local
- **THEN** that ref SHALL appear in `to_pull` and no other category

#### Scenario: Push category contains local-only refs

- **WHEN** a ref exists in local but not in remote
- **THEN** that ref SHALL appear in `to_push` and no other category

#### Scenario: In-sync category for matching hashes

- **WHEN** a ref exists in both local and remote with identical hashes
- **THEN** that ref SHALL appear in `in_sync` and no other category

#### Scenario: Conflicts for divergent hashes

- **WHEN** a ref exists in both local and remote with different hashes
- **THEN** that ref SHALL appear in `conflicts` and no other category

#### Scenario: Empty inputs produce empty output

- **WHEN** both local and remote ref sets are empty
- **THEN** all four output categories SHALL be empty

### Requirement: Verus spec for resolve_conflicts

The `resolve_conflicts` function SHALL have a Verus formal specification proving that conflict resolution preserves the partition invariant and moves conflicts to the correct target category.

#### Scenario: Pull-wins resolution moves conflicts to to_pull

- **WHEN** `resolve_conflicts` is called with `pull_wins = true`
- **THEN** all former conflict refs SHALL appear in `to_pull`
- **AND** `conflicts` SHALL be empty afterward

#### Scenario: Push-wins resolution moves conflicts to to_push

- **WHEN** `resolve_conflicts` is called with `pull_wins = false`
- **THEN** all former conflict refs SHALL appear in `to_push`
- **AND** `conflicts` SHALL be empty afterward

#### Scenario: No conflicts is a no-op

- **WHEN** `resolve_conflicts` is called with an empty conflicts list
- **THEN** all other categories SHALL remain unchanged
