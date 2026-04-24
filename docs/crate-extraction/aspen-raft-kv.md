# Extraction Manifest: aspen-raft-kv

## Candidate

- **Family**: Redb Raft KV
- **Canonical class**: `service library`
- **Canonical crate/path**: future `crates/aspen-raft-kv`
- **Intended audience**: Rust projects that want a reusable replicated KV node facade backed by OpenRaft and Redb without Aspen binary configuration or app bundles.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: reusable service library candidate

## Package and release metadata

- **Package description**: Reusable Redb Raft KV node facade exposing node configuration, membership setup, resource limits, and `KeyValueStore` / `ClusterController` style operations.
- **Documentation entrypoint**: crate-level Rustdoc plus downstream example showing single-node and multi-node setup using canonical APIs.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: monorepo path until publication policy is decided.
- **Semver/compatibility policy**: no external semver guarantee yet; facade configuration and operation traits become semver-relevant once ready.
- **Publish readiness**: blocked; do not mark publishable during this change.

## Feature contract

| Feature set | Status | Purpose |
| --- | --- | --- |
| default | reusable default | Storage/facade contracts and in-process node orchestration without concrete iroh endpoint construction. |
| redb | reusable default or named reusable feature | Use `aspen-redb-storage` backend. |
| iroh-adapter | adapter feature or external crate | Connect to `aspen-raft-network`; not required for storage/facade compile. |
| testing/dev | dev-only | In-memory or deterministic fixtures. |
| trust/secrets/sql/coordination/client-rpc/handler-registry | forbidden by default | Aspen app concerns live in integration crates or explicit opt-in features. |

## OpenRaft boundary

OpenRaft consensus types and traits are allowed as documented public API or implementation detail of this facade. The facade must not expose root Aspen application config as the way to satisfy OpenRaft node identity, membership metadata, or storage settings.

## Dependencies

### Internal Aspen dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `aspen-kv-types` | keep | Reusable KV command/response contract. |
| `aspen-raft-kv-types` | keep | OpenRaft app type config and metadata. |
| `aspen-redb-storage` | keep | Default reusable Redb backend. |
| `aspen-traits` or extracted trait subset | review | Operation traits may be reused, but transitive default leaks must be proven absent. |
| `aspen-constants` / `aspen-time` | keep if leaf-only | Resource limits and explicit time boundaries. |
| `aspen-cluster`, `aspen-core-shell`, `aspen-auth`, `aspen-client-api`, handlers | remove/gate | Aspen app integration belongs in compatibility shells. |

### External dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `openraft` | keep | Consensus engine. |
| `tokio` | keep if facade is async | Runtime shell for consensus tasks; must be explicit. |
| `tracing` / `metrics` | allowed | Observability facade if dependency is documented and optional where reasonable. |

### Binary/runtime dependencies

No binaries. Concrete iroh endpoints are not required for default storage/facade compile.

## Compatibility and aliases

- **Old paths**: reusable portions of `aspen_raft::node::*` and `aspen_raft::RaftNode` construction paths.
- **New path**: `aspen_raft_kv::*`.
- **Compatibility re-exports**: `aspen_raft` re-exports old reusable node paths or migrates all in-repo callers.
- **Owner**: owner needed.
- **Tests**: compile both canonical facade example and Aspen compatibility callers.
- **Removal criteria**: downstream fixture uses `aspen_raft_kv` directly and in-repo consumers no longer need legacy path.

## Representative consumers and re-exporters

- `aspen-raft` compatibility crate
- `aspen-cluster`
- `aspen-core-essentials-handler`
- downstream Redb Raft KV consumer fixture

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-raft-kv` | default | `aspen-raft-kv -> openraft` | owner needed | Consensus engine and public trait boundary. |
| `aspen-raft-kv` | default/redb | `aspen-raft-kv -> aspen-redb-storage` | owner needed | Reusable storage backend. |
| `aspen-raft-kv` | adapter | `aspen-raft-kv -> aspen-raft-network` | owner needed | Explicit iroh adapter integration only. |

## Verification rails

- `cargo check -p aspen-raft-kv --no-default-features`
- `cargo check -p aspen-raft-kv` for default reusable features
- compile proving storage/facade contracts do not construct iroh endpoints
- positive downstream example configuring node identity, membership metadata, storage path, resource limits, and KV operations through reusable types
- negative boundary check proving Aspen binary config, trust/secrets/SQL/coordination/client RPC/handler registry APIs are unavailable by default
- dependency-boundary checker for direct/transitive/representative/re-export leaks
- Aspen compatibility compile for migrated or re-exported `aspen_raft` paths

## First-slice status

Current status is `workspace-internal`. This crate does not exist yet; creation must follow the layer split and OpenRaft boundary review before storage migration proceeds.
