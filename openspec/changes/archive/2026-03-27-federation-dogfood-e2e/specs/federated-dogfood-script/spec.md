## ADDED Requirements

### Requirement: Two independent clusters on localhost

The script SHALL start two fully independent Aspen clusters (alice and bob) on the same host, each with its own cookie, secret key, data directory, and iroh endpoint.

#### Scenario: Clusters start with separate identities

- **WHEN** `dogfood-federation.sh start` runs
- **THEN** two aspen-node processes start with different cookies, different data dirs under `/tmp/aspen-dogfood-federation/`, and both pass `cluster health` checks

#### Scenario: Clusters are isolated

- **WHEN** both clusters are running
- **THEN** KV writes on alice are not visible on bob (separate Raft groups)

### Requirement: Push source to alice's Forge

The script SHALL create a Forge repo on alice's cluster and push Aspen source via `git-remote-aspen`.

#### Scenario: Source pushed successfully

- **WHEN** `dogfood-federation.sh push` runs
- **THEN** a Forge repo exists on alice with Aspen source at `HEAD:main`, including `Cargo.toml`, `flake.nix`, and `.aspen/ci.ncl`

### Requirement: Federate alice's repo

The script SHALL mark alice's Forge repo as federated (public mode) so other clusters can discover and sync it.

#### Scenario: Repo federated

- **WHEN** `dogfood-federation.sh federate` runs after push
- **THEN** `federation federate <repo-id> --mode public` succeeds on alice

### Requirement: Sync federated repo to bob

The script SHALL sync the federated repo from alice to bob using `federation sync` with direct address hints.

#### Scenario: Sync creates mirror

- **WHEN** `dogfood-federation.sh sync` runs with alice's iroh NodeId and address
- **THEN** bob's cluster contains mirror metadata (`_fed:mirror:` KV keys) and the synced git objects

### Requirement: CI build on bob from mirrored content

The script SHALL trigger a CI build on bob using the mirrored content from alice's repo, either by pushing the synced content to a local bob Forge repo with CI watch, or by direct `ci trigger`.

#### Scenario: Pipeline completes

- **WHEN** CI triggers on bob from the mirrored content
- **THEN** a nix build pipeline runs and reaches `success` status within 20 minutes

### Requirement: Verify build output

The script SHALL verify the CI-built binary is runnable and was built from the federated source.

#### Scenario: Binary runs

- **WHEN** the CI pipeline succeeds on bob
- **THEN** the output nix store path contains a runnable `aspen-node` binary (or the configured build target)

### Requirement: Subcommand interface

The script SHALL support subcommands matching `dogfood-local.sh` patterns.

#### Scenario: Individual steps

- **WHEN** user runs `dogfood-federation.sh <cmd>`
- **THEN** these subcommands work: `start`, `stop`, `status`, `push`, `federate`, `sync`, `build`, `verify`, `full`

#### Scenario: Full pipeline

- **WHEN** user runs `dogfood-federation.sh full`
- **THEN** the script executes start → push → federate → sync → build → verify in sequence
