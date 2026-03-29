## ADDED Requirements

### Requirement: Tree objects round-trip with identical bytes

`TreeObject::new()` SHALL sort entries using git's mode-aware sort order. Directory entries (mode `040000`) SHALL be compared as if their name has `/` appended. This matches git's `base_name_compare` behavior.

#### Scenario: Tree with directory and file sharing name prefix

- **WHEN** a tree contains entries `foo` (mode 040000, directory) and `foo.c` (mode 100644, file)
- **THEN** `TreeObject::new()` sorts them in git order: `foo.c` before `foo` (because `'.'` < `'/'`)

#### Scenario: Tree import-export SHA-1 stability

- **WHEN** a tree is imported from git bytes, stored in Forge format, then exported back to git bytes
- **THEN** the exported bytes SHALL be identical to the original bytes, and the SHA-1 computed from exported bytes SHALL match the original SHA-1

#### Scenario: Standard tree entries sort identically

- **WHEN** a tree contains only file entries with distinct names (no directory entries sharing prefixes with file entries)
- **THEN** the sort order SHALL be identical to Rust's `str::cmp` order (no behavioral change for the common case)

### Requirement: Commit messages round-trip without loss

The commit import path SHALL preserve the raw message bytes between the blank-line separator and the end of the content. The export path SHALL reproduce those bytes exactly.

#### Scenario: Standard commit message round-trip

- **WHEN** a commit with message `"Fix bug\n"` is imported and exported
- **THEN** the exported commit content matches the original byte-for-byte

#### Scenario: Multi-paragraph commit message round-trip

- **WHEN** a commit with message `"Title\n\nBody paragraph.\n\nSigned-off-by: A\n"` is imported and exported
- **THEN** the exported commit content matches the original byte-for-byte

#### Scenario: Blob round-trip

- **WHEN** a blob is imported and exported
- **THEN** the exported content matches the original byte-for-byte and the SHA-1 matches

### Requirement: Round-trip test coverage

There SHALL be integration tests that import git objects (blob, tree with mode-aware sort edge cases, commit) through the full `GitImporter` → `GitExporter` pipeline and verify byte-identical output and SHA-1 stability.

#### Scenario: Round-trip test for trees with mixed modes

- **WHEN** a test imports a tree containing both directory and file entries with overlapping name prefixes
- **THEN** the test verifies that `export_tree()` produces bytes identical to the original import input, and the computed SHA-1 matches
