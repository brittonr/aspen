# Extraction Manifest: aspen-raft-kv-types

## Candidate

- **Family**: Redb Raft KV
- **Canonical class**: `protocol/wire`
- **Canonical crate/path**: `crates/aspen-raft-kv-types`, split from current `crates/aspen-raft-types`
- **Intended audience**: Rust projects that need OpenRaft app type configuration, membership metadata, app requests/responses, and storage error types for the reusable KV stack.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: reusable library candidate with public OpenRaft trait/type exposure

## Package and release metadata

- **Package description**: OpenRaft `TypeConfig`, membership metadata, request/response, and error types for the reusable Redb Raft KV stack.
- **Documentation entrypoint**: crate-level Rustdoc explaining which OpenRaft types are public API.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: monorepo path until publication policy is decided.
- **Semver/compatibility policy**: no external semver guarantee yet; OpenRaft exposure must be treated as semver-relevant once ready.
- **Publish readiness**: blocked; do not mark publishable during this change.

## Feature contract

| Feature set | Status | Purpose |
| --- | --- | --- |
| default | reusable default | OpenRaft app type config and KV app data without Aspen app bundles. |
| serde | allowed default if required | Serialization for OpenRaft/log wire data. |
| testing/dev | dev-only | Snapshot/postcard compatibility tests. |

## OpenRaft boundary

Vendored `openraft` 0.10 is a required public/trait dependency for this layer when it exposes `TypeConfig`, app data, log entries, or storage/network trait associated types. The manifest must record every OpenRaft dependency path as public API or implementation detail. Upstream OpenRaft compatibility beyond Aspen's vendored version is deferred.

## Dependencies

### Internal Aspen dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `aspen-kv-types` | keep | Provides reusable transaction result payloads without app runtime. |
| current `aspen-constants` | not used in new default crate | Shared limits can move later if reusable resource policies need them. |
| current `aspen-core` | removed from new default crate | Current legacy dependency blocks neutral type-crate reuse. |
| current `aspen-trust` | removed from new default crate | Trust is app/security integration, not default KV type surface. |

### External dependencies

| Dependency | Decision | Reason |
| --- | --- | --- |
| `openraft` | keep as explicit public trait/type dependency | Required to implement OpenRaft app configuration. |
| `serde` / `postcard` | keep where needed | Wire/log serialization compatibility. |
| `irpc` | review/gate | RPC framework should not be required unless network adapter types need it. |
| `tracing` | review/remove | Logging should not define reusable type surface unless required by macros. |

### Binary/runtime dependencies

None allowed in default reusable features.

## Compatibility and aliases

- **Old path**: `aspen_raft::types::*` and `aspen_raft_types::*`.
- **New path**: `aspen_raft_kv_types::*`.
- **Compatibility re-exports**: `aspen_raft::types::* -> aspen_raft_kv_types::*` during migration.
- **Owner**: owner needed.
- **Tests**: compile both old and new paths until removal.
- **Removal criteria**: all in-repo consumers and downstream fixture import `aspen_raft_kv_types` directly.

## Representative consumers and re-exporters

- `aspen-redb-storage`
- `aspen-raft-kv`
- `aspen-raft-network`
- `aspen-raft` compatibility crate
- downstream Redb Raft KV consumer fixture

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-raft-kv-types` | default | `aspen-raft-kv-types -> openraft` | owner needed | OpenRaft trait/type exposure is the crate purpose and must be semver-documented. |
| `aspen-raft-kv-types` | default | `aspen-raft-kv-types -> aspen-kv-types` | owner needed | KV transaction result payloads are the app-data contract for this reusable layer. |

## Verification rails

- compile current `aspen-raft-types` baseline and `aspen-raft-kv-types` default feature set
- dependency-boundary check proving no default path to root `aspen`, `aspen-core-shell`, trust, secrets, SQL, coordination, handler registries, binaries, or concrete iroh transport
- positive downstream example using `aspen_raft_kv_types` directly
- negative boundary check proving Aspen bootstrap/trust/secrets APIs are unavailable by default
- compatibility compile for `aspen_raft::types::*` until migrated

## First-slice status

Current status is `workspace-internal`. The new `crates/aspen-raft-kv-types` default feature set compiles with OpenRaft and KV type dependencies only. Legacy `crates/aspen-raft-types` still depends on `aspen-core` and `aspen-trust` until compatibility migration completes.
