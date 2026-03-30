## MODIFIED Requirements

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

### Requirement: Federation import uses topological ordering

`federation_import_objects` SHALL import SyncObjects via `import_objects()` with topological sorting. After all batches are collected, it SHALL run a convergent retry loop rather than a single retry pass.

#### Scenario: Post-sync retry converges to full import

- **WHEN** the batch-by-batch import leaves 2,902 objects unmapped due to cross-batch dependencies
- **THEN** the post-sync convergent retry imports all 2,902 objects across multiple passes

#### Scenario: Objects arrive in reverse dependency order

- **WHEN** federation sync returns SyncObjects in order [commit, tree, blob]
- **THEN** all three objects are imported successfully (blobs in wave 0, trees in wave 1, commits in wave 2)
