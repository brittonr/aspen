## ADDED Requirements

### Requirement: Node binary replacement via Nix profile switch

The system SHALL support replacing the running node binary by switching a Nix profile to a new store path. The switch SHALL be atomic and the previous generation SHALL be preserved for rollback.

#### Scenario: Upgrade node via Nix store path

- **WHEN** a `NodeUpgrade` RPC is received with a Nix store path
- **THEN** the node SHALL run `nix-env --profile <profile-path> --set <store-path>`
- **AND** verify the new binary exists at `<store-path>/bin/aspen-node`
- **AND** signal a process restart via the configured init system

#### Scenario: Nix store path not available locally

- **WHEN** a `NodeUpgrade` RPC is received with a store path not in the local Nix store
- **THEN** the node SHALL attempt to fetch it via the cluster's binary cache substituter
- **AND** if the fetch fails, return `STORE_PATH_UNAVAILABLE` error

#### Scenario: Rollback to previous Nix generation

- **WHEN** a `NodeRollback` RPC is received
- **THEN** the node SHALL run `nix-env --profile <profile-path> --rollback`
- **AND** signal a process restart

### Requirement: Node binary replacement via blob fetch

The system SHALL support replacing the running node binary by downloading a blob from the cluster's blob store. This is the fallback for non-Nix environments.

#### Scenario: Upgrade node via blob hash

- **WHEN** a `NodeUpgrade` RPC is received with an artifact blob hash and no Nix store path
- **THEN** the node SHALL download the blob to `<data-dir>/staging/aspen-node-<hash>`
- **AND** validate SHA-256 matches
- **AND** run `<staging-binary> --version` to verify it executes
- **AND** atomically rename over the running binary path
- **AND** preserve the old binary at `<binary-path>.bak`
- **AND** signal a process restart

#### Scenario: Blob download failure

- **WHEN** blob download fails or times out
- **THEN** the upgrade SHALL fail with `ARTIFACT_FETCH_FAILED`
- **AND** the staging file SHALL be cleaned up
- **AND** the running binary SHALL remain unchanged

#### Scenario: Binary validation failure

- **WHEN** the downloaded binary fails `--version` (non-zero exit)
- **THEN** the upgrade SHALL fail with `BINARY_VALIDATION_FAILED`
- **AND** the staging file SHALL be cleaned up

### Requirement: Graceful drain before upgrade

The node SHALL drain in-flight operations before replacing its binary, bounded by a timeout.

#### Scenario: Drain completes within timeout

- **WHEN** a node begins the upgrade process
- **THEN** it SHALL stop accepting new client RPC connections
- **AND** return `NOT_LEADER` for new Raft write proposals
- **AND** continue serving Raft replication traffic
- **AND** wait up to `DRAIN_TIMEOUT_SECS` (30) for in-flight operations
- **AND** proceed with binary replacement after drain

#### Scenario: Drain timeout exceeded

- **WHEN** in-flight operations do not complete within `DRAIN_TIMEOUT_SECS`
- **THEN** the node SHALL cancel remaining operations
- **AND** proceed with binary replacement
- **AND** log a warning with the count of cancelled operations

### Requirement: Process restart

The node SHALL restart itself after binary replacement using the appropriate mechanism for the runtime environment.

#### Scenario: Restart via systemd

- **WHEN** the node detects it is running under systemd
- **THEN** it SHALL execute `systemctl restart aspen-node`

#### Scenario: Restart via process re-exec

- **WHEN** the node is NOT running under systemd
- **THEN** it SHALL call `execve` with the new binary path and original arguments

### Requirement: Upgrade status reporting

The node SHALL report its upgrade status to the cluster via KV writes.

#### Scenario: Successful upgrade lifecycle

- **WHEN** a node drains, replaces binary, and restarts
- **THEN** it SHALL write status `restarting` to `_sys:deploy:node:{node_id}` before restart
- **AND** after restart, write status `healthy` once health checks pass

#### Scenario: Failed upgrade

- **WHEN** any upgrade step fails
- **THEN** the node SHALL write status `failed` with error message to `_sys:deploy:node:{node_id}`
- **AND** remain running on the old binary
