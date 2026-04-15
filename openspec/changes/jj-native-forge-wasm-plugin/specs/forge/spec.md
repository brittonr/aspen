## MODIFIED Requirements

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
