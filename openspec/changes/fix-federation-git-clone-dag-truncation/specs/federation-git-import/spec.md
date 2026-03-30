## MODIFIED Requirements

### Requirement: Federation import uses topological ordering

`federation_import_objects` SHALL import SyncObjects via `import_objects()` with topological sorting. After all batches are collected, it SHALL run a convergent retry loop rather than a single retry pass. After each convergent pass, it SHALL store origin SHA-1 → BLAKE3 mappings for newly imported objects so that subsequent passes can resolve tree entries that reference sub-objects by their original SHA-1. After the final retry pass, it SHALL run a DAG re-resolution pass that verifies and fixes stale BLAKE3 references in imported trees.

#### Scenario: Post-sync retry converges to full import

- **WHEN** the batch-by-batch import leaves 2,902 objects unmapped due to cross-batch dependencies
- **THEN** the post-sync convergent retry imports all 2,902 objects across multiple passes

#### Scenario: Objects arrive in reverse dependency order

- **WHEN** federation sync returns SyncObjects in order [commit, tree, blob]
- **THEN** all three objects are imported successfully (blobs in wave 0, trees in wave 1, commits in wave 2)

#### Scenario: Objects arrive in dependency order

- **WHEN** federation sync returns SyncObjects in order [blob, tree, commit]
- **THEN** all three objects are imported successfully

#### Scenario: Tree references multiple blobs

- **WHEN** a tree SyncObject references 3 blob SyncObjects
- **THEN** all blobs are imported before the tree, and the tree's SHA-1→BLAKE3 entry mappings resolve correctly

#### Scenario: Origin SHA-1 stored between convergent passes

- **WHEN** subtree S is imported in pass 1 with re-serialized SHA-1 Y and origin SHA-1 X (where X ≠ Y)
- **THEN** the mapping store contains `X → blake3(S)` before pass 2 begins
- **AND** parent tree T that references S by SHA-1 X resolves successfully in pass 2

#### Scenario: DAG re-resolution fixes stale references

- **WHEN** tree T was imported in round 3 with entry `blake3=H1` for sub-object S, but S's current envelope hash in the mirror is H2
- **THEN** the post-import re-resolution updates T's entry to `blake3=H2`
- **AND** the mirror's DAG is fully traversable from HEAD
