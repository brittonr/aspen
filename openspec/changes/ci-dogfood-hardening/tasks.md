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
- [x] 3.5 Test: shell job with mock KV store produces log chunks (`_ci:logs:{run_id}:{job_id}:{seq}` + `__complete__` marker)
- [x] 3.6 Test: shell job without KV store still captures stdout in JobResult::Success output

## 4. Checkout Cleanup on Completion (D4)

- [x] 4.1 Add checkout cleanup call in `PipelineOrchestrator::complete_run()` for all terminal states
- [x] 4.2 Ensure cleanup is non-fatal — log warning on failure but don't change pipeline status
- [x] 4.3 Keep cleanup in `sync_run_status()` as belt-and-suspenders (catches timeouts where `complete_run()` isn't called)
- [x] 4.4 Tests: `cleanup_checkout` removes directory; nonexistent path is no-op

## 5. Dogfood VM Test — Nix Executor Path (D1, Highest Priority)

- [x] 5.1 Create `dogfoodFlake` in `ci-dogfood.nix` with lightweight check derivations (source-tree, constants-crate, time-crate, workspace-integrity) using `/bin/sh` — no Rust toolchain needed
- [x] 5.2 Rewrite `dogfoodCiConfig` to use `type = 'nix` jobs with `flake_attr` pointing to `checks.x86_64-linux.*`
- [x] 5.3 Verified `NixBuildWorker` enabled via `full-aspen-node-plugins` (has `ci` feature → `nix-executor`)
- [x] 5.4 Updated log chunk diagnostic from "expected for shell worker" to "NixBuildWorker should write them"
- [x] 5.5 Updated "real-time log stream" subtest to expect NixBuildWorker log chunks (non-fatal for timing)
- [x] 5.6 Removed `rustToolChain`/`gcc` dependencies from VM — nix checks use `/bin/sh` only
- [x] 5.7 VM dogfood test passes: all 4 nix jobs succeed via NixBuildWorker, 4/4 jobs have log chunks, 76s total

## 6. `ignore_paths` Evaluation (D5)

- [x] 6.1 Add `list_changed_paths` to `ConfigFetcher` trait (default returns None); implement in `ForgeConfigFetcher` via recursive tree diff
- [x] 6.2 Add `glob_match` and `matches_any_pattern` helpers for path pattern matching (supports `*`, `**`, `*.ext`, `dir/*`, `dir/**`)
- [x] 6.3 Add `should_trigger_for_paths()` evaluating `ignore_paths`/`only_paths` filters
- [x] 6.4 Wire path filter into `handle_trigger()` after ref pattern check, with 10,000-entry diff limit
- [x] 6.5 Write unit test: docs-only push with `ignore_paths = ["*.md", "docs/*"]` skips trigger
- [x] 6.6 Write unit test: mixed push (md + rs files) triggers normally
- [x] 6.7 Tests for: first push (None changed paths) always triggers, empty changed paths skips, only_paths match/no-match, glob patterns (exact, *.ext, dir/*, **,**/*.ext)

## 7. Watch-Before-Push Race Fix (D6)

- [x] 7.1 Add `replay_buffer: RwLock<VecDeque<(Instant, PendingTrigger)>>` field to `TriggerService` with capacity 32
- [x] 7.2 In `CiTriggerHandler.on_announcement()`, when repo is NOT watched, buffer the announcement instead of discarding
- [x] 7.3 In `watch_repo()`, call `replay_buffered_for_repo()` to replay matching entries and send to trigger_tx
- [x] 7.4 Add buffer expiry: evict entries older than 30s during replay (TTL checked in `replay_buffered_for_repo`)
- [x] 7.5 Test: announcement buffered, then watch_repo() replays and drains buffer
- [x] 7.6 Test: buffer stays bounded at MAX_REPLAY_BUFFER (32) with oldest evicted

## 8. SNIX Source Drift Guard (D7)

- [x] 8.1 Add Nix flake check `snix-rev-check` that extracts rev from `SNIX_GIT_SOURCE` in `checkout.rs` and compares to `snix-src` flake.lock rev
- [x] 8.2 Fixed existing drift: updated `SNIX_GIT_SOURCE` from `8fe3bad...` to `180bfc4...` to match flake input
- [x] 8.3 Verified check passes with current values (`nix build .#checks.x86_64-linux.snix-rev-check`)

## 9. Integration Verification

- [x] 9.1 Run `cargo nextest run --workspace -P quick` — 6,422 tests pass (18 new), 201 skipped
- [x] 9.2 VM dogfood test passes end-to-end (Forge → CI trigger → NixBuildWorker → nix build → log streaming → success)
- [ ] 9.3 Run `nix run .#dogfood-local` on a real machine — full pipeline succeeds
- [x] 9.4 Update napkin with patterns discovered during implementation
