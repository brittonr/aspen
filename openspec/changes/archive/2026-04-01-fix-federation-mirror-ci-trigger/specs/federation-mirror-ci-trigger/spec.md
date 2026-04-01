## ADDED Requirements

### Requirement: Manual CI trigger resolves config from federation-sourced repo

The `CiTriggerPipeline` RPC handler SHALL successfully locate and parse `.aspen/ci.ncl` from a commit tree that was populated via federation clone content pushed to a local Forge repo.

#### Scenario: ci run on repo with federation-cloned content

- **WHEN** a repo on bob's cluster contains commits imported via federation clone and local git push
- **AND** the commit tree includes `.aspen/ci.ncl` at the expected path
- **AND** a user runs `ci run <repo_id>`
- **THEN** `handle_trigger_pipeline` SHALL resolve the ref to a commit hash
- **AND** `walk_tree_for_file` SHALL find `.aspen/ci.ncl` in the commit's tree
- **AND** the Nickel config SHALL parse successfully
- **AND** a pipeline run SHALL be created via the orchestrator

#### Scenario: ci run on repo missing ci config

- **WHEN** a repo contains valid commits but no `.aspen/ci.ncl` file
- **AND** a user runs `ci run <repo_id>`
- **THEN** the handler SHALL return `is_success: false` with error "CI config file (.aspen/ci.ncl) not found in repository"

### Requirement: Tree walk logs reason when config not found

When `walk_tree_for_file` fails to find the requested file, the handler SHALL log which step failed: root tree lookup, intermediate directory entry missing, or leaf file entry missing.

#### Scenario: Root tree entry missing .aspen directory

- **WHEN** `walk_tree_for_file` is called with path `[".aspen", "ci.ncl"]`
- **AND** the root tree has no entry named `.aspen`
- **THEN** the handler SHALL log at `debug` level: the root tree hash and that `.aspen` was not found among the root entries

#### Scenario: .aspen directory exists but ci.ncl missing

- **WHEN** the root tree contains a `.aspen` directory entry
- **AND** the `.aspen` subtree has no entry named `ci.ncl`
- **THEN** the handler SHALL log at `debug` level: the `.aspen` tree hash and that `ci.ncl` was not found

### Requirement: Auto-trigger fires for watched repo on push

The `TriggerService` SHALL process `RefUpdate` announcements for any repo in its `watched_repos` set, regardless of whether the repo was added via `ci watch` RPC or via mirror auto-scan.

#### Scenario: Push to repo watched via ci watch RPC

- **WHEN** `ci watch <repo_id>` has been called
- **AND** a git push to that repo generates a `RefUpdate` gossip announcement
- **THEN** the `CiTriggerHandler` SHALL queue a trigger for processing
- **AND** the trigger SHALL NOT be dropped by the `federation_ci_enabled` gate (since the repo was manually watched, not auto-scanned as a mirror)

#### Scenario: Repo ID from ci watch matches gossip announcement

- **WHEN** a repo is watched via `ci watch` with hex-encoded repo ID
- **AND** the Forge gossip announces a `RefUpdate` for the same repo
- **THEN** the repo ID from the announcement SHALL match the watched set entry exactly
- **AND** the handler SHALL log the repo ID and match result at `info` level

### Requirement: Trigger handler logs watch mismatch diagnostic

When a `RefUpdate` announcement arrives for an unwatched repo, the handler SHALL log the announced repo ID and the current watched repo count, so operators can diagnose mismatches.

#### Scenario: Announcement for unwatched repo with non-empty watch set

- **WHEN** a `RefUpdate` arrives for repo X
- **AND** repo X is NOT in the watched set
- **AND** the watched set contains at least one repo
- **THEN** the handler SHALL log at `info` level: the announced repo ID (hex), the watched set size, and that the announcement was buffered

### Requirement: Regression test for CiTriggerPipeline on tree-walked config

The test suite SHALL include a test that exercises the full `handle_trigger_pipeline` path with a real `ForgeNode` containing a commit tree with `.aspen/ci.ncl`, verifying that the tree walk, Nickel parse, and pipeline creation succeed.

#### Scenario: Integration test with real tree walk

- **WHEN** a test creates a `ForgeNode`, imports a commit tree containing `.aspen/ci.ncl` with valid Nickel config, and calls `handle_trigger_pipeline`
- **THEN** the response SHALL have `is_success: true`
- **AND** the response SHALL contain a non-empty `run_id`
