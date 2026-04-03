## ADDED Requirements

### Requirement: Subcommand interface matching existing scripts

The `aspen-dogfood` binary SHALL expose subcommands that match the current script interface: `start`, `stop`, `status`, `push`, `build`, `deploy`, `verify`, `full-loop`, and `full`. Each subcommand SHALL be independently runnable against an existing cluster (except `start` which creates one).

#### Scenario: Run individual subcommands

- **WHEN** a user runs `aspen-dogfood start` followed by `aspen-dogfood push`
- **THEN** the start command creates a cluster and writes a state file, and the push command reads the state file to connect to the running cluster

#### Scenario: Run full pipeline

- **WHEN** a user runs `aspen-dogfood full`
- **THEN** it executes start → push → build → deploy → verify → stop in sequence, stopping on first failure

### Requirement: Mode selection via flags

The binary SHALL support `--federation` flag for two-cluster federation mode and `--vm-ci` flag for VM-isolated CI execution. These flags SHALL be mutually composable (federation + VM-CI is valid).

#### Scenario: Federation mode

- **WHEN** a user runs `aspen-dogfood --federation full`
- **THEN** it starts two clusters, pushes to the first, verifies federation sync to the second, and runs CI on the second cluster

#### Scenario: VM-CI mode

- **WHEN** a user runs `aspen-dogfood --vm-ci full`
- **THEN** it starts a cluster configured with the VM CI executor and runs the pipeline with CI jobs executing in microVMs

### Requirement: Nix flake integration

The `aspen-dogfood` binary SHALL be exposed as `nix run .#dogfood-local` (replacing the current shell script wrapper). The flake app entry SHALL inject binary paths (`ASPEN_NODE_BIN`) the same way the current `writeShellScript` wrappers do.

#### Scenario: Nix run dogfood-local

- **WHEN** a user runs `nix run .#dogfood-local`
- **THEN** it builds and runs `aspen-dogfood` with the default single-node mode

#### Scenario: Nix run with subcommand passthrough

- **WHEN** a user runs `nix run .#dogfood-local -- start`
- **THEN** the `start` argument is passed through to `aspen-dogfood start`

### Requirement: State file for cross-invocation persistence

The binary SHALL write a JSON state file to the cluster directory containing node PIDs, cluster tickets, and endpoint addresses. Subsequent subcommands SHALL read this file to reconnect to the running cluster. The state file SHALL be deleted on `stop`.

#### Scenario: State file written on start

- **WHEN** `aspen-dogfood start` completes
- **THEN** a `dogfood-state.json` file exists in the cluster directory containing node PIDs and cluster tickets

#### Scenario: State file consumed by push

- **WHEN** `aspen-dogfood push` runs after `start`
- **THEN** it reads the state file, connects to the cluster using the stored ticket, and executes the push

#### Scenario: State file cleaned on stop

- **WHEN** `aspen-dogfood stop` completes
- **THEN** the state file and cluster directory are removed

### Requirement: Structured error reporting

All errors SHALL use snafu context types with actionable messages. Errors from `aspen-client` RPCs SHALL include the operation name, target node, and the underlying error. Process spawn failures SHALL include the binary path and arguments.

#### Scenario: Client RPC error

- **WHEN** a `GetHealth` RPC fails during the start step
- **THEN** the error message includes "health check failed for node 1: <underlying error>" and the binary exits with code 1

#### Scenario: Node process spawn failure

- **WHEN** the `aspen-node` binary is not found at the configured path
- **THEN** the error message includes "failed to spawn aspen-node at <path>: No such file" and the binary exits with code 1
