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

The system SHALL execute pipeline stages in dependency order, parallelizing independent jobs. Execution SHALL be distributed across available worker nodes.

#### Scenario: Sequential stages

- GIVEN stages `build → test → deploy` with dependencies
- WHEN the pipeline executes
- THEN `test` SHALL NOT start until `build` completes successfully
- AND `deploy` SHALL NOT start until `test` completes successfully

#### Scenario: Parallel jobs within a stage

- GIVEN a `test` stage with jobs `[unit-tests, lint, clippy]` with no inter-dependencies
- WHEN the stage executes
- THEN all three jobs MAY run in parallel on different workers

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
