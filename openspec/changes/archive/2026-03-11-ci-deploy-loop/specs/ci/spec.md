## MODIFIED Requirements

### Requirement: Pipeline Configuration

The system SHALL use Nickel as the configuration language for CI/CD pipelines. Pipeline definitions SHALL specify stages, jobs, dependencies, triggers, and executor requirements. The `JobType` enum SHALL include a `Deploy` variant for deployment jobs.

#### Scenario: Define a pipeline with deploy stage

- GIVEN a Nickel configuration file defining stages `[build, test, deploy]`
- WHEN the pipeline is loaded
- THEN each stage SHALL be parsed with its jobs, dependencies, and executor type
- AND deploy jobs SHALL have `artifact_from` referencing a build job name

#### Scenario: Validate deploy job config

- GIVEN a Nickel config with a job of `type = 'deploy` and `artifact_from = "build-node"`
- WHEN validation runs
- THEN the config SHALL be accepted if `"build-node"` is a job in a preceding stage
- AND the config SHALL be rejected if `"build-node"` is not found or is in the same stage

#### Scenario: Deploy job missing artifact_from

- GIVEN a deploy job defined without `artifact_from`
- WHEN validation runs
- THEN the system SHALL reject the config with "deploy job requires artifact_from"

### Requirement: Pipeline Execution

The system SHALL execute pipeline stages in dependency order, parallelizing independent jobs. Deploy jobs SHALL be executed by the pipeline orchestrator directly on the leader node, not dispatched to the worker queue.

#### Scenario: Stage ordering with deploy

- GIVEN stages `build → test → deploy` with dependencies
- WHEN the pipeline executes
- THEN `deploy` SHALL NOT start until `build` and `test` complete successfully

#### Scenario: Job failure skips deploy

- GIVEN `test` fails in a `build → test → deploy` pipeline
- WHEN `test` completes with an error
- THEN `deploy` SHALL be skipped
- AND the pipeline SHALL be marked as failed

#### Scenario: Deploy job execution path

- GIVEN a deploy job with `artifact_from = "build-node"`
- WHEN the deploy stage executes on the leader
- THEN the orchestrator SHALL run `DeployExecutor` in-process
- AND SHALL NOT enqueue the job to the worker pool
