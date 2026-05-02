# Jobs/CI Owner/Public API Review Scope

## Review owner

Aspen jobs and CI maintainers.

## Canonical reusable surfaces

- `aspen-jobs-core`: portable job IDs, payloads, priority/status helpers, retry policy, schedule descriptors, job config/spec/result/status helpers, dependency state, deterministic run-state helpers, queue/DLQ stats, keyspace helpers, wire priority/status helpers, and `JobsCoreError`.
- `aspen-ci-core`: CI schema/config/log chunk types, validation helpers, route constants, retry/priority conversion helpers, timeout/resource/trigger helpers, pipeline limit/order helpers, and `CiCoreError`.
- `aspen-jobs-protocol`: alloc/no-std jobs wire DTOs for job lifecycle, queue stats, worker registration/heartbeat/poll/complete flows.
- `aspen-client-api::CI_REQUEST_VARIANTS`: protocol-owned CI operation-name metadata shared by request metadata and CI handler registration.

## Compatibility shells

- `aspen-jobs`: runtime shell and compatibility migration path. Reusable contracts should be imported from `aspen_jobs_core::*` or `aspen_jobs::core::*`; broad runtime root APIs remain workspace-internal unless explicitly reviewed later.
- `aspen-ci`: orchestration/runtime shell. `PipelineStatus::as_str()` is the canonical pipeline status string mapping; broader orchestration APIs remain workspace-internal unless explicitly reviewed later.

## Runtime adapter boundaries excluded from reusable defaults

- Handlers: `aspen-job-handler`, `aspen-ci-handler`, `aspen-rpc-handlers`.
- Executors/workers: `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, `aspen-ci-executor-nix`, and `aspen-jobs-worker-*`.
- Concrete integrations: root `aspen`, node/bootstrap, Iroh transport, Forge/blob runtime wiring, Redb-backed runtime storage, process execution, shell/VM/Nix/SNIX execution, dogfood, CLI/TUI app shells.

## Evidence required before readiness raise

- Fresh downstream metadata: `jobs-ci-core-downstream-metadata.json`.
- Fresh negative boundary evidence: `jobs-ci-core-forbidden-boundary.txt`.
- Fresh focused compatibility evidence: `jobs-ci-core-compatibility.txt`.
- Fresh readiness checker reports: `jobs-ci-core-readiness.json` and `jobs-ci-core-readiness.md`.
- `verification.md` task coverage linking all evidence.

## Current decision

This start slice records the review scope and keeps `jobs-ci-core` at `workspace-internal`. A later task may raise it to `extraction-ready-in-workspace` only after fresh evidence passes and manifest, inventory, and policy are updated consistently. Publishable/repo-split states remain blocked pending human license/publication policy.
