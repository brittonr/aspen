# Extraction Manifest: KV Branch / Commit DAG

## Candidate

- **Family**: KV branch / commit DAG
- **Canonical class**: `service library`
- **Canonical crate/path**: `crates/aspen-kv-branch`, `crates/aspen-commit-dag`
- **Intended audience**: Rust projects that need copy-on-write KV overlays, speculative branch commits, branch fork/diff helpers, and chain-hashed commit history over `aspen-traits::KeyValueStore` without Aspen's Raft runtime, node app, handlers, or concrete transport.
- **Public API owner**: Aspen branch/DAG maintainers
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: reusable branch/history service library family

## Package and release metadata

- **Package description**: Branch overlay and commit-DAG history libraries for Aspen-compatible KV stores.
- **Documentation entrypoint**: Crate-level Rustdoc plus downstream fixture evidence under the active OpenSpec change.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Monorepo path until publication policy is decided.
- **Semver/compatibility policy**: Commit ID/hash behavior is compatibility-sensitive; golden tests must change only with explicit review.
- **Publish readiness**: Blocked; do not mark publishable during this change.

## Feature contract

| Crate | Feature set | Status | Purpose |
| --- | --- | --- | --- |
| `aspen-commit-dag` | default | reusable default | Commit types, BLAKE3 chain/hash helpers, KV-backed commit storage, fork source loading, diff, and bounded GC. |
| `aspen-kv-branch` | default / no-default | reusable default | Copy-on-write branch overlay over `KeyValueStore` with local dirty map, read-set conflict tracking, scan merge, commit/abort behavior, and resource limits. |
| `aspen-kv-branch` | `commit-dag` | reusable opt-in | Adds atomic commit-DAG metadata writes and branch-tip tracking through `aspen-commit-dag`. |
| Aspen consumers | explicit feature paths only | compatibility | Jobs, CI shell executor, deploy, FUSE, docs, and CLI remain final consumers and must not force app/runtime shells into reusable defaults. |

## Dependencies

### Internal Aspen dependencies

| Crate | Dependency | Decision | Reason |
| --- | --- | --- | --- |
| `aspen-commit-dag` | `aspen-raft` | removed | Commit hash helpers are now owned by `aspen-commit-dag::verified::hash`. |
| `aspen-commit-dag` | `aspen-traits`, `aspen-kv-types`, `aspen-constants` | keep | Reusable KV trait/type contracts and bounded GC constants. |
| `aspen-kv-branch` | `aspen-traits`, `aspen-kv-types`, `aspen-constants` | keep | Branch overlay API and request/response types are trait-based and reusable. |
| `aspen-kv-branch` | `aspen-commit-dag` | gated behind `commit-dag` | Commit history is an explicit opt-in feature; default branch overlay avoids it. |

### External dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `blake3`, `hex`, `postcard`, `serde` | keep | Commit IDs, hash hex, commit serialization, and public wire-safe data types. |
| `async-trait`, `tokio`, `dashmap`, `snafu`, `tracing` | keep | Async trait integration, bounded commit timeout, concurrent dirty maps, error context, and observability. |

### Binary/runtime dependencies

No root app, CLI/TUI, web, dogfood, handler, concrete transport, trust, secrets, SQL, cluster bootstrap, or Raft runtime crate is allowed in the reusable default graphs. `aspen-raft` is specifically forbidden from `aspen-commit-dag` and from `aspen-kv-branch --features commit-dag`.

## Compatibility and aliases

- **Old internal helper path**: `aspen_raft::verified::{ChainHash, GENESIS_HASH, hash_to_hex, hash_from_hex, constant_time_compare}` was used inside `aspen-commit-dag` only.
- **New helper path**: `aspen_commit_dag::verified::hash::{ChainHash, GENESIS_HASH, hash_to_hex, hash_from_hex, constant_time_compare}` and re-exports from `aspen_commit_dag::verified`.
- **Compatibility re-exports**: none. Downstream users should import `aspen-commit-dag` and `aspen-kv-branch` directly.
- **Feature compatibility**: `aspen-kv-branch/commit-dag` keeps existing branch commit-DAG behavior and public `CommitResult::commit_id` behavior.
- **Removal criteria**: No temporary compatibility alias is introduced because removed imports were crate-internal.

