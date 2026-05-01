## Why

`aspen-jobs` is still a runtime-heavy crate: its default graph includes Aspen core, coordination, Iroh, Tokio, Redb, chrono, JSON, process/runtime helpers, and worker/VM support. The reusable job model, retry/schedule policy, dependency/run-state helpers, and CI config/run metadata should be consumable without pulling those runtime shells.

This is the next high-ROI decomposition after foundational types, protocol/wire, coordination, and Redb KV. It directly supports self-hosting by making the CI/jobs control-plane contracts independently verifiable while keeping concrete shell, VM, Nix, blob, Iroh, and handler integrations as adapters.

## What Changes

- Introduce a reusable jobs/CI core boundary, expected to be a new `aspen-jobs-core` crate plus any small additions to `aspen-ci-core`.
- Move or duplicate-for-migration pure job contracts: job IDs, priority, retry policy, schedule descriptors, job specs/results/status helpers, dependency state, and deterministic run-state transition helpers.
- Keep `aspen-jobs` as the compatibility/runtime shell that re-exports or wraps the core surface while retaining manager, scheduler service, worker pools, storage, monitoring, VM/plugin, blob, and Iroh-backed features.
- Add downstream and negative fixtures proving the core surface does not depend on root Aspen, handler crates, concrete transport, worker/executor crates, Redb, Tokio runtime services, or process/Nix/VM adapters.
- Update crate-extraction manifest/policy evidence for the jobs/CI family.

## Capabilities

### New Capabilities
- `jobs-ci-core-extraction`: Independent jobs/CI core contracts, dependency boundaries, fixtures, and compatibility evidence for reusable scheduler/config/run-state APIs.

### Modified Capabilities
- `architecture-modularity`: The extraction inventory and checker policy SHALL track the jobs/CI core boundary as a separate reusable service-library candidate from runtime shells.

## Impact

- **Files**: `crates/aspen-jobs-core/` (new), `crates/aspen-jobs/`, `crates/aspen-ci-core/`, workspace `Cargo.toml`, `docs/crate-extraction.md`, `docs/crate-extraction/jobs-ci-core.md`, `docs/crate-extraction/policy.ncl`, `scripts/check-crate-extraction-readiness.rs`, downstream fixtures/evidence.
- **APIs**: New canonical imports from `aspen_jobs_core::*`; existing `aspen_jobs::*` imports remain compatible for this change.
- **Dependencies**: Reusable default surface must avoid runtime shells and concrete adapters; runtime crates may depend on the new core crate.
- **Testing**: Targeted `cargo check`/tests for the new core, downstream fixture checks, negative fixture failure proof, extraction-readiness report, and compatibility checks for jobs/CI consumers.
