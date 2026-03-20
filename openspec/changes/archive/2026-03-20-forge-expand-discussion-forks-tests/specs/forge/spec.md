## MODIFIED Requirements

### Requirement: Repository Management

The system SHALL support creating, listing, forking, and deleting Git repositories. Repository metadata SHALL be stored in the cluster's KV store. Forked repositories SHALL track their upstream origin via `ForkInfo` on `RepoIdentity`.

#### Scenario: Create repository

- GIVEN an authenticated user with create permissions
- WHEN the user creates repository `"my-project"`
- THEN repository metadata SHALL be stored at `repos:<repo_id>:identity` in the KV store
- AND the repository SHALL be ready to accept pushes
- AND `fork_info` SHALL be `None`

#### Scenario: List repositories

- GIVEN repositories `"alpha"`, `"beta"`, `"gamma"` exist
- WHEN a user lists repositories
- THEN all three SHALL be returned with their metadata

#### Scenario: Fork repository

- GIVEN repository `"upstream"` exists with commits on `heads/main`
- WHEN a user forks it as `"my-fork"`
- THEN a new repository `"my-fork"` SHALL be created
- AND `fork_info.upstream_repo_id` SHALL reference `"upstream"`
- AND all refs SHALL be copied from `"upstream"`

#### Scenario: Delete repository

- GIVEN repository `"old-project"` exists
- WHEN an administrator deletes it
- THEN the metadata SHALL be removed from the KV store
- AND the repository SHALL no longer accept operations
