## ADDED Requirements

### Requirement: Sorted merge of branch and parent scan results

The scan merge function SHALL produce a lexicographically sorted sequence of key-value pairs by merging the branch's dirty entries with the parent's scan results. Branch entries SHALL take precedence over parent entries for the same key. Tombstoned keys SHALL be excluded from the output.

#### Scenario: Branch write overrides parent value

- **WHEN** the parent scan returns key "a" with value "parent-val"
- **AND** the branch has a dirty write for key "a" with value "branch-val"
- **THEN** the merged result SHALL contain key "a" with value "branch-val"
- **AND** the parent value SHALL NOT appear

#### Scenario: Tombstone removes parent key

- **WHEN** the parent scan returns key "b" with value "val"
- **AND** the branch has a tombstone for key "b"
- **THEN** the merged result SHALL NOT contain key "b"

#### Scenario: Branch-only keys are included

- **WHEN** the branch has a dirty write for key "c" with value "new"
- **AND** the parent scan does not contain key "c"
- **AND** key "c" matches the scan prefix
- **THEN** the merged result SHALL contain key "c" with value "new"

#### Scenario: Results are lexicographically sorted

- **WHEN** the parent scan returns keys ["a", "d", "f"]
- **AND** the branch has dirty writes for keys ["b", "e"]
- **THEN** the merged result SHALL be ordered ["a", "b", "d", "e", "f"]

#### Scenario: Limit is respected

- **WHEN** the merged result would contain 50 entries
- **AND** the scan limit is 10
- **THEN** the merged result SHALL contain exactly the first 10 entries in sorted order

### Requirement: Prefix filtering for branch entries

The scan merge function SHALL only include branch entries whose keys start with the scan prefix. Branch entries outside the prefix SHALL be ignored.

#### Scenario: Branch entries outside prefix are excluded

- **WHEN** the scan prefix is "config/"
- **AND** the branch has dirty writes for keys "config/db" and "users/admin"
- **THEN** the merged result SHALL include "config/db"
- **AND** the merged result SHALL NOT include "users/admin"

#### Scenario: Tombstones outside prefix are ignored

- **WHEN** the scan prefix is "config/"
- **AND** the branch has a tombstone for key "users/admin"
- **THEN** the tombstone SHALL have no effect on the merged result

### Requirement: Deterministic pure function

The scan merge logic SHALL be implemented as a deterministic pure function with no I/O, no async, and no time dependency. It SHALL be placed in `src/verified/` for Verus formal verification.

#### Scenario: Same inputs produce same output

- **WHEN** the merge function is called twice with identical inputs (branch entries, parent entries, prefix, limit)
- **THEN** both calls SHALL return identical results

### Requirement: Verus specification for scan merge correctness

Verus specs SHALL prove three properties: (1) no tombstoned key appears in output, (2) output is sorted, (3) branch entries take precedence over parent entries for duplicate keys.

#### Scenario: Tombstone exclusion proof

- **WHEN** the Verus spec for scan merge is verified
- **THEN** the `ensures` clause SHALL prove that for all keys in the output, no corresponding tombstone exists in the branch

#### Scenario: Sort order proof

- **WHEN** the Verus spec for scan merge is verified
- **THEN** the `ensures` clause SHALL prove that for all adjacent pairs (i, i+1) in the output, `output[i].key < output[i+1].key`

#### Scenario: Branch precedence proof

- **WHEN** the Verus spec for scan merge is verified
- **THEN** the `ensures` clause SHALL prove that if a key exists in both branch and parent, the output value equals the branch value
