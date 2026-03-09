## MODIFIED Requirements

### Requirement: Executor Backends

The system SHALL support multiple executor backends for running CI jobs. Each executor SHALL claim only the job types it can handle. The shell executor SHALL NOT claim job types belonging to dedicated executors.

#### Scenario: Shell executor claims only shell types

- **WHEN** `LocalExecutorWorker` registers with the job manager
- **THEN** it SHALL claim only `["shell_command", "local_executor"]`
- **AND** it SHALL NOT claim `ci_nix_build`, `ci_vm`, or `cloud_hypervisor`

#### Scenario: Nix executor claims nix types

- **WHEN** `NixBuildWorker` registers with the job manager
- **THEN** it SHALL claim `["ci_nix_build"]`

#### Scenario: Unclaimed job type stays queued

- **WHEN** a `ci_nix_build` job is enqueued but no `NixBuildWorker` is registered
- **THEN** the job SHALL remain in the queue with status `pending`
- **AND** a warning SHALL be logged: "no worker registered for job type ci_nix_build"

#### Scenario: Shell executor

- **WHEN** a job configured with `type = 'shell` is enqueued
- **THEN** the `LocalExecutorWorker` SHALL dequeue and execute it on the host

#### Scenario: Nix executor

- **WHEN** a job configured with `type = 'nix` is enqueued
- **THEN** the `NixBuildWorker` SHALL dequeue and execute it via `nix build`

#### Scenario: VM executor

- **WHEN** a job configured with `type = 'vm` is enqueued
- **THEN** a VM worker (self-registered via `aspen-node --worker-only`) SHALL dequeue and execute it

### Requirement: Pipeline Cleanup

The system SHALL clean up checkout directories when a pipeline reaches any terminal state (success, failed, cancelled, checkout_failed).

#### Scenario: Successful pipeline cleans up checkout

- **WHEN** a pipeline completes with status `success`
- **THEN** the checkout directory `/tmp/ci-checkout-{run_id}/` SHALL be removed

#### Scenario: Failed pipeline cleans up checkout

- **WHEN** a pipeline completes with status `failed`
- **THEN** the checkout directory SHALL be removed

#### Scenario: Cleanup failure is non-fatal

- **WHEN** checkout cleanup fails (e.g., permission denied)
- **THEN** the pipeline status SHALL NOT be affected
- **AND** a warning SHALL be logged with the cleanup error

### Requirement: Dogfood VM test exercises Nix executor

The NixOS VM integration test for dogfood (`ci-dogfood.nix`) SHALL use `type = 'nix` jobs that execute via `NixBuildWorker`, matching the production `.aspen/ci.ncl` configuration.

#### Scenario: Dogfood test builds via Nix

- **WHEN** the `ci-dogfood` NixOS VM test runs
- **THEN** CI jobs SHALL use `type = 'nix` with `flake_attr` referencing Nix check derivations
- **AND** jobs SHALL be executed by `NixBuildWorker` (not `LocalExecutorWorker`)

#### Scenario: Dogfood test validates log streaming

- **WHEN** a Nix job completes in the dogfood test
- **THEN** `ci logs {run_id} {job_id}` SHALL return non-empty log content
- **AND** log chunks SHALL exist in the KV store under `_ci:logs:*` keys

### Requirement: SNIX source constant consistency

The `SNIX_GIT_SOURCE` constant in `checkout.rs` SHALL match the snix revision used in the Nix flake. A CI check SHALL detect drift between these values.

#### Scenario: Matching revisions pass

- **WHEN** `SNIX_GIT_SOURCE` contains rev `abc123` and the flake's `snix-src` input points to `abc123`
- **THEN** the CI check SHALL pass

#### Scenario: Mismatched revisions fail

- **WHEN** `SNIX_GIT_SOURCE` contains rev `abc123` but the flake's `snix-src` input points to `def456`
- **THEN** the CI check SHALL fail with a message identifying both revisions
