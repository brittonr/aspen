# Extraction Manifest: aspen-redb-storage

## Candidate

- **Family**: Redb Raft KV
- **Canonical class**: `storage/backend`
- **Canonical crate/path**: `crates/aspen-redb-storage`
- **Intended audience**: Rust projects that need Redb-backed OpenRaft log/state-machine storage, snapshots, chain integrity, CAS, leases, and single-transaction durability for the reusable KV stack.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: reusable storage backend candidate with optional OpenRaft public trait exposure

## Package and release metadata

- **Package description**: Redb storage backend for reusable OpenRaft KV, including unified log/state-machine storage and verified storage helpers.
- **Documentation entrypoint**: crate-level Rustdoc plus storage architecture docs for single-fsync Redb behavior.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: monorepo path until publication policy is decided.
- **Semver/compatibility policy**: no external semver guarantee yet; storage trait and snapshot formats become semver-relevant once ready.
- **Publish readiness**: blocked; do not mark publishable during this change.

## Feature contract

| Feature set | Status | Purpose |
| --- | --- | --- |
| default | reusable default | Pure verified storage helpers and constants only. |
| raft-storage | required named reusable feature | OpenRaft Redb log/state-machine storage implementation. |
| testing/dev | dev-only | Property, crash, integrity, and fixture tests. |
| trust/secrets/sql/coordination | forbidden by default | App/runtime concerns must stay outside this storage backend unless a later design adds named integration features. |

## OpenRaft boundary

When `raft-storage` is enabled, OpenRaft storage traits and associated types are public API. The manifest and checker must classify those paths explicitly. Default pure-helper builds should not require OpenRaft.

## Dependencies

### Internal Aspen dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `aspen-constants` | keep | Provides storage constants without pulling `aspen-core` or app/runtime bundles. |
| `aspen-kv-types` | keep under `raft-storage` | State-machine commands and responses are reusable contract types. |
| `aspen-raft-kv-types` | keep under `raft-storage` | Storage traits need app type configuration. |
| `aspen-time` | review | Time must be injected or isolated; storage helpers should stay deterministic where possible. |

### External dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `redb` | keep under `raft-storage` | Backend purpose. |
| `openraft` | keep under `raft-storage` | Required for storage trait implementation. |
| `blake3` / `hex` | keep | Chain and snapshot integrity. |
| `serde` / `postcard` | keep where needed | Snapshot/log/state serialization. |
| `tokio` / `futures` | keep under `raft-storage` only | OpenRaft async trait implementation. |

### Binary/runtime dependencies

None allowed. No iroh endpoint construction, node bootstrap, handler registry, dogfood, UI, trust, secrets, SQL, or coordination dependencies in default reusable storage features.

## Compatibility and aliases

- **Old paths**: `aspen_raft::storage_shared::*`, `aspen_raft::storage::redb_store::*`, selected storage validation/integrity paths.
- **New path**: `aspen_redb_storage::*`.
- **Compatibility re-exports**: `aspen_raft` re-exports old storage paths during migration.
- **Owner**: owner needed.
- **Tests**: compile and run storage tests through new path and old compatibility path.
- **Removal criteria**: in-repo consumers and downstream fixture use `aspen_redb_storage` directly; old path has no remaining direct imports.

## Representative consumers and re-exporters

- `aspen-raft-kv`
- `aspen-raft` compatibility crate
- `aspen-cluster` through compatibility path
- downstream Redb Raft KV consumer fixture

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-redb-storage` | `raft-storage` | `aspen-redb-storage -> redb` | owner needed | Backend purpose. |
| `aspen-redb-storage` | `raft-storage` | `aspen-redb-storage -> openraft` | owner needed | OpenRaft storage trait implementation. |
| `aspen-redb-storage` | `raft-storage` | `aspen-redb-storage -> aspen-raft-kv-types` | owner needed | App type config for reusable KV stack. |

## Verification rails

- `cargo check -p aspen-redb-storage --no-default-features`
- `cargo check -p aspen-redb-storage --features raft-storage`
- Redb single-transaction log+state proof for append/apply path
- crash-recovery or failure-injection proof that partial log/state commits are not observable
- chain-integrity and snapshot-integrity tests after move
- CAS and lease/TTL regression tests
- dependency-boundary checker for default and `raft-storage` feature sets
- positive downstream storage example using `aspen_redb_storage` directly
- negative boundary check proving iroh/node/handler APIs are unavailable from storage default
- Aspen compatibility compile through `aspen_raft` re-exports

## First-slice status

Current status is `workspace-internal`. The default pure-helper crate now uses leaf `aspen-constants` instead of `aspen-core`, and I9 evidence proves the default storage tree no longer reaches `aspen-core`, `aspen-core-shell`, iroh, transport, or the adapter crate. Baseline evidence still shows concrete Redb/OpenRaft storage modules live in `crates/aspen-raft/src/storage*`; moving those modules and preserving storage safety rails are required before readiness.
