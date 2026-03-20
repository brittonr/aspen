## ADDED Requirements

### Requirement: Three-way tree merge

The system SHALL merge two trees against a common base tree using standard three-way merge semantics. For each entry name across the three trees, the system SHALL classify the change and produce either a merged entry or a conflict.

#### Scenario: Clean merge with non-overlapping changes

- **GIVEN** base tree has entries `["a.rs", "b.rs"]`
- **AND** ours modifies `"a.rs"` (different hash) and keeps `"b.rs"` unchanged
- **AND** theirs keeps `"a.rs"` unchanged and modifies `"b.rs"`
- **WHEN** `merge_trees(base, ours, theirs)` is called
- **THEN** the result SHALL be a merged tree with our version of `"a.rs"` and their version of `"b.rs"`
- **AND** `conflicts` SHALL be empty

#### Scenario: Conflict on both-modified

- **GIVEN** base tree has entry `"config.toml"` with hash H0
- **AND** ours changes it to H1 and theirs changes it to H2 (H1 ≠ H2)
- **WHEN** `merge_trees(base, ours, theirs)` is called
- **THEN** the result SHALL have no merged tree
- **AND** `conflicts` SHALL contain one entry for `"config.toml"` with kind `BothModified`

#### Scenario: Convergent change is not a conflict

- **GIVEN** base tree has entry `"shared.rs"` with hash H0
- **AND** both ours and theirs change it to the same hash H1
- **WHEN** `merge_trees(base, ours, theirs)` is called
- **THEN** the result SHALL include `"shared.rs"` with hash H1
- **AND** `conflicts` SHALL be empty

#### Scenario: Add on one side only

- **GIVEN** base tree does not contain `"new_file.rs"`
- **AND** ours adds `"new_file.rs"` with hash H1
- **AND** theirs does not add `"new_file.rs"`
- **WHEN** `merge_trees(base, ours, theirs)` is called
- **THEN** the merged tree SHALL include `"new_file.rs"` with hash H1

#### Scenario: Both add same file with different content

- **GIVEN** base tree does not contain `"new.rs"`
- **AND** ours adds `"new.rs"` with hash H1 and theirs adds `"new.rs"` with hash H2 (H1 ≠ H2)
- **WHEN** `merge_trees(base, ours, theirs)` is called
- **THEN** `conflicts` SHALL contain one entry for `"new.rs"` with kind `BothAdded`

#### Scenario: Delete on one side, unchanged on other

- **GIVEN** base tree has entry `"old.rs"` with hash H0
- **AND** ours removes `"old.rs"` and theirs keeps it with hash H0
- **WHEN** `merge_trees(base, ours, theirs)` is called
- **THEN** the merged tree SHALL NOT include `"old.rs"`

#### Scenario: Modify-delete conflict

- **GIVEN** base tree has entry `"lib.rs"` with hash H0
- **AND** ours modifies it to H1 and theirs removes it
- **WHEN** `merge_trees(base, ours, theirs)` is called
- **THEN** `conflicts` SHALL contain one entry for `"lib.rs"` with kind `ModifyDelete`

#### Scenario: Recursive subdirectory merge

- **GIVEN** base tree has directory `"src/"` containing `["a.rs", "b.rs"]`
- **AND** ours modifies `"src/a.rs"` and theirs modifies `"src/b.rs"`
- **WHEN** `merge_trees(base, ours, theirs)` is called
- **THEN** the merged tree SHALL contain directory `"src/"` with a new tree hash
- **AND** the merged `"src/"` subtree SHALL contain our `"a.rs"` and their `"b.rs"`

#### Scenario: Unchanged subtree skipped during merge

- **GIVEN** base, ours, and theirs all have directory `"vendor/"` with the same hash
- **WHEN** `merge_trees(base, ours, theirs)` is called
- **THEN** `"vendor/"` SHALL be included in the merged tree with its original hash
- **AND** the `"vendor/"` subtree SHALL NOT be fetched

### Requirement: Three-way classification is symmetric

The conflict detection SHALL be symmetric: `is_conflict(base, ours, theirs)` SHALL equal `is_conflict(base, theirs, ours)`. This SHALL be verified by Verus formal specification.

#### Scenario: Symmetry of conflict detection

- **GIVEN** any combination of base hash B, ours hash O, theirs hash T
- **WHEN** `is_conflict(B, O, T)` and `is_conflict(B, T, O)` are evaluated
- **THEN** both SHALL return the same boolean result

### Requirement: Merge result invariants

A successful (conflict-free) merge SHALL produce a tree where every entry is present in at least one of the input trees. No merge SHALL invent entries not present in base, ours, or theirs.

#### Scenario: No phantom entries after merge

- **GIVEN** base tree has entries `["a", "b"]`, ours has `["a", "b", "c"]`, theirs has `["a", "b", "d"]`
- **WHEN** `merge_trees(base, ours, theirs)` succeeds without conflicts
- **THEN** the merged tree SHALL contain only entries from `{"a", "b", "c", "d"}`
- **AND** no other entries SHALL be present

### Requirement: Merge resource bounds

The system SHALL enforce a maximum recursion depth (`MAX_MERGE_DEPTH`, default 64) for nested directory merges and a maximum number of conflict entries (`MAX_MERGE_CONFLICTS`, default 1,000).

#### Scenario: Merge aborted at depth limit

- **GIVEN** trees with directory nesting deeper than `MAX_MERGE_DEPTH`
- **WHEN** `merge_trees()` is called
- **THEN** the merge SHALL return an error indicating the depth limit was exceeded

#### Scenario: Conflict accumulation capped

- **GIVEN** a merge that produces more than `MAX_MERGE_CONFLICTS` conflicting entries
- **WHEN** `merge_trees()` is called
- **THEN** the result SHALL contain at most `MAX_MERGE_CONFLICTS` conflict entries
- **AND** the result SHALL indicate that additional conflicts were truncated
