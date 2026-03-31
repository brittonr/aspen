## ADDED Requirements

### Requirement: Gitlink entries preserved during tree import

Tree import MUST preserve gitlink entries (mode 160000) in the resulting `TreeObject`. The original 20-byte SHA-1 hash MUST be stored zero-padded in the 32-byte hash field.

#### Scenario: Import tree containing gitlink

- **WHEN** a git tree object containing a mode 160000 entry is imported
- **THEN** the resulting `TreeObject.entries` includes a `TreeEntry` with `mode == 0o160000`, the original entry name, and the SHA-1 bytes in `hash[..20]` with `hash[20..32]` zeroed

#### Scenario: Import tree with mixed entry types

- **WHEN** a git tree contains blobs, subtrees, and gitlinks
- **THEN** all entry types are preserved and the entry count matches the original

### Requirement: Gitlink entries exported with correct SHA-1

Tree export MUST reconstruct gitlink entries using the stored SHA-1 bytes directly, not via BLAKE3→SHA-1 mapping lookup.

#### Scenario: Export tree containing gitlink

- **WHEN** a `TreeObject` containing a gitlink entry is exported to git format
- **THEN** the gitlink entry's 20-byte SHA-1 is written from `hash[..20]` and the mode is emitted as `160000`

#### Scenario: Exported tree SHA-1 matches original

- **WHEN** a tree with gitlinks is imported then exported
- **THEN** the exported tree content is byte-identical to the original, producing the same SHA-1

### Requirement: DAG walk skips gitlink references

The DAG traversal MUST NOT attempt to resolve gitlink entry hashes as objects in the current repository.

#### Scenario: DAG walk encounters gitlink in tree

- **WHEN** the DAG walker processes a tree containing a gitlink entry
- **THEN** the gitlink hash is not added to the traversal queue

### Requirement: TreeEntry provides gitlink helpers

`TreeEntry` MUST provide methods to construct, identify, and extract SHA-1 from gitlink entries.

#### Scenario: Construct gitlink entry

- **WHEN** `TreeEntry::gitlink(name, sha1_bytes)` is called
- **THEN** the entry has `mode == 0o160000` and `hash[..20] == sha1_bytes` and `hash[20..32] == [0; 12]`

#### Scenario: Identify gitlink entry

- **WHEN** `is_gitlink()` is called on an entry with mode 160000
- **THEN** it returns true

#### Scenario: Extract SHA-1 from gitlink

- **WHEN** `gitlink_sha1_bytes()` is called on a gitlink entry
- **THEN** it returns the first 20 bytes of the hash field
