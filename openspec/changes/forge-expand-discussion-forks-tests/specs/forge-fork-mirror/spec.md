## ADDED Requirements

### Requirement: Repository forking

The system SHALL support forking a repository, creating a new repository linked to the upstream with a copy of all current refs and shared object history.

#### Scenario: Fork a repository

- **WHEN** a user forks repo `upstream` with name `my-fork`
- **THEN** a new repo `my-fork` SHALL be created with `fork_info.upstream_repo_id` set to `upstream`'s RepoId
- AND all refs from `upstream` SHALL be copied to the fork
- AND the fork's delegates SHALL be set to the forking user
- AND Git objects SHALL NOT be duplicated (shared via iroh-blobs content addressing)

#### Scenario: Fork with custom delegates

- **WHEN** a user forks repo `upstream` with delegates `[A, B]` and threshold `1`
- **THEN** the fork SHALL use the provided delegates and threshold
- AND the fork SHALL still reference `upstream` as its origin

#### Scenario: Fork a non-existent repo

- **WHEN** a user attempts to fork a repo that does not exist
- **THEN** the system SHALL return `ForgeError::RepoNotFound`

#### Scenario: Fork preserves identity lineage

- **WHEN** a user forks repo `upstream` to create `fork1`, then forks `fork1` to create `fork2`
- **THEN** `fork1.fork_info.upstream_repo_id` SHALL be `upstream`'s ID
- AND `fork2.fork_info.upstream_repo_id` SHALL be `fork1`'s ID

### Requirement: Fork metadata on RepoIdentity

`RepoIdentity` SHALL include an optional `fork_info: Option<ForkInfo>` field. `ForkInfo` SHALL contain the upstream repo ID and an optional upstream cluster public key (None for same-cluster forks).

#### Scenario: Non-forked repo has no fork info

- **WHEN** a repo is created via `create_repo`
- **THEN** `fork_info` SHALL be `None`

#### Scenario: Forked repo exposes upstream

- **WHEN** a forked repo's identity is retrieved
- **THEN** `fork_info` SHALL contain the upstream repo ID
- AND if the upstream is in a different cluster, `upstream_cluster` SHALL be set

### Requirement: Mirror configuration

The system SHALL support configuring a repository as a read-only mirror of an upstream repository. Mirror configuration SHALL be persisted in the Raft KV store.

#### Scenario: Enable mirror mode

- **WHEN** a user enables mirror mode on repo `M` with upstream `U` and interval `300` seconds
- **THEN** a `MirrorConfig` SHALL be stored at `forge:mirror:{repo_id}` in KV
- AND the mirror worker SHALL begin polling upstream refs

#### Scenario: Disable mirror mode

- **WHEN** a user disables mirror mode on repo `M`
- **THEN** the `MirrorConfig` SHALL be removed from KV
- AND the mirror worker SHALL stop polling for this repo

#### Scenario: Mirror interval bounds

- **WHEN** a user sets mirror interval below 60 seconds
- **THEN** the system SHALL clamp the interval to 60 seconds
- **WHEN** a user sets mirror interval above 3600 seconds
- **THEN** the system SHALL clamp the interval to 3600 seconds

### Requirement: Mirror ref synchronization

The mirror worker SHALL periodically fetch upstream refs and update the local repo's refs to match. Only fast-forward updates SHALL be applied automatically.

#### Scenario: Mirror syncs new commits

- **WHEN** the upstream repo `U` has `heads/main` at commit `C2` and the mirror has it at `C1` (ancestor of `C2`)
- **THEN** the mirror worker SHALL advance `heads/main` to `C2`
- AND new Git objects SHALL be fetched via SyncService

#### Scenario: Mirror skips diverged refs

- **WHEN** the upstream `heads/main` has diverged from the mirror's `heads/main` (not a fast-forward)
- **THEN** the mirror worker SHALL log a warning and skip the ref update
- AND the ref SHALL remain at its current value

#### Scenario: Mirror syncs new branches

- **WHEN** the upstream creates a new branch `heads/feature`
- **THEN** the mirror worker SHALL create the same ref locally

#### Scenario: Mirror removes deleted branches

- **WHEN** the upstream deletes branch `heads/old-feature`
- **THEN** the mirror worker SHALL remove the corresponding local ref

### Requirement: Mirror resource bounds

The system SHALL enforce Tiger Style resource bounds on mirror operations.

#### Scenario: Maximum mirrored repos per node

- **WHEN** a node has 1000 active mirrors and a user tries to enable another
- **THEN** the system SHALL return an error indicating the mirror limit is reached

#### Scenario: Mirror worker concurrency

- **WHEN** multiple mirrors are due for sync simultaneously
- **THEN** the mirror worker SHALL process at most 10 concurrent syncs

### Requirement: Fork and mirror CLI commands

The CLI SHALL provide commands for forking repositories and managing mirrors.

#### Scenario: Fork via CLI

- **WHEN** a user runs `aspen-cli repo fork --upstream U --name my-fork`
- **THEN** the fork SHALL be created and its repo ID printed

#### Scenario: Enable mirror via CLI

- **WHEN** a user runs `aspen-cli repo mirror --repo R --upstream U --interval 300`
- **THEN** mirror mode SHALL be enabled for repo `R`

#### Scenario: Disable mirror via CLI

- **WHEN** a user runs `aspen-cli repo mirror --repo R --disable`
- **THEN** mirror mode SHALL be disabled

#### Scenario: Show mirror status via CLI

- **WHEN** a user runs `aspen-cli repo mirror --repo R --status`
- **THEN** the current mirror config, last sync time, and sync status SHALL be displayed

### Requirement: Fork and mirror RPC protocol

The client API SHALL include RPC types for fork and mirror operations.

#### Scenario: ForgeForkRepo RPC

- **WHEN** a client sends `ForgeForkRepo { upstream_repo_id, name, delegates, threshold }`
- **THEN** the node SHALL create the fork and return `{ repo_id, fork_info }`

#### Scenario: ForgeSetMirror RPC

- **WHEN** a client sends `ForgeSetMirror { repo_id, upstream_repo_id, interval_secs }`
- **THEN** the node SHALL persist the mirror config and return success

#### Scenario: ForgeGetMirrorStatus RPC

- **WHEN** a client sends `ForgeGetMirrorStatus { repo_id }`
- **THEN** the node SHALL return `{ config, last_sync_ms, synced_refs_count }`
