## MODIFIED Requirements

### Requirement: Federation import uses topological ordering

`federation_import_objects` SHALL import SyncObjects via `import_objects()` with topological sorting. After all batches are collected, it SHALL run a convergent retry loop rather than a single retry pass. For each successfully imported object whose `SyncObject.envelope_hash` is present, it SHALL also write an origin→mirror BLAKE3 remap entry to KV.

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

#### Scenario: Remap entries written alongside SHA-1 mappings

- **WHEN** `federation_import_objects` imports 1,000 objects, each with `envelope_hash` set
- **THEN** 1,000 remap entries exist in KV at `forge:remap:{repo}:{envelope_hash_hex}`
