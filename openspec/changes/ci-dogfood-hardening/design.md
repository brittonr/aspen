## Context

The CI dogfood pipeline (`scripts/dogfood-local.sh` + `.aspen/ci.ncl`) is the primary proof that Aspen can build itself. It currently works end-to-end: git push → Forge → CI auto-trigger → nix build → success. However, the NixOS VM integration test (`nix/tests/ci-dogfood.nix`) uses `type = 'shell` jobs instead of `type = 'nix`, exercising a fundamentally different code path than production. Several other flaws — dead config, executor misrouting, log streaming gaps, resource leaks — reduce confidence in the dogfood signal.

Three executor crates handle CI jobs:

- `aspen-ci-executor-shell` (`LocalExecutorWorker`): shell commands on host
- `aspen-ci-executor-nix` (`NixBuildWorker`): `nix build` on host with log streaming
- `aspen-ci-executor-vm` (`CloudHypervisorWorker`): VM pool manager (VMs self-register)

The job queue routes by `job_type` string. Currently `LocalExecutorWorker` claims `["shell_command", "ci_nix_build", "ci_vm", "cloud_hypervisor", "local_executor"]` — it acts as a catch-all fallback, which conflicts with dedicated workers.

## Goals / Non-Goals

**Goals:**

- VM dogfood test exercises the real Nix executor path (`NixBuildWorker`)
- `ci logs --follow` works for all job types (shell, nix, vm)
- Shell executor doesn't steal jobs from Nix/VM executors
- Checkout directories are cleaned up on all terminal pipeline states
- `ignore_paths` config is actually evaluated against changed files
- `ci watch` + push race is eliminated
- SNIX_GIT_SOURCE constant drift is caught automatically

**Non-Goals:**

- Multi-node dogfood testing (valuable but separate effort)
- Replacing the shell-based dogfood script with a Rust implementation
- VM executor architecture changes (pool manager pattern is fine)
- Binary reproducibility verification in CI (verify step is local-only by design)

## Decisions

### D1: VM dogfood test switches to Nix executor (Priority: Highest)

The `ci-dogfood.nix` test will use `type = 'nix` jobs with flake check attributes, matching the real `.aspen/ci.ncl` pattern. Instead of `cargo build` shell commands, jobs will build actual Nix checks from the Aspen flake.

**Approach**: Create a minimal flake.nix overlay in the dogfood test that defines lightweight checks (e.g., `cargo check -p aspen-constants`, `cargo test -p aspen-constants`) as Nix derivations. The dogfood CI config references these as `flake_attr = "checks.x86_64-linux.dogfood-build"`. This exercises NixBuildWorker → `nix build .#checks...` → log streaming → artifact collection.

**Why not use the real flake checks**: Building `aspen-node` (658 crates) in a QEMU VM would take hours. Zero-dep crates (`aspen-constants`, `aspen-time`) can build in the Nix sandbox with only the Rust toolchain, keeping the test under 5 minutes.

**Alternative considered**: Keep shell jobs but add a separate `ci-nix-executor-test` — rejected because the point of dogfood is testing the *same* path production uses.

### D2: Shell log streaming via shared `log_bridge` pattern

Extract the `log_bridge` function from `aspen-ci-executor-nix/src/worker.rs` into `aspen-ci-core` (or `aspen-ci-executor-shell/src/common/`) so both `NixBuildWorker` and `LocalExecutorWorker` use the same KV chunk writing logic. `LocalExecutorWorker.execute()` will create an `mpsc::Sender<String>` and forward stdout/stderr lines to it, identical to how `NixBuildWorker` does.

**Alternative considered**: Duplicating the log_bridge code in shell executor — rejected for DRY violation; both workers need identical chunk format, periodic flush, and completion markers.

### D3: Strict job type routing

Change `LocalExecutorWorker.job_types()` to return only `["shell_command", "local_executor"]`. Remove `ci_nix_build`, `ci_vm`, and `cloud_hypervisor` from its claims. If no dedicated worker is registered for a job type, the job stays in the queue (visible via `ci status`) rather than being silently executed by the wrong executor.

**Risk**: Existing single-node deployments without `nix-executor` feature may have `ci_nix_build` jobs that were being handled by the shell executor as a fallback. Mitigation: log a warning when a job type has no registered handler, and document the feature requirements clearly.

### D4: Checkout cleanup on all terminal states

Move cleanup into `PipelineOrchestrator::complete_run()` which is called for all terminal states (success, failed, cancelled). Currently cleanup only happens in `cancel()` and `sync_run_status()` (on poll). The cleanup in `complete_run()` guarantees it happens exactly once regardless of how the pipeline terminates.

### D5: `ignore_paths` via changed-file diffing

When a `TriggerEvent` includes `old_hash`, diff the two commits' trees to get a list of changed file paths. Check each path against `ignore_paths` glob patterns. If ALL changed paths match an ignore pattern, skip the trigger.

**Constraint**: When `old_hash` is None (first push), there's no diff — always trigger. This matches git semantics (no previous state to diff against).

**Implementation location**: Inside `TriggerService::handle_trigger()`, after config is fetched but before pipeline is started. Requires `ForgeNode` access to diff trees.

### D6: Gossip buffering for watch-before-push race

Add a bounded replay buffer (capacity: 32) to `TriggerService` that stores recent `RefUpdate` announcements. When `watch_repo()` is called, replay any buffered announcements for that repo. Buffer entries expire after 30 seconds (covers the typical watch→push gap).

**Alternative considered**: Making `watch_repo()` synchronous with a barrier that blocks push processing — rejected because gossip callbacks are sync and can't block.

### D7: SNIX_GIT_SOURCE compile-time validation

Add a `build.rs` in `aspen-ci` that reads `SNIX_GIT_SOURCE` from `checkout.rs` and compares the rev hash against an environment variable `SNIX_REV` set by the Nix build. If they don't match, emit `cargo:warning=SNIX_GIT_SOURCE mismatch`. For CI, the Nix flake's `snix-src` input provides the authoritative rev.

**Alternative considered**: Runtime check — rejected because the constant is used during checkout, which happens before any runtime validation could run.

## Risks / Trade-offs

- **[D1 VM test flake instability]** → Nix builds in QEMU are slower than shell commands. Mitigate by using zero-dep crates and generous timeouts (300s). The `ci-nix-build-test` already proves this works.
- **[D3 Breaking existing deployments]** → Users running shell executor as catch-all lose implicit Nix job support. Mitigate with clear error message ("no worker registered for job type ci_nix_build — enable nix-executor feature") and migration docs.
- **[D5 Tree diff performance]** → Large repos with many files could make diffing slow. Mitigate with a diff-entry limit (10,000 entries) — if exceeded, always trigger (safe default).
- **[D6 Buffer memory]** → 32 entries × ~200 bytes each = ~6KB. Negligible.
- **[D2 Log volume for shell jobs]** → Shell jobs may produce more output than nix builds. The existing 8KB flush threshold + 500ms periodic timer handles this; `MAX_LOG_SIZE` caps total storage.
