## MODIFIED Requirements

### Requirement: Executor Backends

The system SHALL support multiple executor backends for running CI jobs. The shell and Nix executors SHALL run Nix evaluation and build commands through a `NixBuildSupervisor` that manages them as supervised child processes with configurable timeouts.

#### Scenario: Shell executor

- GIVEN a job configured with `executor: shell`
- WHEN the job runs
- THEN commands SHALL execute in a shell on the worker node

#### Scenario: Nix executor

- GIVEN a job configured with `executor: nix`
- WHEN the job runs
- THEN the job SHALL execute inside a Nix build sandbox with specified dependencies

#### Scenario: VM executor

- GIVEN a job configured with `executor: vm`
- WHEN the job runs
- THEN the job SHALL execute inside an isolated virtual machine
- AND the VM SHALL be destroyed after the job completes

#### Scenario: Nix build supervised with timeout

- GIVEN a shell or Nix executor job running a `nix build` command
- WHEN the build exceeds `nix_build_timeout_secs` (default: 1800)
- THEN the `NixBuildSupervisor` SHALL kill the child process
- AND the job SHALL be marked as failed with a timeout error

## ADDED Requirements

### Requirement: Deploy stage stateful option

The Nickel CI configuration schema SHALL include a `deploy.stateful` boolean option on deploy stages. When `stateful` is `true`, the deploy executor SHALL write lifecycle state to Raft KV. When `false`, only job logs are persisted.

#### Scenario: Stateful deploy stage in Nickel config

- **WHEN** a deploy stage sets `stateful = true` (or omits the field)
- **THEN** the parsed `DeployRequest` SHALL have `stateful: true`
- **AND** the deploy executor SHALL write state under `_deploy:state:{deploy_id}:`

#### Scenario: Stateless deploy stage in Nickel config

- **WHEN** a deploy stage sets `stateful = false`
- **THEN** the parsed `DeployRequest` SHALL have `stateful: false`
- **AND** the deploy executor SHALL NOT write any keys under `_deploy:state:`
