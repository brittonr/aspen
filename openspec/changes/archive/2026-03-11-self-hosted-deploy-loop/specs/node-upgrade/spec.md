## ADDED Requirements

### Requirement: Node binary replacement via Nix profile switch

The system SHALL support replacing the running node binary by switching a Nix profile to a new store path. The switch SHALL be atomic (Nix guarantees this) and the previous generation SHALL be preserved for rollback.

#### Scenario: Upgrade node via Nix store path

- **WHEN** a `NodeUpgrade` RPC is received with a Nix store path `/nix/store/abc...-aspen-node`
- **THEN** the node SHALL run `nix-env --profile <profile-path> --set <store-path>`
- **AND** verify the new binary exists at `<store-path>/bin/aspen-node`
- **AND** signal a process restart via the configured init system

#### Scenario: Nix store path not available locally

- **WHEN** a `NodeUpgrade` RPC is received with a store path not present in the local Nix store
- **THEN** the node SHALL attempt to fetch it via `nix copy --from <cache-url>` using the cluster's binary cache substituter
- **AND** if the fetch fails, the upgrade SHALL fail with `STORE_PATH_UNAVAILABLE` error

#### Scenario: Rollback to previous Nix generation

- **WHEN** a `NodeRollback` RPC is received
- **THEN** the node SHALL run `nix-env --profile <profile-path> --rollback`
- **AND** signal a process restart

### Requirement: Node binary replacement via direct blob fetch

The system SHALL support replacing the running node binary by downloading a blob from the cluster's blob store and replacing the binary on disk. This is the fallback for non-Nix environments.

#### Scenario: Upgrade node via blob hash

- **WHEN** a `NodeUpgrade` RPC is received with an artifact blob hash and no Nix store path
- **THEN** the node SHALL download the blob to a staging path `<data-dir>/staging/aspen-node-<hash>`
- **AND** validate the downloaded binary by checking its SHA-256 matches the expected hash
- **AND** run `<staging-binary> --version` to verify it executes successfully
- **AND** atomically rename the staging binary over the running binary path
- **AND** preserve the old binary at `<binary-path>.bak`
- **AND** signal a process restart

#### Scenario: Blob download failure

- **WHEN** blob download fails or times out
- **THEN** the upgrade SHALL fail with `ARTIFACT_FETCH_FAILED` error
- **AND** the staging file SHALL be cleaned up
- **AND** the running binary SHALL remain unchanged

#### Scenario: Binary validation failure

- **WHEN** the downloaded binary fails the `--version` smoke test (non-zero exit)
- **THEN** the upgrade SHALL fail with `BINARY_VALIDATION_FAILED` error
- **AND** the staging file SHALL be cleaned up

### Requirement: Graceful drain before upgrade

The node SHALL drain in-flight operations before replacing its binary. Draining SHALL be bounded by a timeout.

#### Scenario: Drain completes within timeout

- **WHEN** a node begins the upgrade process
- **THEN** it SHALL stop accepting new client RPC connections
- **AND** stop accepting new Raft write proposals (return `NOT_LEADER`)
- **AND** continue serving Raft replication traffic
- **AND** wait up to `DRAIN_TIMEOUT_SECS` (default 30) for in-flight operations to complete
- **AND** proceed with binary replacement after drain completes

#### Scenario: Drain timeout exceeded

- **WHEN** in-flight operations do not complete within `DRAIN_TIMEOUT_SECS`
- **THEN** the node SHALL cancel remaining operations
- **AND** proceed with binary replacement (forced drain)
- **AND** log a warning with the count of cancelled operations

### Requirement: Process restart mechanism

The node SHALL restart itself after binary replacement using the appropriate mechanism for the runtime environment.

#### Scenario: Restart via systemd

- **WHEN** the node detects it is running under systemd (via `$NOTIFY_SOCKET` or `$INVOCATION_ID`)
- **THEN** it SHALL execute `systemctl restart aspen-node` to restart the service
- **AND** the systemd unit SHALL start the new binary

#### Scenario: Restart via process re-exec

- **WHEN** the node is NOT running under systemd
- **THEN** it SHALL call `execve` with the new binary path and the original command-line arguments
- **AND** the new process SHALL inherit the node's data directory, secret key, and configuration

### Requirement: Upgrade status reporting

The node SHALL report its upgrade status to the cluster coordinator via KV writes.

#### Scenario: Successful upgrade lifecycle

- **WHEN** a node successfully drains, replaces binary, and restarts
- **THEN** it SHALL write its upgrade status to `_sys:deploy:node:{node_id}` with status `restarting` before restart
- **AND** after restart, write status `healthy` once it passes health checks

#### Scenario: Failed upgrade

- **WHEN** any step of the upgrade fails (fetch, validate, drain)
- **THEN** the node SHALL write status `failed` with an error message to `_sys:deploy:node:{node_id}`
- **AND** remain running on the old binary
