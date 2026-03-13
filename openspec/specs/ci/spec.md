# CI/CD Specification

## Purpose

Distributed CI/CD pipeline system with Nickel-based configuration, multiple executor backends (shell, Nix, VM), and integration with the Forge for trigger-on-push workflows. Aims to replace GitHub Actions for Aspen's own builds.

## Requirements

### Requirement: Pipeline Configuration

The system SHALL use Nickel as the configuration language for CI/CD pipelines. Pipeline definitions SHALL specify stages, jobs, dependencies, triggers, and executor requirements.

#### Scenario: Define a pipeline

- GIVEN a Nickel configuration file defining stages `[build, test, deploy]`
- WHEN the pipeline is loaded
- THEN each stage SHALL be parsed with its jobs, dependencies, and executor type

#### Scenario: Validate configuration

- GIVEN a Nickel config with a circular dependency between jobs
- WHEN validation runs
- THEN the system SHALL reject the config with a descriptive error

### Requirement: Pipeline Execution

The system SHALL execute pipeline stages in dependency order, parallelizing independent jobs. Execution SHALL be distributed across available worker nodes. When a cluster cache is available, Nix builds SHALL automatically use it as a substituter.

#### Scenario: Sequential stages

- GIVEN stages `build → test → deploy` with dependencies
- WHEN the pipeline executes
- THEN `test` SHALL NOT start until `build` completes successfully
- AND `deploy` SHALL NOT start until `test` completes successfully

#### Scenario: Parallel jobs within a stage

- GIVEN a `test` stage with jobs `[unit-tests, lint, clippy]` with no inter-dependencies
- WHEN the stage executes
- THEN all three jobs MAY run in parallel on different workers

#### Scenario: Nix build with cache substituter

- GIVEN a CI job that runs `nix build` and `use_cluster_cache` is enabled
- WHEN the executor runs the build command
- THEN it SHALL inject `--substituters http://127.0.0.1:{proxy_port}` and `--trusted-public-keys {cache_public_key}` into the Nix invocation
- AND the cache proxy SHALL translate HTTP requests to aspen client RPC

#### Scenario: Nix build without cache

- GIVEN a CI job that runs `nix build` and `use_cluster_cache` is disabled
- WHEN the executor runs the build command
- THEN it SHALL NOT modify the Nix invocation's substituter configuration

#### Scenario: Job failure propagation

- GIVEN `build` fails in a `build → test → deploy` pipeline
- WHEN `build` completes with an error
- THEN `test` and `deploy` SHALL be skipped
- AND the pipeline SHALL be marked as failed

### Requirement: Executor Backends

The system SHALL support multiple executor backends for running CI jobs.

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

### Requirement: Forge Integration

The system SHALL trigger pipelines based on Forge events (push, tag, branch creation).

#### Scenario: Trigger on push

- GIVEN a pipeline configured to trigger on pushes to `main`
- WHEN a commit is pushed to `main` in the forge
- THEN the pipeline SHALL be triggered automatically

### Requirement: Artifact Storage

The system SHALL store CI artifacts (build outputs, logs, test results) as content-addressed blobs via iroh-blobs.

#### Scenario: Upload build artifact

- GIVEN a job produces binary `aspen-node`
- WHEN the job completes successfully
- THEN the artifact SHALL be stored as an iroh-blob
- AND the blob hash SHALL be recorded in the pipeline results

<!-- Merged from forge-ci-integration -->
### Requirement: Pipeline Execution

The system SHALL execute pipeline stages in dependency order, parallelizing independent jobs. Execution SHALL be distributed across available worker nodes. When a cluster cache is available, Nix builds SHALL automatically use it as a substituter. Pipeline runs SHALL be indexed by repository and ref for efficient status queries.

#### Scenario: Sequential stages

- **WHEN** stages `build → test → deploy` with dependencies
- **THEN** `test` SHALL NOT start until `build` completes successfully
- **AND** `deploy` SHALL NOT start until `test` completes successfully

#### Scenario: Parallel jobs within a stage

- **WHEN** a `test` stage with jobs `[unit-tests, lint, clippy]` with no inter-dependencies
- **THEN** all three jobs MAY run in parallel on different workers

#### Scenario: Nix build with cache substituter

