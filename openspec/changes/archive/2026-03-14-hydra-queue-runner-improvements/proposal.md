## Why

Aspen's CI nix build workers lack visibility into machine health beyond binary healthy/unhealthy status, have no memory of previously failed derivations, and don't report build phase timing. The hydra-queue-runner project (a Rust rewrite of Hydra's C++ queue runner by helsinki-systems) has solved these problems with production-tested patterns that map directly onto Aspen's architecture.

## What Changes

- Add Linux PSI (Pressure Stall Information) collection and disk-free thresholds to worker heartbeats, enabling the scheduler to avoid dispatching jobs to workers under CPU, memory, or I/O pressure
- Track nix build phase timings (import, build, upload) per job, exposing them in job results and aggregating per-worker statistics for bottleneck diagnosis
- Cache failed derivation output paths in the KV store so future builds of the same derivation skip immediately with a `CachedFailure` status instead of wasting time rebuilding

## Capabilities

### New Capabilities

- `worker-health-psi`: PSI-based worker health reporting and pressure-aware job scheduling
- `build-phase-timing`: Per-phase timing breakdown for nix build jobs (import, build, upload)
- `cached-build-failures`: Derivation failure cache to skip known-bad builds

### Modified Capabilities

- `jobs`: Worker health reporting extends the heartbeat/health model with structured pressure metrics and disk thresholds
- `ci`: Nix build executor gains phase timing instrumentation and cached failure checks before dispatch

## Impact

- **crates/aspen-jobs/**: Worker health model, heartbeat messages, scheduler capacity checks, job result metadata
- **crates/aspen-ci/**: NixBuildWorker executor instrumentation, cached failure lookup on dispatch, phase timing in build results
- **crates/aspen-core/**: New KV key prefixes for failure cache (`_ci:failed-paths:`) and worker health snapshots
- **crates/aspen-coordination/**: No changes expected
- **Verified modules**: Phase timing computation and PSI threshold checks are pure functions suitable for `src/verified/`
