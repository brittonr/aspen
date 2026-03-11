## MODIFIED Requirements

### Requirement: Pipeline Configuration

The system SHALL use Nickel as the configuration language for CI/CD pipelines. Pipeline definitions SHALL specify stages, jobs, dependencies, triggers, and executor requirements. The `JobType` enum SHALL include a `Deploy` variant for deployment jobs.

#### Scenario: Define a pipeline

- GIVEN a Nickel configuration file defining stages `[build, test, deploy]`
- WHEN the pipeline is loaded
- THEN each stage SHALL be parsed with its jobs, dependencies, and executor type

#### Scenario: Validate configuration

- GIVEN a Nickel config with a circular dependency between jobs
- WHEN validation runs
- THEN the system SHALL reject the config with a descriptive error

#### Scenario: Define a deploy job

- GIVEN a Nickel configuration with a job of `type = 'deploy`
- WHEN the pipeline is loaded
- THEN the job SHALL be parsed with `artifact_from`, `strategy`, `health_check_timeout_secs`, and `max_concurrent` fields
- AND missing `artifact_from` SHALL cause a validation error

### Requirement: Pipeline Execution

The system SHALL execute pipeline stages in dependency order, parallelizing independent jobs. Execution SHALL be distributed across available worker nodes. When a cluster cache is available, Nix builds SHALL automatically use it as a substituter. Deploy jobs SHALL be routed to the cluster's deployment coordinator rather than a worker node.

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

#### Scenario: Deploy job execution

- GIVEN a deploy job with `artifact_from = "build-node"`
- WHEN the deploy stage executes
- THEN the executor SHALL resolve the artifact from the build-node job result
- AND call `ClusterDeploy` RPC to initiate rolling deployment
- AND monitor deployment status until completion or failure