- **WHEN** a CI job that runs `nix build` and `use_cluster_cache` is enabled
- **THEN** it SHALL inject `--substituters http://127.0.0.1:{proxy_port}` and `--trusted-public-keys {cache_public_key}` into the Nix invocation
- **AND** the cache proxy SHALL translate HTTP requests to aspen client RPC

#### Scenario: Nix build without cache

- **WHEN** a CI job that runs `nix build` and `use_cluster_cache` is disabled
- **THEN** it SHALL NOT modify the Nix invocation's substituter configuration

#### Scenario: Job failure propagation

- **WHEN** `build` fails in a `build → test → deploy` pipeline
- **THEN** `test` and `deploy` SHALL be skipped
- **AND** the pipeline SHALL be marked as failed

#### Scenario: Pipeline status indexed by ref

- **WHEN** a pipeline run starts for repo `R` on ref `refs/heads/main`
- **THEN** the latest run ID SHALL be written to `_ci:ref-status:{repo_hex}:refs/heads/main`
- **AND** querying `ci status R main` SHALL return the latest pipeline run status

#### Scenario: Pipeline status updated on completion

- **WHEN** a pipeline run completes (success or failure)
- **THEN** the ref-status index SHALL be updated with final status
- **AND** the run record at `_ci:runs:{run_id}` SHALL reflect the terminal state

<!-- Merged from ci-nix-flake-checkout -->
### Requirement: SNIX cache upload from CI builds

The NixBuildWorker SHALL use SNIX services (BlobService, DirectoryService, PathInfoService) for cache uploads when available, in addition to the legacy blob store path.

#### Scenario: SNIX services wired in node binary

- **WHEN** the node binary starts with `snix` feature enabled and `config.snix.is_enabled` is true
- **THEN** `NixBuildWorkerConfig` SHALL receive non-None SNIX service references
- **AND** `upload_store_paths_snix()` SHALL be invoked for store path uploads

#### Scenario: SNIX services unavailable

- **WHEN** the node binary starts without `snix` feature or with SNIX disabled
- **THEN** `NixBuildWorkerConfig` SHALL have `snix_*_service: None`
- **AND** the legacy `upload_store_paths()` path SHALL be used as fallback

## ADDED Requirements

### Requirement: Pipeline config SHALL support publish_to_cache

The Nickel CI config schema and `JobConfig` struct SHALL include a `publish_to_cache` boolean field that controls whether build outputs are uploaded to the Nix binary cache.

#### Scenario: Default publish_to_cache

- **WHEN** a job config does not specify `publish_to_cache`
- **THEN** it SHALL default to `true`

#### Scenario: Disable cache publishing

- **WHEN** a job config sets `publish_to_cache = false`
- **THEN** the NixBuildWorker SHALL skip store path uploads for that job

## ADDED Requirements

### Requirement: CI jobs use branch overlay for workspace isolation

CI job executors SHALL create a `BranchOverlay` for each job's workspace. All KV writes during job execution SHALL go through the branch. On job success, the branch SHALL be committed. On job failure, the branch SHALL be dropped with no cleanup.

#### Scenario: Successful job commits workspace

- **WHEN** a CI job runs inside a branch overlay
- **AND** the job writes build artifacts to its workspace prefix
- **AND** the job completes successfully
- **THEN** all workspace writes SHALL be committed to the base store atomically

#### Scenario: Failed job leaves no orphaned keys

- **WHEN** a CI job runs inside a branch overlay
- **AND** the job writes partial results to its workspace prefix
- **AND** the job fails
- **THEN** the branch SHALL be dropped
- **AND** no workspace keys SHALL exist in the base store

#### Scenario: Job crash leaves no orphaned keys

- **WHEN** a CI job is running inside a branch overlay
- **AND** the node crashes mid-job
- **THEN** the in-memory branch state SHALL be lost
- **AND** the base store SHALL contain no partial workspace keys from the crashed job
- **AND** pipeline recovery SHALL reschedule the job

### Requirement: Parallel pipeline jobs use independent branches

Each parallel job within a CI pipeline stage SHALL use its own `BranchOverlay`. Branches SHALL be independent — one job's failure SHALL NOT affect another job's branch.

#### Scenario: Independent parallel branches

- **WHEN** jobs "build", "lint", and "test" run in parallel
- **AND** each has its own branch overlay
- **AND** "test" fails
- **THEN** the "test" branch SHALL be dropped
- **AND** the "build" and "lint" branches SHALL remain unaffected
- **AND** "build" and "lint" MAY still commit on success
