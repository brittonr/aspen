# Forge Specification

## Purpose

Decentralized Git hosting built on Aspen's distributed primitives. Stores Git objects as content-addressed blobs via iroh-blobs, repository metadata in the Raft KV store, and supports bidirectional sync with external Git remotes via the `git-bridge` feature.
## Requirements
### Requirement: Repository Management

The system SHALL support creating, listing, and deleting repositories. Repository metadata SHALL be stored in the cluster's KV store and SHALL record which repository backends are enabled for that repo, including Git, JJ, or both.

#### Scenario: Create repository

- GIVEN an authenticated user with create permissions
- WHEN the user creates repository `"my-project"`
- THEN repository metadata SHALL be stored at `repos/<owner>/my-project` in the KV store
- AND the repository SHALL be ready to accept operations for its configured backends

#### Scenario: Create JJ-enabled repository

- GIVEN an authenticated user with create permissions
- WHEN the user creates repository `"my-project"` with JJ support enabled
- THEN the repo metadata SHALL record JJ as an enabled backend
- AND Forge SHALL allocate JJ-specific state namespaces for bookmarks and change-id indexes

#### Scenario: List repositories

- GIVEN repositories `"alpha"`, `"beta"`, `"gamma"` exist
- WHEN a user lists repositories
- THEN all three SHALL be returned with their metadata
- AND each repo entry SHALL report its enabled backends

#### Scenario: Repository capability discovery

- GIVEN a repository with Git and JJ support enabled
- WHEN a client queries the repository metadata surface before choosing a transport path for a target node or eligible node set
- THEN Forge SHALL report the enabled backends for that repo
- AND the response SHALL include routing identifiers only for nodes that currently advertise the selected backend
- AND nodes without an active backend implementation SHALL not advertise routing identifiers for that backend

#### Scenario: Delete repository

- GIVEN repository `"old-project"` exists
- WHEN an administrator deletes it
- THEN the metadata SHALL be removed from the KV store
- AND the repository SHALL no longer accept operations from any backend
- AND repo-scoped JJ bookmark namespaces, change-id indexes, and discovery metadata for that repo SHALL be removed or tombstoned so they are no longer reachable through normal repo operations

### Requirement: Git Object Storage

The system SHALL store Git objects (commits, trees, blobs, tags) as content-addressed blobs via iroh-blobs. Objects SHALL be retrievable by their Git SHA and distributable across cluster nodes. The system SHALL support reading individual files from the tree at a specific commit without full checkout.

#### Scenario: Push objects

- **WHEN** a user pushes a commit with 10 new objects
- **THEN** each Git object SHALL be stored as an iroh-blob
- **AND** reference updates SHALL be committed through Raft consensus

#### Scenario: Fetch objects

- **WHEN** a client performs a Git fetch
- **THEN** the forge SHALL serve them from iroh-blobs
- **AND** objects may be fetched from any node that has them

#### Scenario: Read file at commit

- **WHEN** `read_file_at_commit(repo_id, commit_hash, "path/to/file")` is called
- **THEN** the system SHALL resolve the commit object, walk its tree to the given path, and return the blob contents
- **AND** return `None` if the path does not exist in the tree

#### Scenario: Push emits hook event

- **WHEN** a git push successfully updates refs on a repository
- **THEN** the system SHALL emit a `ForgePushCompleted` hook event with repo_id, ref_name, new_hash, and old_hash
- **AND** hook subscribers SHALL receive the event

#### Scenario: Push emits gossip announcement and commit status query support

- **WHEN** a git push updates refs
- **THEN** a `RefUpdate` gossip announcement SHALL be broadcast
- **AND** other nodes SHALL be able to query commit statuses for the pushed commit hash

#### Scenario: Diff between two commits

- **WHEN** `diff_commits(commit_a, commit_b)` is called
- **THEN** the system SHALL resolve both commits to their root trees and produce a list of `DiffEntry` records
- **AND** the result SHALL be equivalent to `diff_trees(tree_a, tree_b)`

#### Scenario: Merge patch via ForgeNode

- **WHEN** `merge_patch(repo_id, patch_id)` is called on ForgeNode
- **THEN** the system SHALL enforce branch protection, perform three-way tree merge, create a merge commit, advance the target ref, and transition the patch to `Merged` state

<!-- Synced from forge-merge-and-diff change (2026-03-20) -->

### Requirement: Git Remote Helper

The system SHALL provide a `git-remote-aspen` binary that implements the Git remote helper protocol, allowing standard Git clients to push/pull using `aspen://` URLs.

#### Scenario: Clone via remote helper

- GIVEN the `git-remote-aspen` binary is in the user's PATH
- WHEN the user runs `git clone aspen://<endpoint-id>/my-project`
- THEN the repository SHALL be cloned over iroh QUIC
- AND the working tree SHALL be identical to the original

#### Scenario: Push via remote helper

- GIVEN a local repository with `aspen://` remote configured
- WHEN the user runs `git push`
- THEN new objects SHALL be sent over iroh QUIC
- AND reference updates SHALL be committed through Raft

