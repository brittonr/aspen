## ADDED Requirements

### Requirement: Deploy job type in CI pipeline configuration

The CI configuration language SHALL support a `deploy` job type that references build artifacts and triggers a cluster deployment.

#### Scenario: Define a deploy stage in Nickel config

- **WHEN** a `.aspen/ci.ncl` file contains a stage with a job of `type = 'deploy`
- **THEN** the parser SHALL accept the config
- **AND** the deploy job SHALL reference an artifact source via `artifact_from` (name of a build job)
- **AND** the deploy job SHALL specify a `strategy` (default: `rolling`)

#### Scenario: Deploy job missing artifact_from

- **WHEN** a deploy job is defined without `artifact_from`
- **THEN** pipeline validation SHALL reject the config with error "deploy job requires artifact_from"

#### Scenario: Deploy job references non-existent build job

- **WHEN** a deploy job's `artifact_from` references a job name that does not exist in previous stages
- **THEN** pipeline validation SHALL reject the config with error "artifact_from references unknown job"

### Requirement: Deploy executor resolves artifacts from build jobs

The deploy executor SHALL locate the build artifact produced by the referenced build job and pass it to the cluster deployment coordinator.

#### Scenario: Resolve Nix store path from build job

- **WHEN** a deploy executor runs and the referenced build job produced a Nix store path
- **THEN** the executor SHALL read the job result from KV (`__jobs:{job_id}`)
- **AND** extract `result.Success.data.output_paths[0]` as the Nix store path
- **AND** call `ClusterDeploy` RPC with the store path

#### Scenario: Resolve blob hash from build job

- **WHEN** a deploy executor runs and the referenced build job produced blob artifacts
- **THEN** the executor SHALL extract `result.Success.data.artifacts[].blob_hash` from the job result
- **AND** call `ClusterDeploy` RPC with the blob hash

#### Scenario: Build job has no artifacts

- **WHEN** the referenced build job completed but produced no output paths or artifact hashes
- **THEN** the deploy executor SHALL fail with `NO_ARTIFACTS_FOUND` error

### Requirement: Deploy executor monitors deployment completion

The deploy executor SHALL poll deployment status and report progress as CI job logs.

#### Scenario: Successful deployment

- **WHEN** the deploy executor starts a deployment via `ClusterDeploy` RPC
- **THEN** it SHALL poll `ClusterDeployStatus` RPC every 5 seconds
- **AND** log per-node upgrade progress as the deployment proceeds
- **AND** report success when all nodes are upgraded and healthy
- **AND** the CI job SHALL be marked as `success`

#### Scenario: Deployment failure

- **WHEN** the deployment fails (node health check timeout or explicit failure)
- **THEN** the deploy executor SHALL log the failure details
- **AND** the CI job SHALL be marked as `failed`
- **AND** the pipeline SHALL be marked as `failed`

### Requirement: Deploy stage depends on build and test stages

The deploy stage SHALL only execute after build and test stages pass. The pipeline config SHALL enforce this dependency.

#### Scenario: Deploy skipped on test failure

- **WHEN** the test stage fails
- **THEN** the deploy stage SHALL be skipped
- **AND** the pipeline SHALL be marked as `failed` without attempting deployment

#### Scenario: Deploy runs after successful build and test

- **WHEN** both build and test stages complete successfully
- **THEN** the deploy stage SHALL execute its jobs
