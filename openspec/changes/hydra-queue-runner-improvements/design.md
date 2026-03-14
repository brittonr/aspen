## Context

Aspen's worker health model is a single `HealthStatus` enum (Healthy/Unhealthy/Unknown) with a scalar `load: f32`. The scheduler picks workers via `available_capacity() = 1.0 - load`. This works for basic load balancing but can't detect a machine that's thrashing on I/O with low CPU, or one running low on disk space. The hydra-queue-runner solved this with Linux PSI metrics, disk-free thresholds, and multi-dimensional capacity checks.

Nix builds have three distinct phases — importing requisites, building the derivation, uploading outputs — but Aspen's `NixBuildOutput` and `JobOutput` record none of these durations individually. Diagnosing whether builds are slow because of cache misses (import), compilation (build), or artifact upload requires external instrumentation.

When a derivation fails, every downstream build that depends on it will independently try to build it and fail identically. Hydra avoids this with a `failed_paths` table. Aspen has no equivalent.

## Goals / Non-Goals

**Goals:**

- Workers report PSI pressure (CPU, memory, I/O) and disk-free percentages alongside existing load/health data
- Scheduler rejects workers under pressure even if they have free job slots
- Nix build jobs report import/build/upload phase durations in their `JobOutput` metadata
- Phase timings aggregate per-worker for operational visibility
- Failed derivation output paths are cached in the KV store and checked before dispatch
- All threshold computations and phase timing logic live in `src/verified/` as pure functions

**Non-Goals:**

- Presigned URL uploads for direct-to-cache artifact transfer (future work)
- GC roots management during builds (nix handles this locally)
- Dependency-aware queue sorting by reverse dependency count (existing affinity system is sufficient)
- Job submission rate limiting per worker (existing rate_limit_has_capacity covers this)

## Decisions

### PSI collection via /proc/pressure

Workers read `/proc/pressure/{cpu,memory,io}` on each heartbeat tick. These files exist on any Linux kernel ≥ 4.20. The `avg10` value (10-second average) is used for scheduling decisions — short enough to react to spikes, long enough to avoid noise.

Alternative: Use load averages only. Rejected because load average conflates CPU queue depth with I/O wait, and doesn't reflect memory pressure at all. PSI separates these concerns.

### Extend WorkerStats, not WorkerInfo

`WorkerStats` already carries `load` and `health`. Adding PSI and disk-free fields here keeps the data model clean — `WorkerInfo` is identity/capabilities, `WorkerStats` is runtime state.

New fields on `WorkerStats`:

- `cpu_pressure_avg10: f32` — CPU some avg10
- `memory_pressure_avg10: f32` — Memory full avg10
- `io_pressure_avg10: f32` — I/O full avg10
- `disk_free_build_pct: f64` — Build directory free space percentage
- `disk_free_store_pct: f64` — Nix store free space percentage

### Threshold-based capacity in verified module

Pure function `has_pressure_capacity(stats, thresholds) -> bool` in `aspen-jobs/src/verified/`. The scheduler calls this before `available_capacity()`. If pressure exceeds any threshold, the worker is treated as unavailable regardless of job slot count.

Default thresholds (configurable):

- CPU PSI avg10 > 75.0 → reject
- Memory PSI avg10 > 50.0 → reject
- I/O PSI avg10 > 80.0 → reject
- Build dir free < 5.0% → reject
- Store free < 5.0% → reject

Alternative: Use PSI as a continuous score multiplier on capacity. Rejected because the threshold model is simpler, more predictable, and matches hydra-queue-runner's proven approach.

### Phase timing as JobOutput metadata keys

Rather than changing `NixBuildOutput`'s struct fields, phase durations are written as well-known keys in `JobOutput.metadata`:

- `nix_import_time_ms`
- `nix_build_time_ms`
- `nix_upload_time_ms`

This avoids coupling the general job framework to nix-specific concepts while keeping the data accessible. The CI orchestrator reads these keys for pipeline-level reporting.

Alternative: Add a `NixBuildTimings` struct to `NixBuildOutput`. Rejected because `JobOutput.metadata` already exists for extensible key-value data and other executor types may want their own phase breakdowns.

### Failed paths cache in KV with TTL

Failed derivation output paths are stored at `_ci:failed-paths:{store_path_hash}` with a configurable TTL (default: 24 hours). Before dispatching a nix build, the executor checks this prefix. On a hit, the job is immediately marked `CachedFailure` without scheduling.

Cache entries are keyed by the BLAKE3 hash of the store path (not the full path) to keep keys short and uniform.

TTL prevents stale failures from blocking builds permanently. A manual retry clears the cache entry for that derivation.

Alternative: Store in an in-memory set per node. Rejected because cache must survive node restarts and be visible cluster-wide (a derivation that fails on node 1 shouldn't be retried on node 2).

## Risks / Trade-offs

- **PSI not available on macOS/older Linux**: Workers on systems without `/proc/pressure/` report `0.0` for all PSI values, effectively disabling pressure-based rejection. The capacity check degrades to the existing load-based model.
- **Cached failure false positives**: A derivation that fails due to a transient error (network, OOM) gets cached. Mitigation: TTL expiry + manual cache clear on retry. The `can_cache` flag from build results (like Hydra uses) could be added later to distinguish transient from deterministic failures.
- **Phase timing overhead**: Wrapping each nix build phase in `Instant::now()` / `elapsed()` adds negligible overhead (<1μs per measurement).
- **KV pressure from failure cache**: Each entry is ~100 bytes. Even 10,000 failed paths would be ~1MB. Well within Raft KV capacity.
