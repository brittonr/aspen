# Forge Specification

## Purpose

Decentralized Git hosting built on Aspen's distributed primitives. Stores Git objects as content-addressed blobs via iroh-blobs, repository metadata in the Raft KV store, and supports bidirectional sync with external Git remotes via the `git-bridge` feature.

## Requirements

### Requirement: Repository Management

The system SHALL support creating, listing, and deleting Git repositories. Repository metadata SHALL be stored in the cluster's KV store.

#### Scenario: Create repository

- GIVEN an authenticated user with create permissions
- WHEN the user creates repository `"my-project"`
- THEN repository metadata SHALL be stored at `repos/<owner>/my-project` in the KV store
- AND the repository SHALL be ready to accept pushes

#### Scenario: List repositories

- GIVEN repositories `"alpha"`, `"beta"`, `"gamma"` exist
- WHEN a user lists repositories
- THEN all three SHALL be returned with their metadata

#### Scenario: Delete repository

- GIVEN repository `"old-project"` exists
- WHEN an administrator deletes it
- THEN the metadata SHALL be removed from the KV store
- AND the repository SHALL no longer accept operations

### Requirement: Git Object Storage

The system SHALL store Git objects (commits, trees, blobs, tags) as content-addressed blobs via iroh-blobs. Objects SHALL be retrievable by their Git SHA and distributable across cluster nodes.

#### Scenario: Push objects

- GIVEN a user pushes a commit with 10 new objects
- WHEN the push is received
- THEN each Git object SHALL be stored as an iroh-blob
- AND reference updates SHALL be committed through Raft consensus

#### Scenario: Fetch objects

- GIVEN a client performs a Git fetch
- WHEN the client requests objects it doesn't have
- THEN the forge SHALL serve them from iroh-blobs
- AND objects may be fetched from any node that has them

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
