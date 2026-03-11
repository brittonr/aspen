## ADDED Requirements

### Requirement: Artifact resolution from build job results

The deploy executor SHALL locate the build artifact from the referenced job's KV result and pass it to `ClusterDeploy` RPC.

#### Scenario: Resolve Nix store path

- GIVEN a pipeline run where build job `"build-node"` produced a Nix store path
- WHEN the deploy executor resolves the artifact
- THEN it SHALL read `__jobs:{job_id}` from KV
- AND extract `result.Success.data.output_paths[0]`
- AND call `ClusterDeploy` with `DeployArtifact::NixStorePath`

#### Scenario: Resolve blob hash fallback

- GIVEN a build job that produced blob artifacts but no Nix store path in `output_paths`
- WHEN the deploy executor resolves the artifact
- THEN it SHALL extract `result.Success.data.artifacts[].blob_hash`
- AND call `ClusterDeploy` with `DeployArtifact::BlobHash`

#### Scenario: No artifacts found

- GIVEN a build job that produced no output paths or blob hashes
- WHEN the deploy executor resolves the artifact
- THEN it SHALL fail with error code `NO_ARTIFACTS_FOUND`
- AND the deploy job SHALL be marked as failed

#### Scenario: Referenced job not yet completed

- GIVEN stage ordering guarantees that build completes before deploy starts
- WHEN the deploy executor reads the job result from KV
- THEN the result SHALL always be present (stage ordering ensures this)

### Requirement: Deployment progress as CI job logs

The deploy executor SHALL poll deployment status and emit per-node progress as CI job log lines.

#### Scenario: Successful deployment with progress

- WHEN the executor initiates deployment via `ClusterDeploy`
- THEN it SHALL poll `ClusterDeployStatus` every 5 seconds
- AND emit a log line when any node's status changes (e.g., "Node 2: draining", "Node 2: healthy ✓")
- AND mark the CI job as `success` when `DeploymentStatus::Completed`

#### Scenario: Deployment failure

- WHEN deployment fails (health timeout or node upgrade failure)
- THEN the executor SHALL emit the failure details as log lines
- AND mark the CI job as `failed` with per-node error information

#### Scenario: Log lines visible via ci logs --follow

- GIVEN a deploy job in progress
- WHEN an operator runs `aspen-cli ci logs --follow <run_id> <deploy_job_id>`
- THEN they SHALL see per-node deployment progress in real time
- AND the log format SHALL use `[deploy]` prefix for all lines

### Requirement: Dogfood script full-loop command

The dogfood script SHALL support a `full-loop` command that runs the complete self-hosted cycle.

#### Scenario: full-loop success

- GIVEN a running Aspen cluster started by `dogfood-local.sh start`
- WHEN the operator runs `dogfood-local.sh full-loop`
- THEN it SHALL execute: start → push → build → deploy → verify
- AND `deploy` SHALL extract the CI-built artifact and call `cluster deploy`
- AND `verify` SHALL confirm the running node reports the new version

#### Scenario: deploy command standalone

- GIVEN a completed CI build with artifacts
- WHEN the operator runs `dogfood-local.sh deploy`
- THEN it SHALL extract the artifact from the latest pipeline run
- AND call `cli cluster deploy <store-path>`
- AND poll `cli cluster deploy-status` with progress output

### Requirement: NixOS VM integration test for full cycle

The VM test SHALL validate the complete Forge → CI → Build → Deploy cycle in isolation.

#### Scenario: Full pipeline in VM

- GIVEN a single-node NixOS VM with aspen-node, aspen-cli, git-remote-aspen, and nix
- WHEN the test pushes a minimal flake to Forge
- THEN CI SHALL auto-trigger a build pipeline
- AND the build SHALL produce a Nix store path
- AND the deploy stage SHALL call `ClusterDeploy` with the store path
- AND the node SHALL restart with the new binary
- AND `cluster health` SHALL pass after restart

#### Scenario: VM test resource configuration

- GIVEN the VM test needs to run `nix build` inside the VM
- THEN `writableStoreUseTmpfs` SHALL be `false` (disk-backed, not tmpfs)
- AND `diskSize` SHALL be at least 20480 MB
- AND `memorySize` SHALL be at least 4096 MB
- AND `nix.settings.sandbox` SHALL be `false`
