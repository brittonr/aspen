# Extraction Manifest: Testing Harness

## Candidate

- **Family**: `testing-harness`
- **Canonical class**: `service library`
- **Crates**: `aspen-testing-core`, `aspen-testing`, `aspen-testing-fixtures`, `aspen-testing-madsim`, `aspen-testing-network`, `aspen-testing-patchbay`
- **Intended audience**: Aspen and downstream extraction fixtures that need reusable deterministic mocks, workloads, assertions, fixtures, and simulation helpers without full Aspen cluster bootstrap.
- **Public API owner**: architecture-modularity
- **Readiness state**: `workspace-internal`

## Package metadata

- **Documentation entrypoint**: testing crate Rustdoc, test harness docs, and this manifest.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Aspen monorepo path until publication policy is decided.
- **Semver policy**: internal compatibility only until generic fixture API is stable.
- **Publication policy**: no publishable/repo-split state in this change.

## Feature contract

| Surface | Reusable default | Runtime/adapter boundary |
| --- | --- | --- |
| `aspen-testing-core` | deterministic in-memory traits, bounded fixtures, reusable assertion helpers. | none beyond lightweight async/time utilities. |
| `aspen-testing-fixtures` | generic builders and sample data when runtime-free. | app/cluster-specific fixtures behind named features. |
| `aspen-testing` | compatibility facade for existing suites. | root app, raft/blob/cluster/jobs/coordination/forge/CI integrations. |
| `aspen-testing-madsim` | simulation helpers only if runtime dependencies are explicit and documented. | concrete cluster bootstrap, OpenRaft/Iroh runtime setup. |
| network/patchbay crates | adapter-purpose helpers. | namespace, network, concrete transport, and host integration. |

## Dependency decisions

- Reusable defaults must not pull root `aspen`, node bootstrap, handler registries, binary shells, concrete cluster runtime, patchbay namespaces, or Iroh runtime unless the adapter purpose is explicit.
- `aspen-testing-core` is the preferred reusable root for generic mocks/workloads/assertions.
- Existing `aspen-testing` may remain a compatibility facade while finer helpers move/gate.

## Compatibility plan

- Preserve existing madsim, network, and patchbay suites through documented compatibility paths.
- Representative consumers: madsim tests, network tests, patchbay suites, NixOS VM tests, downstream crate-extraction fixtures.
- Every moved helper needs old path/new path, owner, test, and removal/retention criteria.

## Downstream fixture plan

- Fixture uses generic in-memory KV/cluster/workload/assertion helpers without root `aspen` or node bootstrap.
- Fixture proves generated metadata has no handler registry, binary shell, or concrete cluster runtime dependency.
- Negative fixture/checker mutation rejects app-bootstrap and concrete transport leaks in reusable defaults.

## Verification rails

- Positive downstream: fixture `cargo metadata`, `cargo check`, and focused helper tests.
- Negative boundary: dependency-boundary checker mutation for root app, node bootstrap, handler registry, binary shell, concrete cluster runtime, and patchbay leaks.
- Compatibility: existing madsim/network/patchbay suites or generated harness inventory checks named by implementation tasks.

## First blocker

I13 inventory identifies `aspen-testing-core` as the reusable default root for deterministic helpers, bounded assertions, retry/wait utilities, and generic mock state. Generic `aspen-testing-fixtures` builders can remain reusable; `aspen-testing` stays a compatibility facade, while madsim/network/patchbay crates remain explicit adapters for concrete simulation, namespace, transport, and cluster bootstrap behavior.
