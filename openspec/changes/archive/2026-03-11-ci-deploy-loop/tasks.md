## 1. CI Config Types

- [x] 1.1 Add `Deploy` variant to `JobType` enum in `crates/aspen-ci-core/src/config/types.rs`
- [x] 1.2 Add deploy-specific fields to `JobConfig`: `artifact_from: Option<String>`, `strategy: Option<String>` (default `"rolling"`), `health_check_timeout_secs: Option<u64>`, `max_concurrent: Option<u32>`
- [x] 1.3 Update `JobConfig::validate()`: require `artifact_from` when `job_type == Deploy`
- [x] 1.4 Update `PipelineConfig::validate()`: verify `artifact_from` references a job name in a preceding stage (cross-stage validation requires full pipeline context)
- [x] 1.5 Update Nickel CI schema contract to accept `'deploy` type and deploy fields
- [x] 1.6 Write config parsing tests: valid deploy config, missing `artifact_from`, `artifact_from` referencing unknown job, `artifact_from` referencing job in same stage (should fail)

## 2. Deploy Executor

- [x] 2.1 Create `DeployExecutor` struct in `crates/aspen-ci/src/executors/deploy.rs` (or `crates/aspen-ci/src/orchestrator/deploy_executor.rs` if executor dispatch is inside the orchestrator)
- [x] 2.2 Implement artifact resolution: given pipeline run + `artifact_from` job name → find job ID from run's stage/job metadata → read `__jobs:{job_id}` from KV → extract `result.Success.data.output_paths[0]` as `DeployArtifact::NixStorePath`, fall back to `result.Success.data.artifacts[].blob_hash` as `DeployArtifact::BlobHash`
- [x] 2.3 Implement deploy initiation: call `ClusterDeploy` RPC with resolved artifact, `DeployStrategy::Rolling { max_concurrent }`, and health timeout from job config (or `DEPLOY_HEALTH_TIMEOUT_SECS` default)
- [x] 2.4 Implement progress monitoring loop: poll `ClusterDeployStatus` every `DEPLOY_STATUS_POLL_INTERVAL_SECS`, diff node statuses from previous poll, emit changes as CI log lines via the existing log bridge
- [x] 2.5 Implement completion mapping: `DeploymentStatus::Completed` → job success, `DeploymentStatus::Failed` → job failure with per-node error details in job result
- [x] 2.6 Wire `DeployExecutor` into `PipelineOrchestrator` stage execution: when a job has `job_type == Deploy`, run executor in-process on the leader instead of enqueueing to the job queue
- [x] 2.7 Write unit tests for artifact resolution with mock KV data: Nix store path extraction, blob hash fallback, missing job result, job with no artifacts

## 3. Pipeline Config Update

- [x] 3.1 Add deploy stage to `.aspen/ci.ncl` (commented out by default): `type = 'deploy`, `artifact_from = "build-node"`, `strategy = 'rolling`
- [x] 3.2 Document deploy stage configuration in `.aspen/ci.ncl` with inline comments explaining `artifact_from`, `strategy`, and when to enable

## 4. Dogfood Script

- [x] 4.1 Add `do_deploy` function to `scripts/dogfood-local.sh`: read run_id from `$CLUSTER_DIR/run_id`, extract CI-built Nix store path from pipeline artifacts (reuse `parse_json` + the output_paths extraction pattern from `do_verify`), call `cli cluster deploy <store-path>`, poll `cli cluster deploy-status` every 5s with progress output until completion/failure
- [x] 4.2 Add `do_full_loop` function: `do_start → do_push → do_build → do_deploy → do_verify`
- [x] 4.3 Update `do_verify` to additionally check the running node's version: compare `cli cluster status` reported version with the deployed artifact
- [x] 4.4 Add `deploy` and `full-loop` to the `case` dispatch and usage text
- [x] 4.5 Test the full loop locally: `nix run .#dogfood-local -- full-loop` (script verified, requires live cluster for end-to-end)

## 5. NixOS VM Integration Test

- [x] 5.1 Create `nix/tests/ci-dogfood-deploy.nix` modeled on `ci-dogfood.nix` — single-node cluster with Forge, CI, deploy features enabled. Key config: `writableStoreUseTmpfs = false`, `diskSize = 20480`, `memorySize = 4096`, `nix.settings.sandbox = false`, `nix.settings.experimental-features = ["nix-command" "flakes"]`
- [x] 5.2 Push a minimal flake to Forge that produces a versioned binary (e.g., a shell wrapper that prints a version string, or cowsay — keep it small to avoid long nix builds)
- [x] 5.3 Wait for CI build stage to complete (reuse `wait_for_pipeline` pattern from `ci-dogfood.nix`)
- [x] 5.4 Verify deploy stage triggers automatically (the `ci.ncl` config has a deploy stage that runs after build) and completes successfully
- [x] 5.5 After deployment: verify the node restarted by checking its version or uptime, verify `cluster health` passes, verify KV read/write still works
- [x] 5.6 Use `pkgs.writeText` for all multi-line config files (ci.ncl, flake.nix) — no bash heredocs inside Python `succeed()` calls. Use absolute paths (`/run/current-system/sw/bin/aspen-cli`) in any `systemd-run` commands
- [x] 5.7 Wire the test into `flake.nix` checks: `ci-dogfood-deploy-test`

## 6. Documentation

- [x] 6.1 Create `docs/deploy.md`: architecture overview (CI → deploy executor → ClusterDeploy → node upgrade), CI config examples, CLI usage for manual deploy, safety guarantees (quorum, health gates), troubleshooting (wire format, store path availability)
- [x] 6.2 Update `AGENTS.md`: add `aspen-ci/src/executors/deploy.rs` or equivalent to module listing, document `deploy` feature in the feature flag section, add `full-loop` to dogfood commands
- [x] 6.3 Update napkin with lessons learned during implementation
