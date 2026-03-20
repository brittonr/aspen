## ADDED Requirements

### Requirement: Structural tree diff

The system SHALL compare two `TreeObject`s and produce a list of `DiffEntry` records indicating which files were added, removed, or modified. The diff SHALL recurse into subdirectories. Entries with matching BLAKE3 hashes SHALL be omitted (unchanged). Entire subtrees SHALL be skipped when their root directory entry hashes match.

#### Scenario: Identical trees produce empty diff

- **GIVEN** tree A and tree B have the same BLAKE3 hash
- **WHEN** `diff_trees(A, B)` is called
- **THEN** the result SHALL be an empty list of `DiffEntry` records

#### Scenario: Added file detected

- **GIVEN** tree A has entries `["README.md"]` and tree B has entries `["README.md", "LICENSE"]`
- **AND** `README.md` has the same hash in both trees
- **WHEN** `diff_trees(A, B)` is called
- **THEN** the result SHALL contain exactly one `DiffEntry` with path `"LICENSE"` and kind `Added`

#### Scenario: Removed file detected

- **GIVEN** tree A has entries `["README.md", "LICENSE"]` and tree B has entries `["README.md"]`
- **AND** `README.md` has the same hash in both trees
- **WHEN** `diff_trees(A, B)` is called
- **THEN** the result SHALL contain exactly one `DiffEntry` with path `"LICENSE"` and kind `Removed`

#### Scenario: Modified file detected

- **GIVEN** tree A and tree B both contain entry `"main.rs"` but with different BLAKE3 hashes
- **WHEN** `diff_trees(A, B)` is called
- **THEN** the result SHALL contain a `DiffEntry` with path `"main.rs"` and kind `Modified`
- **AND** the entry SHALL include `old_hash` and `new_hash`

#### Scenario: Nested directory diff

- **GIVEN** tree A has directory `"src/"` containing `["lib.rs"]`
- **AND** tree B has directory `"src/"` containing `["lib.rs", "util.rs"]`
- **AND** `src/lib.rs` has the same hash in both
- **WHEN** `diff_trees(A, B)` is called
- **THEN** the result SHALL contain a `DiffEntry` with path `"src/util.rs"` and kind `Added`

#### Scenario: Unchanged subtree skipped

- **GIVEN** tree A and tree B both contain directory `"vendor/"` with the same BLAKE3 hash
- **WHEN** `diff_trees(A, B)` is called
- **THEN** no entries under `"vendor/"` SHALL appear in the result
- **AND** the `"vendor/"` subtree SHALL NOT be fetched from the blob store

#### Scenario: Mode change detected

- **GIVEN** tree A has `"deploy.sh"` with mode `0o100644` and tree B has `"deploy.sh"` with mode `0o100755`
- **AND** both entries have the same content hash
- **WHEN** `diff_trees(A, B)` is called
- **THEN** the result SHALL contain a `DiffEntry` with path `"deploy.sh"` and kind `Modified`
- **AND** the entry SHALL record `old_mode` as `0o100644` and `new_mode` as `0o100755`

### Requirement: Commit-to-commit diff

The system SHALL support diffing two commits by resolving each to its root tree and delegating to tree diff.

#### Scenario: Diff between parent and child commit

- **GIVEN** commit C1 with tree T1 and commit C2 (child of C1) with tree T2
- **WHEN** `diff_commits(C1, C2)` is called
- **THEN** the result SHALL be equivalent to `diff_trees(T1, T2)`

#### Scenario: Diff with content loading

- **GIVEN** commit C1 and C2 where file `"config.toml"` was modified
- **WHEN** `diff_commits(C1, C2)` is called with `include_content: true`
- **THEN** the `DiffEntry` for `"config.toml"` SHALL include `old_content` and `new_content` byte vectors

#### Scenario: Content loading skips oversized blobs

- **GIVEN** a modified file `"data.bin"` whose blob exceeds `MAX_DIFF_BLOB_SIZE` (1 MB)
- **WHEN** diff is computed with `include_content: true`
- **THEN** the `DiffEntry` SHALL have `old_content: None` and `new_content: None`
- **AND** the entry SHALL still report the file as `Modified` with correct hashes

### Requirement: Diff result ordering

The system SHALL return `DiffEntry` records sorted lexicographically by path. This ensures deterministic output regardless of tree traversal order.

#### Scenario: Entries sorted by path

- **GIVEN** a diff produces entries for paths `["src/z.rs", "README.md", "src/a.rs"]`
- **WHEN** the diff result is returned
- **THEN** the entries SHALL be ordered as `["README.md", "src/a.rs", "src/z.rs"]`

### Requirement: Diff resource bounds

The system SHALL enforce a maximum number of diff entries (`MAX_DIFF_ENTRIES`, default 10,000) to prevent unbounded computation on extremely large tree comparisons.

#### Scenario: Diff truncated at limit

- **GIVEN** two trees that differ in 15,000 files
- **WHEN** `diff_trees()` is called
- **THEN** the result SHALL contain at most `MAX_DIFF_ENTRIES` entries
- **AND** the result SHALL indicate truncation occurred
