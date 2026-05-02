# Extraction Manifest: Jobs and CI Core

## Candidate

- **Family**: `jobs-ci-core`
- **Canonical class**: `service library`
- **Crates**: `aspen-jobs-core`, `aspen-jobs`, `aspen-jobs-protocol`, `aspen-jobs-guest`, `aspen-jobs-worker-blob`, `aspen-jobs-worker-maintenance`, `aspen-jobs-worker-replication`, `aspen-jobs-worker-shell`, `aspen-jobs-worker-sql`, `aspen-ci-core`, `aspen-ci`, `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, `aspen-ci-executor-nix`
- **Intended audience**: Aspen and downstream systems that need reusable scheduler/config/run-state/artifact contracts without concrete worker/executor runtime shells.
- **Public API owner**: Aspen jobs and CI maintainers
- **Readiness state**: `extraction-ready-in-workspace`

## Package metadata

- **Documentation entrypoint**: `aspen-ci-core` schema docs, jobs/CI Rustdoc, and this manifest.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Aspen monorepo path until publication policy is decided.
- **Semver policy**: internal compatibility only; schema formats become explicit contracts after fixture goldens land.
- **Publication policy**: no publishable/repo-split state in this change.

## Feature contract

| Surface | Reusable default | Runtime/adapter boundary |
| --- | --- | --- |
| CI schema/config | `aspen-ci-core` structs, Nickel config contract metadata, validation helpers. | Nickel evaluation, SNIX evaluation/build, node/forge integration. |
| Job scheduler/run state | `aspen-jobs-core` deterministic state transitions, IDs, priority, retry policy, schedule descriptors, dependency state, job config/spec/result/status helpers, VM payload schema helpers, keyspace helpers, wire priority/status helpers, and queue/DLQ stats. | `aspen-jobs` worker pools, storage, schedulers, process execution, VM/Nix executors, Iroh/blob runtime. |
| CI/job routing contracts | `aspen-ci-core` job route constants plus priority/retry conversion helpers; `aspen-client-api` CI request variant handles for protocol metadata and handler registration. | `aspen-ci`, `aspen-ci-handler`, executor implementation, Forge/blob/node runtime wiring. |
| Job protocol | domain-schema compatibility only; generic wire extraction is governed by protocol/wire manifest. | handler registries and client/server runtime. |
| Executors/workers | no reusable default. | adapter/runtime crates behind named features or crates. |

## Dependency decisions

- `aspen-jobs-core` is the canonical reusable jobs scheduler/run-state contract surface for deterministic job IDs, specs, status transitions, retry policy, dependency state, queue/DLQ stats, VM payload schema helpers, and jobs keyspace/wire helpers.
- `aspen-ci-core` is the current lightest CI schema surface and owns reusable config/run metadata plus CI job route and retry/priority mapping helpers before `aspen-ci` readiness changes.
- `aspen-client-api` owns CI request variant handle metadata when protocol metadata and handler registration require the same stable operation-name set.
- `aspen-jobs` and `aspen-ci` currently pull runtime crates; reusable logic must move to `aspen-jobs-core` / `aspen-ci-core` or be feature-gated before readiness.
- Worker/executor process spawning, shell, VM, Nix, SNIX, Forge, blob, Iroh, and node bootstrap dependencies are adapter/runtime purpose only.

## Compatibility plan

- Keep existing `aspen-ci` and `aspen-jobs` public paths until consumers migrate.
- `aspen-jobs` re-exports the portable jobs contract under `aspen_jobs::core::*` and provides named crate-root aliases for new deterministic helpers while preserving historical runtime root types.
- Representative consumers: `aspen-ci`, `aspen-jobs`, `aspen-job-handler`, `aspen-ci-handler`, `aspen-dogfood`, `aspen-cli`, executor crates, Forge CI triggers.
- Cross-crate compatibility shims should be removed once call sites can use the owning model/helper directly; CI pipeline status strings now use `PipelineStatus::as_str()` instead of a handler-local compatibility wrapper.
- Every retained runtime edge needs owner, feature/adapter name, test, and removal/retention criteria.

## Owner/public API review checklist

- **Review owner**: Aspen jobs and CI maintainers.
- **Review scope**:
  - `aspen-jobs-core`: job IDs, payloads, priority/status helpers, retry policy, schedule descriptors, job config/spec/result/status helpers, dependency state, deterministic run-state helpers, queue/DLQ stats, keyspace helpers, and wire priority/status helpers.
  - `aspen-ci-core`: CI schema/config/log chunk types, validation helpers, route constants, retry/priority conversion helpers, timeout/resource/trigger helpers, and pipeline validation/order helpers.
  - `aspen-jobs-protocol`: jobs wire DTOs, with serialization compatibility governed by the protocol/wire manifest.
  - `aspen-jobs` and `aspen-ci`: compatibility/runtime shells only; retained re-exports must have owner, tests, and removal/retention criteria.
  - `aspen-client-api`: CI request variant operation-name metadata remains owned by protocol maintainers and is consumed by Jobs/CI handlers.
- **Out of scope for reusable public API**: worker pools, process execution, shell/VM/Nix/SNIX executors, handler registries, node/bootstrap integration, concrete Iroh/blob/Forge runtime wiring, Redb-backed runtime storage, and dogfood/CLI app shells.
- **Review evidence captured for readiness raise**:
  - fresh downstream metadata: `jobs-ci-core-downstream-metadata.json`;
  - fresh negative boundary evidence: `jobs-ci-core-forbidden-boundary.txt`;
  - fresh focused compatibility evidence: `jobs-ci-core-compatibility.txt`;
  - fresh checker reports: `jobs-ci-core-readiness.json` and `jobs-ci-core-readiness.md`;
  - `verification.md` task coverage linking all evidence.
- **Completion status**: owner/public API review completed under `openspec/changes/review-jobs-ci-public-api`; canonical reusable imports are stable inside the workspace, runtime shells remain explicit adapters, compatibility re-exports are documented, and publishable/repo-split labels remain blocked pending license/publication policy.

## Representative consumers

- Canonical portable fixture: `openspec/changes/archive/2026-05-01-extract-jobs-ci-core/fixtures/jobs-ci-core-portable-smoke` depends on `aspen-jobs-core`, `aspen-ci-core`, and `aspen-jobs-protocol` only.
- Compatibility consumers: `aspen-jobs`, `aspen-ci`, `aspen-job-handler`, `aspen-ci-handler`, `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, `aspen-ci-executor-nix`, `aspen-cli`, and `aspen-dogfood`.

