# Extraction Manifest: aspen-raft-kv-types

## Candidate

- **Family**: Redb Raft KV
- **Canonical class**: `protocol/wire`
- **Canonical crate/path**: `crates/aspen-raft-kv-types`, split from current `crates/aspen-raft-types`
- **Intended audience**: Rust projects that need OpenRaft app type configuration, membership metadata, app requests/responses, and storage error types for the reusable KV stack.
- **Public API owner**: Aspen Raft/KV extraction maintainers
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

- **Old API paths**: `aspen_raft::types::*` and `aspen_raft_types::*`.
- **New API path**: `aspen_raft_kv_types::*`.
- **Compatibility re-exports**: `aspen_raft::types::*` re-exports `RaftKvTypeConfig`, `RaftKvRequest`, `RaftKvResponse`, `RaftKvStorageError`, `RaftKvMemberInfo`, `BatchWriteOp`, `BatchCondition`, `TxnCompareSpec`, `TxnCompareOp`, `TxnCompareTarget`, and `TxnOpSpec` from `aspen_raft_kv_types`. Module-level alias `aspen_raft::raft_kv_types` also available.
- **Owner**: Aspen Raft/KV extraction maintainers.
- **Tests**: compile both old and new paths until removal.
- **Removal criteria**: all in-repo consumers and downstream fixture import `aspen_raft_kv_types` directly.

### `aspen-raft-types` package transition plan

The legacy `aspen-raft-types` package is an Aspen app-compatibility package until `I12` migrates or aliases direct consumers. The reusable package/API transition is tracked separately from the `aspen_raft::types::*` re-export path.

| Direct consumer | Current dependency | Transition decision | Dependency-key alias decision | Temporary compatibility / re-export decision | Owner | Verification rail | Removal criteria |
| --- | --- | --- | --- | --- | --- | --- | --- |
| `crates/aspen-raft` | `aspen-raft-types` | Keep as app-compatibility shell until storage migration and non-storage app payloads are split; migrate reusable `AppTypeConfig` use to `aspen-raft-kv-types` in `I12`. | No dependency-key alias in production; add a direct `aspen-raft-kv-types` dependency for reusable app types, keep `aspen-raft-types` only for app-compat payloads during migration. | Keep `aspen_raft::types::*` and `aspen_raft_types::*` compatibility re-exports until `I12` proves old and new import paths compile. | Aspen Raft storage/API owner | `cargo check -p aspen-raft` plus storage migration rails. | No reusable consumer imports `aspen_raft_types::*`; app-only trust payloads stay outside reusable default features. |
| `crates/aspen-raft-network` | `aspen-raft-types` | Keep as explicit adapter dependency during `I9`; migrate reusable adapter type aliases to `aspen-raft-kv-types` or document app-only network types in `I12`. | No dependency-key alias; add a direct `aspen-raft-kv-types` dependency for reusable adapter aliases while keeping `aspen-raft-types` only for tracked app-only network data. | No new compatibility crate; adapter may temporarily re-export canonical aliases behind the `aspen-raft-network` surface until direct consumers migrate. | Aspen Raft network adapter owner | `cargo check -p aspen-raft-network --no-default-features`. | Adapter defaults compile against `aspen-raft-kv-types` without pulling trust/secrets/app bundles, or legacy dependency is manifest-tracked as app-only. |
| `crates/aspen-transport` | `aspen-raft-types` | Leave on app-compatibility package until a transport/RPC extraction change; do not make it part of reusable KV defaults. | No alias in this change; any future dependency-key alias belongs to the transport/RPC extraction plan, not the Redb Raft KV reusable default. | No temporary Redb Raft KV compatibility crate; transport keeps legacy app package until its own manifest owns the migration. | Aspen transport/RPC owner | `cargo check -p aspen-transport`. | Transport/RPC manifest owns any remaining app-type dependency. |
| `crates/aspen-cluster` | `aspen-raft-types` | Migrate reusable membership/type references through `aspen-raft-kv-types` or `aspen-raft` compatibility re-exports after `I10`; cluster bootstrap remains app shell. | No dependency-key alias; migrate reusable imports to direct `aspen-raft-kv-types` where possible and keep app bootstrap imports on compatibility surfaces. | Use `aspen_raft::types::*` only as a temporary re-export during `I12`; do not introduce a standalone compatibility package for cluster. | Aspen cluster bootstrap owner | `cargo check -p aspen-cluster`. | Cluster imports reusable KV types only through canonical crate or tracked compatibility re-export. |
| `crates/aspen-testing-patchbay` | `aspen-raft-types` | Keep as test harness dependency until legacy compatibility imports are migrated in `I12`. | No dependency-key alias; add direct dev/test dependency on `aspen-raft-kv-types` when tests switch to canonical imports. | Temporary compatibility imports are allowed only in migration tests that prove old/new paths; no new compatibility crate. | Aspen testing/patchbay owner | `cargo check -p aspen-testing-patchbay`. | Patchbay harness imports canonical reusable types or documented app-compatibility aliases only. |

No dependency-key alias or temporary compatibility crate/re-export may be removed until every row has passing verification evidence and the downstream reusable fixture imports `aspen_raft_kv_types` directly.

## Representative consumers and re-exporters

- `aspen-redb-storage`
- `aspen-raft-kv`
- `aspen-raft-network`
- `aspen-raft` compatibility crate
- downstream Redb Raft KV consumer fixture

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-raft-kv-types` | default | `aspen-raft-kv-types -> openraft` | Aspen Raft/KV extraction maintainers | OpenRaft trait/type exposure is the crate purpose and must be semver-documented. |
| `aspen-raft-kv-types` | default | `aspen-raft-kv-types -> aspen-kv-types` | Aspen Raft/KV extraction maintainers | KV transaction result payloads are the app-data contract for this reusable layer. |

## Verification rails

- compile current `aspen-raft-types` baseline and `aspen-raft-kv-types` default feature set
- dependency-boundary check proving no default path to root `aspen`, `aspen-core-shell`, trust, secrets, SQL, coordination, handler registries, binaries, or concrete iroh transport
- positive downstream example using `aspen_raft_kv_types` directly
- negative boundary check proving Aspen bootstrap/trust/secrets APIs are unavailable by default
- compatibility compile for `aspen_raft::types::*` until migrated

## First-slice status

Current status is `workspace-internal`. The new `crates/aspen-raft-kv-types` default feature set compiles with OpenRaft and KV type dependencies only. Legacy `crates/aspen-raft-types` still depends on `aspen-core` and `aspen-trust` until compatibility migration completes.
