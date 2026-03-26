## ADDED Requirements

### Requirement: Deploy wait flag blocks until terminal state

The `aspen-cli cluster deploy` command SHALL accept a `--wait` flag that causes the CLI to poll `ClusterDeployStatus` until the deployment reaches a terminal state (completed, failed, or rolled_back).

#### Scenario: Wait returns success on completed deployment

- **WHEN** `aspen-cli cluster deploy /nix/store/abc --wait` is run
- **AND** the deployment completes successfully
- **THEN** the CLI SHALL exit with code 0
- **AND** the CLI SHALL print per-node status transitions as they occur

#### Scenario: Wait returns failure on failed deployment

- **WHEN** `aspen-cli cluster deploy /nix/store/abc --wait` is run
- **AND** a node fails health checks during deployment
- **THEN** the CLI SHALL exit with a non-zero exit code
- **AND** the CLI SHALL print the failure reason and per-node errors

#### Scenario: Wait without the flag returns immediately

- **WHEN** `aspen-cli cluster deploy /nix/store/abc` is run without `--wait`
- **THEN** the CLI SHALL print the deploy_id and exit immediately (current behavior unchanged)

### Requirement: Deploy timeout flag caps wait duration

The `aspen-cli cluster deploy` command SHALL accept a `--timeout` flag (in seconds) that caps how long `--wait` blocks. The default timeout SHALL be 3600 seconds.

#### Scenario: Timeout expires before deployment completes

- **WHEN** `aspen-cli cluster deploy /nix/store/abc --wait --timeout 60` is run
- **AND** the deployment does not reach a terminal state within 60 seconds
- **THEN** the CLI SHALL exit with a non-zero exit code
- **AND** the CLI SHALL print a timeout error with the last known deployment status

#### Scenario: Timeout is ignored without wait

- **WHEN** `aspen-cli cluster deploy /nix/store/abc --timeout 60` is run without `--wait`
- **THEN** the `--timeout` flag SHALL have no effect

### Requirement: Deploy wait emits streaming status

While waiting, the CLI SHALL poll deployment status at a fixed interval and print per-node status changes as they occur, showing node ID, old status, and new status.

#### Scenario: Node status transition displayed

- **WHEN** the CLI is waiting for deployment completion
- **AND** node 2 transitions from "pending" to "draining"
- **THEN** the CLI SHALL print a line indicating node 2's status changed to "draining"

#### Scenario: JSON mode emits structured status

- **WHEN** `aspen-cli --json cluster deploy /nix/store/abc --wait` is run
- **THEN** each status update SHALL be emitted as a JSON object on its own line
- **AND** the final result SHALL be a JSON object with the terminal deployment status