## Downstream fixture plan

- Fixture uses canonical scheduler/config/run-state APIs without root `aspen`, handler registries, node bootstrap, shell executors, VM executors, or concrete process/Nix runtime dependencies.
- Fixture includes Nickel config contract examples without invoking runtime Nickel/SNIX evaluators unless explicitly in adapter feature tests.
- Negative fixture/checker mutation rejects shell/VM/Nix executor APIs from reusable defaults.

## Verification rails

- Positive downstream: schema/config/run-state fixture `cargo metadata`, `cargo check`, focused tests.
- Negative boundary: dependency-boundary checker mutation for root app/handler/executor/process-spawn dependency leaks.
- Compatibility: focused `aspen-ci`, `aspen-jobs`, handler, CLI, and dogfood checks named by implementation tasks.

## Completed slices and current blocker

I8 inventory started by treating `aspen-ci-core` and `aspen-jobs-protocol` as the first reusable default surfaces, with `aspen-jobs`, worker crates, CI handlers, shell/VM/Nix executors, and node/runtime integrations retained as adapter shells. I9 added portable fixture metadata plus negative boundary checks rejecting root app, handler, process-spawn, VM, Nix/SNIX executor, and concrete worker imports from reusable defaults. I10 compatibility checks passed for affected jobs/CI consumers and runtime shells.

The post-extraction cleanup stint then moved additional dependency-light contracts and drift-prone string mappings to their owning surfaces:

- `aspen-jobs-core`: `JobPayload`, priority/status wire helpers, jobs keyspace helpers.
- `aspen-ci-core`: CI job route constants and retry/priority conversion helpers.
- `aspen-ci`: `PipelineStatus::as_str()` as the canonical pipeline status string mapping.
- `aspen-client-api`: `CI_REQUEST_VARIANTS` as the canonical CI request operation-name set shared by request metadata and `aspen-ci-handler` registration.
- Final consumers: TUI protocol-to-display DTO conversions remain local to `aspen-tui`.

Jobs/CI core is `extraction-ready-in-workspace` after owner/public API review in `openspec/changes/review-jobs-ci-public-api`. Publishable/repo-split states remain blocked pending human license/publication policy.

## I8 surface inventory

- Reusable default: `aspen-jobs-core` for deterministic job IDs, payloads, scheduler/run-state transitions, retry/dependency state, queue/DLQ stats, keyspace helpers, and wire priority/status helpers.
- Reusable default: `aspen-ci-core` for CI schema/config/log chunks, route constants, retry/priority conversion helpers, and pure validation/timeout/resource/trigger helpers.
- Reusable default: `aspen-jobs-protocol` for alloc/no-std job protocol DTOs.
- Runtime shells: `aspen-jobs`, `aspen-jobs-worker-*`, `aspen-ci`, `aspen-ci-handler`, `aspen-job-handler`, `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, and `aspen-ci-executor-nix` remain explicit adapters/integration crates.
- Evidence: `openspec/changes/archive/2026-05-01-decompose-next-five-crate-families-implementation/evidence/i8-jobs-ci-core-inventory.txt` and `openspec/changes/archive/2026-05-01-decompose-next-five-crate-families-implementation/evidence/i8-jobs-ci-core-surface-inventory.md`.


## I9 fixture boundary

- Positive fixture: `jobs-ci-core-portable-smoke` exercises `aspen-ci-core` pipeline config validation, pure dependency helpers, CI log chunk serialization, and `aspen-jobs-protocol` DTOs.
- Negative fixture: `jobs-ci-runtime-negative` depends only on reusable defaults and proves runtime shells are unavailable by failing imports from `aspen-ci`, handlers, root jobs, shell worker, and shell/VM/Nix executors.
- Evidence: `openspec/changes/archive/2026-05-01-decompose-next-five-crate-families-implementation/evidence/i9-jobs-ci-fixtures.txt`.


## I10 compatibility

- Compatibility command passed for reusable defaults plus `aspen-ci`, `aspen-jobs`, handlers, and shell/VM/Nix executor crates.
- Evidence: `openspec/changes/archive/2026-05-01-decompose-next-five-crate-families-implementation/evidence/i10-jobs-ci-compatibility.txt`.
