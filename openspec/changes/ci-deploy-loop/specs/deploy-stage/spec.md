## ADDED Requirements

### Requirement: Deploy job type in CI pipeline configuration

The CI config language SHALL support a `deploy` job type that references build artifacts and triggers a cluster deployment.

#### Scenario: Define a deploy stage in Nickel config

- **WHEN** `.aspen/ci.ncl` contains a stage with a job of `type = 'deploy`
- **THEN** the parser SHALL accept the config
- **AND** the job SHALL have `artifact_from` referencing a build job name
- **AND** the job SHALL accept optional `strategy` (default: `rolling`), `health_check_timeout_secs`, `max_concurrent`

#### Scenario: Deploy job missing artifact_from

- **WHEN** a deploy job is defined without `artifact_from`
- **THEN** validation SHALL reject the config with "deploy job requires artifact_from"

#### Scenario: artifact_from references unknown job

- **WHEN** `artifact_from` names a job that doesn't exist in previous stages
- **THEN** validation SHALL reject with "artifact_from references unknown job"

### Requirement: Deploy executor resolves artifacts from build jobs

The deploy executor SHALL locate the build artifact from the referenced job and pass it to `ClusterDeploy` RPC.

#### Scenario: Resolve Nix store path

- **WHEN** the referenced build job produced a Nix store path
- **THEN** the executor SHALL read `__jobs:{job_id}` from KV
- **AND** extract `result.Success.data.output_paths[0]`
- **AND** call `ClusterDeploy` with the store path

#### Scenario: Resolve blob hash

- **WHEN** the referenced build job produced blob artifacts but no Nix store path
- **THEN** the executor SHALL extract `result.Success.data.artifacts[].blob_hash`
- **AND** call `ClusterDeploy` with the blob hash

#### Scenario: Build job has no artifacts

- **WHEN** the referenced job produced no output paths or blob hashes
- **THEN** the executor SHALL fail with `NO_ARTIFACTS_FOUND`

### Requirement: Deploy executor monitors deployment progress

The executor SHALL poll deployment status and emit progress as CI job logs.

#### Scenario: Successful deployment

- **WHEN** the executor initiates deployment via `ClusterDeploy`
- **THEN** it SHALL poll `ClusterDeployStatus` every 5 seconds
- **AND** log per-node status changes
- **AND** mark the CI job as `success` when deployment completes

#### Scenario: Deployment failure

- **WHEN** deployment fails (health timeout or node failure)
- **THEN** the executor SHALL log failure details
- **AND** mark the CI job as `failed`

### Requirement: Deploy stage dependency enforcement

The deploy stage SHALL only run after build and test stages pass.

#### Scenario: Test failure skips deploy

- **WHEN** the test stage fails
- **THEN** the deploy stage SHALL be skipped

#### Scenario: Deploy after successful build and test

- **WHEN** build and test stages complete successfully
- **THEN** the deploy stage SHALL execute
