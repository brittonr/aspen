## 1. CI Config Types

- [ ] 1.1 Add `Deploy` variant to `JobType` enum in `aspen-ci-core/src/config/types.rs`
- [ ] 1.2 Add deploy-specific fields to `JobConfig`: `artifact_from: Option<String>`, `strategy: Option<DeployStrategy>`, `health_check_timeout_secs: Option<u64>`, `max_concurrent: Option<u32>`
- [ ] 1.3 Add `DeployStrategy` enum to config types: `Rolling` (default)
- [ ] 1.4 Update `JobConfig::validate()`: require `artifact_from` when `job_type == Deploy`, validate `artifact_from` references a job in a preceding stage
- [ ] 1.5 Update Nickel CI schema contract to accept `'deploy` type and deploy fields
- [ ] 1.6 Write config parsing tests for deploy job configs (valid, missing artifact_from, unknown artifact_from)

## 2. Deploy Executor

- [ ] 2.1 Create `DeployExecutor` in `aspen-ci` (or a new `aspen-ci-executor-deploy` crate if cleaner)
- [ ] 2.2 Implement artifact resolution: given pipeline run ID + artifact_from job name → find job ID → read `__jobs:{job_id}` from KV → extract Nix store path or blob hash
- [ ] 2.3 Implement deploy initiation: call `ClusterDeploy` RPC with resolved artifact, strategy, and health timeout
- [ ] 2.4 Implement progress monitoring loop: poll `ClusterDeployStatus` every `DEPLOY_STATUS_POLL_INTERVAL_SECS`, emit per-node status changes as job log lines
- [ ] 2.5 Implement completion mapping: deployment success → job success, deployment failure → job failure with error details
- [ ] 2.6 Register `DeployExecutor` in pipeline orchestrator dispatch (not worker pool — runs in-process on leader)
- [ ] 2.7 Write tests for artifact resolution with mock KV data (Nix path, blob hash, missing artifacts)

## 3. Pipeline Config Update

- [ ] 3.1 Add stage 4 `deploy` block to `.aspen/ci.ncl`: `type = 'deploy`, `artifact_from = "build-node"`, `strategy = 'rolling`, `depends_on = ["build", "test"]`
- [ ] 3.2 Gate the deploy stage with `when = "refs/heads/main"` so feature branches don't auto-deploy

## 4. Dogfood Script

- [ ] 4.1 Add `do_deploy` function to `scripts/dogfood-local.sh`: extract CI-built artifact reference from pipeline run, call `cli cluster deploy`, poll status with progress output
- [ ] 4.2 Add `do_full_loop` command: `start → push → build → deploy → verify`
- [ ] 4.3 Update `do_verify` to check the running node reports the new version (compare `cluster status` git hash with the pushed commit)
- [ ] 4.4 Add `deploy` and `full-loop` to the case statement and usage text

## 5. NixOS VM Integration Test

- [ ] 5.1 Create `nix/tests/ci-dogfood-deploy.nix` with a 3-node cluster, Forge, CI enabled
- [ ] 5.2 Push a minimal flake to Forge that produces a versioned aspen-node binary
- [ ] 5.3 Wait for CI build + test stages to complete
- [ ] 5.4 Trigger deploy stage (or let auto-trigger handle it if deploy stage is in ci.ncl)
- [ ] 5.5 Verify each node restarts: check version output, health, Raft membership
- [ ] 5.6 Verify cluster stays operational during rolling upgrade: KV read/write works at each step
- [ ] 5.7 Test rollback: trigger `cluster rollback`, verify nodes revert to previous binary

## 6. Documentation

- [ ] 6.1 Create `docs/deploy.md`: architecture overview, CI config examples, CLI usage, safety guarantees, troubleshooting
- [ ] 6.2 Update `AGENTS.md`: add deploy module descriptions, `deploy` feature flag, test commands
- [ ] 6.3 Update README self-hosting section: describe the complete Forge → CI → Build → Deploy loop
- [ ] 6.4 Update napkin with any lessons learned during implementation
