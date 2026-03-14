## 1. PSI Collection and Worker Health Model

- [x] 1.1 Add PSI and disk-free fields to `WorkerStats` in `crates/aspen-coordination/src/worker_coordinator/types.rs`: `cpu_pressure_avg10: f32`, `memory_pressure_avg10: f32`, `io_pressure_avg10: f32`, `disk_free_build_pct: f64`, `disk_free_store_pct: f64`
- [x] 1.2 Create `crates/aspen-jobs/src/verified/pressure.rs` with pure functions: `parse_psi_avg10(content: &str) -> f32` and `has_pressure_capacity(cpu: f32, mem: f32, io: f32, disk_build: f64, disk_store: f64, thresholds: &PressureThresholds) -> bool`
- [x] 1.3 Add `PressureThresholds` config struct with defaults (cpu: 75.0, mem: 50.0, io: 80.0, disk: 5.0%) and wire into worker coordinator config
- [x] 1.4 Implement `/proc/pressure/{cpu,memory,io}` reader in `crates/aspen-jobs/src/system_info.rs` — parse `avg10` from `some`/`full` lines, return 0.0 on non-Linux or missing files
- [x] 1.5 Implement disk free percentage reader using `statvfs` for build dir and nix store dir, return 0.0 on failure
- [x] 1.6 Wire PSI and disk-free collection into the worker heartbeat tick — populate the new `WorkerStats` fields on each heartbeat
- [x] 1.7 Unit tests for `parse_psi_avg10` with real `/proc/pressure/` format samples and edge cases (empty, malformed, missing)
- [x] 1.8 Unit tests for `has_pressure_capacity` covering all threshold boundaries

## 2. Pressure-Aware Scheduling

- [x] 2.1 Modify `select_worker` in `crates/aspen-coordination/src/worker_coordinator/load_balancing.rs` to call `has_pressure_capacity()` as an additional filter before capacity ranking
- [x] 2.2 Update `calculate_available_capacity_f32` in `crates/aspen-coordination/src/verified/worker.rs` to return 0.0 when pressure thresholds are exceeded
- [x] 2.3 Add integration test: submit job when only worker exceeds CPU PSI threshold → job stays queued
- [x] 2.4 Add integration test: worker recovers below thresholds → queued job dispatches on next tick

## 3. Nix Build Phase Timing

- [x] 3.1 Create `crates/aspen-ci-executor-nix/src/timing.rs` with `BuildPhaseTimings` struct (import_ms: u64, build_ms: u64, upload_ms: u64) and `to_metadata()` method that returns `HashMap<String, String>`
- [x] 3.2 Add `crates/aspen-jobs/src/verified/timing.rs` with `aggregate_phase_timings(current_totals, new_timing) -> Totals` using saturating arithmetic
- [x] 3.3 Instrument `NixBuildWorker::execute_build` in `crates/aspen-ci-executor-nix/src/executor.rs` — wrap import, build, and upload phases with `Instant::now()` / `elapsed()`
- [x] 3.4 Write phase timing keys (`nix_import_time_ms`, `nix_build_time_ms`, `nix_upload_time_ms`) into `JobOutput.metadata` on build completion
- [x] 3.5 On build failure, write timing for completed phases and `"0"` for skipped phases
- [x] 3.6 Add `total_import_time_ms`, `total_build_time_ms`, `total_upload_time_ms` fields to `WorkerStats` and update them on each nix build completion
- [x] 3.7 Unit tests for `aggregate_phase_timings` including overflow/saturation edge cases
- [x] 3.8 Integration test: run nix build job, verify all three timing keys present in job result metadata

## 4. Cached Build Failures

- [x] 4.1 Add `_ci:failed-paths:` KV prefix constant to `crates/aspen-ci/src/orchestrator/` constants
- [x] 4.2 Create `crates/aspen-ci/src/failure_cache.rs` with `record_failure(store, output_paths, ttl_ms)` and `check_failure(store, output_paths) -> Option<String>` and `clear_failure(store, output_paths)` functions
- [x] 4.3 Implement `record_failure`: BLAKE3-hash each output path, write to `_ci:failed-paths:{hash}` with TTL value in the entry
- [x] 4.4 Implement `check_failure`: scan output paths against failure cache, return first hit
- [x] 4.5 Implement `clear_failure`: delete cache entries for all output paths of a derivation
- [x] 4.6 Hook `check_failure` into the CI pipeline executor before nix build job dispatch — on cache hit, mark job `CachedFailure` and propagate `DepFailed` to dependents
- [x] 4.7 Hook `record_failure` into `NixBuildWorker` on non-retryable build failure
- [x] 4.8 Hook `clear_failure` into the pipeline retry path so manual retries clear stale entries
- [x] 4.9 Add TTL expiry check in `check_failure` — compare entry timestamp + TTL against current time, delete expired entries on read
- [x] 4.10 Unit tests for failure cache: record, check hit, check miss, TTL expiry, clear on retry
- [x] 4.11 Integration test: fail a nix build, submit same derivation again → immediate CachedFailure without worker dispatch

## 5. Wire Verified Modules

- [x] 5.1 Re-export `pressure` and `timing` modules from `crates/aspen-jobs/src/verified/mod.rs`
- [x] 5.2 Add Verus specs for `has_pressure_capacity` in `crates/aspen-jobs/verus/pressure_spec.rs` — ensures result is false when any metric exceeds its threshold
- [x] 5.3 Add Verus specs for `aggregate_phase_timings` in `crates/aspen-jobs/verus/timing_spec.rs` — ensures monotonicity of totals
- [x] 5.4 Run `nix run .#verify-verus` and fix any verification failures
