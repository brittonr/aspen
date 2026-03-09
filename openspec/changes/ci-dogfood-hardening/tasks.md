## 1. Job Type Routing Fix (D3)

- [x] 1.1 Change `LocalExecutorWorker.job_types()` in `crates/aspen-ci-executor-shell/src/local_executor/mod.rs` to return only `["shell_command", "local_executor"]`
- [x] 1.2 Update the `test_worker_job_types` test to assert only `shell_command` and `local_executor`
- [x] 1.3 Add a log warning in `JobManager` when a dequeued job type has no registered worker (unhandled type detection)
- [x] 1.4 Verify `NixBuildWorker.job_types()` still returns `["ci_nix_build"]` and `CloudHypervisorWorker.job_types()` returns `[]`

## 2. Shared Log Bridge (D2)

- [x] 2.1 Extract `log_bridge`, `flush_chunk`, `CiLogChunk`, `CiLogCompleteMarker`, and constants (`FLUSH_THRESHOLD`, `FLUSH_INTERVAL_MS`, `CI_LOG_KV_PREFIX`, `CI_LOG_COMPLETE_MARKER`) from `aspen-ci-executor-nix/src/worker.rs` into `aspen-ci-executor-shell/src/common.rs`
- [x] 2.2 Update `NixBuildWorker` to import `log_bridge` from `aspen-ci-executor-shell` instead of its local copy
- [x] 2.3 Verify `NixBuildWorker` tests still pass after the extraction

## 3. Shell Executor Log Streaming

- [x] 3.1 Add `kv_store: Option<Arc<dyn KeyValueStore>>` field to `LocalExecutorWorkerConfig`
- [x] 3.2 In `LocalExecutorWorker.execute()`, create an `mpsc::Sender<String>` and spawn `log_bridge` when `kv_store` and `run_id` are available
- [x] 3.3 Forward stdout and stderr lines to the log sender during command execution
- [x] 3.4 Drop the sender and await the bridge handle after command completion (matching `NixBuildWorker` pattern)
- [ ] 3.5 Write a test: shell job with mock KV store produces log chunks with correct key format
- [ ] 3.6 Write a test: shell job without KV store still captures output in `JobOutput.metadata`

## 4. Checkout Cleanup on Completion (D4)

- [x] 4.1 Add checkout cleanup call in `PipelineOrchestrator::complete_run()` for all terminal states
- [x] 4.2 Ensure cleanup is non-fatal — log warning on failure but don't change pipeline status
- [x] 4.3 Keep cleanup in `sync_run_status()` as belt-and-suspenders (catches timeouts where `complete_run()` isn't called)
- [ ] 4.4 Write a test: pipeline success triggers cleanup of `/tmp/ci-checkout-{run_id}/`

## 5. Dogfood VM Test — Nix Executor Path (D1, Highest Priority)

- [ ] 5.1 Create a Nix overlay in `ci-dogfood.nix` that defines lightweight check derivations (e.g., `checks.x86_64-linux.dogfood-check` that runs `cargo check -p aspen-constants` and `dogfood-test` that runs `cargo test -p aspen-constants`)
- [ ] 5.2 Rewrite the `dogfoodCiConfig` to use `type = 'nix` jobs with `flake_attr` pointing to the overlay checks
- [ ] 5.3 Ensure `NixBuildWorker` is enabled on the test node (verify `nix-executor` feature is compiled in)
- [ ] 5.4 Update the assert in "all pipeline jobs completed" to verify jobs were executed by `NixBuildWorker` (check log chunks exist in KV via `ci logs`)
- [ ] 5.5 Change the "real-time log stream captured output" subtest from diagnostic-only to asserting non-empty log content (Nix executor writes chunks, so this should now pass)
- [ ] 5.6 Run `nix build .#checks.x86_64-linux.ci-dogfood-test --impure` and verify all subtests pass

## 6. `ignore_paths` Evaluation (D5)

- [ ] 6.1 Add `diff_trees(old_hash, new_hash) -> Vec<String>` function to `crates/aspen-ci/src/checkout.rs` (or a new `diff.rs`) that walks two commit trees and returns changed file paths
- [ ] 6.2 Add `matches_ignore_pattern(path, patterns) -> bool` helper using glob matching on `ignore_paths`
- [ ] 6.3 In `TriggerService::handle_trigger()`, after fetching config, compute the diff (if `old_hash` is Some) and check if all changed files match ignore patterns — skip trigger if so
- [ ] 6.4 Add 10,000-entry diff limit — if exceeded, log warning and always trigger
- [ ] 6.5 Write unit test: docs-only push with `ignore_paths = ["*.md", "docs/*"]` skips trigger
- [ ] 6.6 Write unit test: mixed push (md + rs files) triggers normally
- [ ] 6.7 Write unit test: first push (old_hash = None) always triggers

## 7. Watch-Before-Push Race Fix (D6)

- [ ] 7.1 Add `recent_announcements: RwLock<VecDeque<(Instant, PendingTrigger)>>` field to `TriggerService` with capacity 32
- [ ] 7.2 In `CiTriggerHandler.on_announcement()`, when repo is NOT watched, store the announcement in the buffer instead of discarding
- [ ] 7.3 In `watch_repo()`, after inserting repo_id, replay any buffered announcements for that repo (filter by repo_id, remove from buffer, send to trigger_tx)
- [ ] 7.4 Add buffer expiry: in `process_triggers()` or a separate maintenance task, evict entries older than 30 seconds
- [ ] 7.5 Write test: announcement arrives, then watch_repo() triggers replay within 30s
- [ ] 7.6 Write test: announcement older than 30s is not replayed

## 8. SNIX Source Drift Guard (D7)

- [ ] 8.1 Add a Nix flake check (`checks.x86_64-linux.snix-rev-check`) that extracts the rev from `SNIX_GIT_SOURCE` in `checkout.rs` and compares it to the `snix-src` flake input rev
- [ ] 8.2 Verify the check passes with current values
- [ ] 8.3 Add the check to the CI pipeline (either in `.aspen/ci.ncl` or as part of the existing `clippy` check)

## 9. Integration Verification

- [ ] 9.1 Run `cargo nextest run --workspace -P quick` — all tests pass
- [ ] 9.2 Run `nix build .#checks.x86_64-linux.ci-dogfood-test --impure` — dogfood test passes with Nix executor
- [ ] 9.3 Run `nix run .#dogfood-local` on a real machine — full pipeline succeeds
- [ ] 9.4 Update napkin with any new patterns discovered during implementation