### Requirement: Git Bridge (Bidirectional Sync)

The system SHALL support bidirectional synchronization with external Git remotes (GitHub, GitLab) via the `git-bridge` feature.

#### Scenario: Mirror from GitHub

- GIVEN a bridge configured to mirror `github.com/org/repo`
- WHEN new commits are pushed to GitHub
- THEN the forge SHALL pull and store them within the configured sync interval

#### Scenario: Push to external remote

- GIVEN a bridge configured for bidirectional sync
- WHEN new commits are pushed to the forge
- THEN the forge SHALL push them to the external remote

<!-- Merged from forge-ci-integration -->

### Requirement: Git Object Storage

The system SHALL store Git objects (commits, trees, blobs, tags) as content-addressed blobs via iroh-blobs. Objects SHALL be retrievable by their Git SHA and distributable across cluster nodes. The system SHALL support reading individual files from the tree at a specific commit without full checkout.

#### Scenario: Push objects

- **WHEN** a user pushes a commit with 10 new objects
- **THEN** each Git object SHALL be stored as an iroh-blob
- **AND** reference updates SHALL be committed through Raft consensus

#### Scenario: Fetch objects

- **WHEN** a client performs a Git fetch
- **THEN** the forge SHALL serve them from iroh-blobs
- **AND** objects may be fetched from any node that has them

#### Scenario: Read file at commit

- **WHEN** `read_file_at_commit(repo_id, commit_hash, "path/to/file")` is called
- **THEN** the system SHALL resolve the commit object, walk its tree to the given path, and return the blob contents
- **AND** return `None` if the path does not exist in the tree

#### Scenario: Push emits hook event

- **WHEN** a git push successfully updates refs on a repository
- **THEN** the system SHALL emit a `ForgePushCompleted` hook event with repo_id, ref_name, new_hash, and old_hash
- **AND** hook subscribers SHALL receive the event

#### Scenario: Push emits gossip announcement and commit status query support

- **WHEN** a git push updates refs
- **THEN** a `RefUpdate` gossip announcement SHALL be broadcast
- **AND** other nodes SHALL be able to query commit statuses for the pushed commit hash

#### Scenario: Diff between two commits

- **WHEN** `diff_commits(commit_a, commit_b)` is called
- **THEN** the system SHALL resolve both commits to their root trees and produce a list of `DiffEntry` records
- **AND** the result SHALL be equivalent to `diff_trees(tree_a, tree_b)`

#### Scenario: Merge patch via ForgeNode

- **WHEN** `merge_patch(repo_id, patch_id)` is called on ForgeNode
- **THEN** the system SHALL enforce branch protection, perform three-way tree merge, create a merge commit, advance the target ref, and transition the patch to `Merged` state

<!-- Synced from forge-merge-and-diff change (2026-03-20) -->

### Requirement: Git Object Storage

The system SHALL store Git objects (commits, trees, blobs, tags) as content-addressed blobs via iroh-blobs. Objects SHALL be retrievable by their Git SHA and distributable across cluster nodes. The system SHALL support reading individual files from the tree at a specific commit without full checkout.

#### Scenario: Push objects

- **WHEN** a user pushes a commit with 10 new objects
- **THEN** each Git object SHALL be stored as an iroh-blob
- **AND** reference updates SHALL be committed through Raft consensus

#### Scenario: Fetch objects

- **WHEN** a client performs a Git fetch
- **THEN** the forge SHALL serve them from iroh-blobs
- **AND** objects may be fetched from any node that has them

#### Scenario: Read file at commit

- **WHEN** `read_file_at_commit(repo_id, commit_hash, "path/to/file")` is called
- **THEN** the system SHALL resolve the commit object, walk its tree to the given path, and return the blob contents
- **AND** return `None` if the path does not exist in the tree

#### Scenario: Push emits hook event

- **WHEN** a git push successfully updates refs on a repository
- **THEN** the system SHALL emit a `ForgePushCompleted` hook event with repo_id, ref_name, new_hash, and old_hash
- **AND** hook subscribers SHALL receive the event

#### Scenario: Push emits gossip announcement and commit status query support

- **WHEN** a git push updates refs
- **THEN** a `RefUpdate` gossip announcement SHALL be broadcast
- **AND** other nodes SHALL be able to query commit statuses for the pushed commit hash

#### Scenario: Diff between two commits

- **WHEN** `diff_commits(commit_a, commit_b)` is called
- **THEN** the system SHALL resolve both commits to their root trees and produce a list of `DiffEntry` records
- **AND** the result SHALL be equivalent to `diff_trees(tree_a, tree_b)`

#### Scenario: Merge patch via ForgeNode

- **WHEN** `merge_patch(repo_id, patch_id)` is called on ForgeNode
- **THEN** the system SHALL enforce branch protection, perform three-way tree merge, create a merge commit, advance the target ref, and transition the patch to `Merged` state

<!-- Synced from forge-merge-and-diff change (2026-03-20) -->

