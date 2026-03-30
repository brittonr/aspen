## ADDED Requirements

### Requirement: Federation import uses topological ordering

`federation_import_objects` SHALL import SyncObjects via `import_objects()` with topological sorting. After all batches are collected, it SHALL run a convergent retry loop rather than a single retry pass.

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

### Requirement: ImportResult carries per-object hash mappings

`ImportResult` SHALL include a `mappings` field of type `Vec<(Sha1Hash, blake3::Hash)>` containing the SHA-1 and BLAKE3 hash of every object processed (imported or already present).

#### Scenario: Fresh import of 5 objects

- **WHEN** `import_objects()` imports 5 new objects
- **THEN** `result.mappings` contains exactly 5 entries, each with the correct SHA-1 and BLAKE3 pair

#### Scenario: Import with some objects already present

- **WHEN** `import_objects()` receives 5 objects, 2 of which already have hash mappings
- **THEN** `result.mappings` contains 5 entries (3 newly imported + 2 existing), `objects_imported` is 3, `objects_skipped` is 2

### Requirement: Federated clone produces working git repo

A `git clone aspen://<ticket>/fed:<origin>:<repo>` SHALL produce a repository with correct refs and fetchable objects when the origin cluster is reachable, the repo is federated, and all objects are exported by the origin.

#### Scenario: Clone large repo with cross-batch dependencies

- **WHEN** origin cluster has a repo with 33,807 objects transferred across 17 batches, where trees reference blobs from different batches
- **THEN** `git clone` succeeds, HEAD resolves to a valid commit, and all tree entries are fetchable

#### Scenario: Clone single-branch repo via federation

- **WHEN** origin cluster has a federated repo with one branch (heads/main) pointing to a commit with a tree and two blobs
- **THEN** `git clone` succeeds, `git log` shows the commit, and `git ls-tree HEAD` shows both files

#### Scenario: Clone multi-branch repo via federation

- **WHEN** origin cluster has a federated repo with two branches (heads/main, heads/dev) pointing to different commits
- **THEN** `git clone` succeeds, both branches are listed by `git branch -r`, and each branch's commit is fetchable

#### Scenario: Origin cluster unreachable

- **WHEN** `git clone` is attempted with a `fed:` URL but the origin cluster is not reachable
- **THEN** git reports an error (not an empty repo)

### Requirement: Existing push path unaffected

All callers of `import_objects()` SHALL continue to work with the extended `ImportResult`. The `mappings` field is additive.

#### Scenario: Regular git push via git-remote-aspen

- **WHEN** a user pushes to a non-federated repo via `git push aspen main`
- **THEN** the push succeeds with the same behavior as before (objects imported, refs updated)
