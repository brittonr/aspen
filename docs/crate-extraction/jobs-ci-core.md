# Extraction Manifest: Jobs and CI Core

## Candidate

- **Family**: `jobs-ci-core`
- **Canonical class**: `service library`
- **Crates**: `aspen-jobs`, `aspen-jobs-protocol`, `aspen-jobs-guest`, `aspen-jobs-worker-blob`, `aspen-jobs-worker-maintenance`, `aspen-jobs-worker-replication`, `aspen-jobs-worker-shell`, `aspen-jobs-worker-sql`, `aspen-ci-core`, `aspen-ci`, `aspen-ci-executor-shell`, `aspen-ci-executor-vm`, `aspen-ci-executor-nix`
- **Intended audience**: Aspen and downstream systems that need reusable scheduler/config/run-state/artifact contracts without concrete worker/executor runtime shells.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`

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
| Job scheduler/run state | deterministic state transitions, IDs, artifact metadata, queue/run models. | workers, shell process execution, VM executor, Nix executor, Iroh/blob runtime. |
| Job protocol | domain-schema compatibility only; generic wire extraction is governed by protocol/wire manifest. | handler registries and client/server runtime. |
| Executors/workers | no reusable default. | adapter/runtime crates behind named features or crates. |

## Dependency decisions

- `aspen-ci-core` is the current lightest schema surface and should absorb reusable config/run metadata before `aspen-ci` readiness changes.
- `aspen-jobs` and `aspen-ci` currently pull root/runtime crates; reusable logic must move or be feature-gated before readiness.
- Worker/executor process spawning, shell, VM, Nix, SNIX, Forge, blob, Iroh, and node bootstrap dependencies are adapter/runtime purpose only.

## Compatibility plan

- Keep existing `aspen-ci` and `aspen-jobs` public paths until consumers migrate.
- Representative consumers: `aspen-ci`, `aspen-jobs`, `aspen-job-handler`, `aspen-ci-handler`, `aspen-dogfood`, `aspen-cli`, executor crates, Forge CI triggers.
- Every retained runtime edge needs owner, feature/adapter name, test, and removal/retention criteria.

## Downstream fixture plan

- Fixture uses canonical scheduler/config/run-state APIs without root `aspen`, handler registries, node bootstrap, shell executors, VM executors, or concrete process/Nix runtime dependencies.
- Fixture includes Nickel config contract examples without invoking runtime Nickel/SNIX evaluators unless explicitly in adapter feature tests.
- Negative fixture/checker mutation rejects shell/VM/Nix executor APIs from reusable defaults.

## Verification rails

- Positive: schema/config/run-state fixture `cargo metadata`, `cargo check`, focused tests.
- Negative: checker mutation for root app/handler/executor/process-spawn dependency leaks.
- Compatibility: focused `aspen-ci`, `aspen-jobs`, handler, CLI, and dogfood checks named by implementation tasks.

## First blocker

Define reusable surface ownership for schema, scheduler, run-state, artifact metadata, and Nickel config contracts, then gate worker runtime/executor/process-spawning/node/handler shells.