## Representative consumers

- `aspen-jobs --features kv-branch`
- `aspen-ci-executor-shell --features kv-branch`
- `aspen-deploy --features kv-branch`
- `aspen-fuse --features kv-branch`
- `aspen-docs --features commit-dag-federation`
- `aspen-cli --features commit-dag`
- Downstream fixture under `openspec/changes/extract-kv-branch-commit-dag/fixtures/downstream-branch-dag`

## Representative consumers and re-exporters

- **Representative consumers**: `aspen-jobs`, `aspen-ci-executor-shell`, `aspen-deploy`, `aspen-fuse`, `aspen-docs`, `aspen-cli`, and the downstream branch/DAG fixture.
- **Re-exporters**: compatibility re-exports: none. Existing consumers use direct crate paths or feature-enabled dependency paths.

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-commit-dag` | default | `aspen-commit-dag -> aspen-traits` | Aspen branch/DAG maintainers | Reusable KV trait boundary. |
| `aspen-commit-dag` | default | `aspen-commit-dag -> aspen-kv-types` | Aspen branch/DAG maintainers | Reusable KV request/response types. |
| `aspen-commit-dag` | default | `aspen-commit-dag -> aspen-constants` | Aspen branch/DAG maintainers | Shared bounded limits. |
| `aspen-kv-branch` | default | `aspen-kv-branch -> aspen-traits` | Aspen branch/DAG maintainers | Reusable KV trait boundary. |
| `aspen-kv-branch` | default | `aspen-kv-branch -> aspen-kv-types` | Aspen branch/DAG maintainers | Reusable KV request/response types. |
| `aspen-kv-branch` | default | `aspen-kv-branch -> aspen-constants` | Aspen branch/DAG maintainers | Shared branch resource limits. |
| `aspen-kv-branch` | `commit-dag` | `aspen-kv-branch -> aspen-commit-dag` | Aspen branch/DAG maintainers | Explicit commit history integration feature. |

## Verification rails

- `cargo check -p aspen-commit-dag`
- `cargo check -p aspen-kv-branch`
- `cargo check -p aspen-kv-branch --no-default-features`
- `cargo check -p aspen-kv-branch --features commit-dag`
- `cargo tree -p aspen-commit-dag --edges normal` plus negative boundary grep for `aspen-raft`, root app/runtime shells, handler crates, concrete transport, trust, secrets, and SQL crates
- `cargo tree -p aspen-kv-branch --edges normal`, `--no-default-features`, and `--features commit-dag` plus negative boundary greps proving default avoids `aspen-commit-dag` and all reusable graphs avoid forbidden app/runtime shells
- `rg -n 'aspen_raft::verified|aspen-raft' crates/aspen-commit-dag` proving source imports from the Raft helper surface are absent except historical Verus comments if retained
- positive downstream fixture for canonical branch overlay and commit DAG APIs
- negative boundary metadata checks proving the downstream fixture has no root Aspen, `aspen-raft`, handlers, binaries, or concrete transport crates
- compatibility checks for jobs, CI shell executor, deploy, FUSE, docs, and CLI feature paths
- dependency-boundary checker with `--candidate-family kv-branch-commit-dag`, including mutation checks for forbidden dependency, missing owner, invalid readiness state, and missing downstream fixture evidence

## First-slice status

Current status is `workspace-internal`. The first implementation slice localizes the hash helper surface in `aspen-commit-dag`, removes the normal `aspen-raft` dependency, and proves branch/DAG feature boundaries with compile, tree, fixture, and consumer evidence before any publication readiness state is raised.
