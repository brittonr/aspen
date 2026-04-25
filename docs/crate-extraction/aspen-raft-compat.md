# Extraction Manifest: aspen-raft Compatibility

## Candidate

- **Family**: Redb Raft KV
- **Canonical class**: `service library`
- **Canonical crate/path**: `crates/aspen-raft` as Aspen compatibility and integration crate
- **Intended audience**: Aspen in-repo consumers that need existing import paths while reusable Redb Raft KV layers move to canonical crates.
- **Public API owner**: Aspen Raft compatibility maintainers
- **Readiness state**: `workspace-internal`
- **Dependency policy class**: Aspen compatibility/integration shell, not reusable default library candidate

## Package and release metadata

- **Package description**: Aspen Raft compatibility layer and integration shell over reusable KV, storage, network, and app-type crates.
- **Documentation entrypoint**: crate-level Rustdoc listing canonical new paths and temporary compatibility paths.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: monorepo path until publication policy is decided.
- **Semver/compatibility policy**: preserve in-repo compatibility during migration; external stability not promised yet.
- **Publish readiness**: not publishable during this change; app/runtime integration crate may remain workspace-internal indefinitely.

## Feature contract

| Feature set | Status | Purpose |
| --- | --- | --- |
| default | compatibility default | Preserve Aspen current behavior while delegating reusable internals to new crates. |
| sql / coordination / trust / secrets / sharding / testing | opt-in app features | Aspen integration features stay here or in higher integration crates. |
| compatibility-reexports | temporary behavior | Re-export old paths while callers migrate. |

## OpenRaft boundary

`aspen-raft` may expose OpenRaft through compatibility paths, but canonical reusable OpenRaft API ownership moves to `aspen-raft-kv-types`, `aspen-redb-storage`, and `aspen-raft-kv`. Compatibility docs must identify old path, new path, tests, owner, and removal criterion.

## Dependencies

### Internal Aspen dependencies

`aspen-raft` may continue to depend on Aspen app/runtime crates because it is the compatibility shell, but reusable defaults must move to canonical crates and must not require this package through downstream proof.

Current app/runtime dependencies include `aspen-core-shell`, `aspen-auth`, `aspen-client-api`, `aspen-transport`, `aspen-sharding`, optional `aspen-coordination`, optional `aspen-sql`, optional `aspen-trust`, optional `aspen-secrets`, concrete iroh, and handler/client-adjacent types.

### External dependencies

`openraft`, `redb`, `iroh`, `tokio`, metrics/tracing, and serialization dependencies are allowed here because this is the Aspen integration shell.

### Binary/runtime dependencies

No binary target is owned here, but this crate supports node/bootstrap consumers. Runtime dependencies are expected for compatibility, not reusable default proof.

## Compatibility and aliases

| Old path | New canonical path | Tests | Owner | Removal criterion |
| --- | --- | --- | --- | --- |
| `aspen_raft::types::*` | `aspen_raft_kv_types::*` | compile old and new imports | Aspen Raft/KV extraction maintainers | all in-repo consumers and downstream fixture use new path |
| `aspen_raft::storage_shared::*` | `aspen_redb_storage::*` | storage tests through both paths | Aspen Redb storage maintainers | no direct old imports remain |
| `aspen_raft::storage::redb_store::*` | `aspen_redb_storage::*` | Redb log/store tests through both paths | Aspen Redb storage maintainers | no direct old imports remain |
| reusable `aspen_raft::node::*` | `aspen_raft_kv::*` | facade/compat compile tests | Aspen Raft/KV facade maintainers | callers use `aspen_raft_kv` directly |
| reusable `aspen_raft::network::*` | `aspen_raft_network::*` | adapter/compat compile tests | Aspen Raft network adapter maintainers | callers use `aspen_raft_network` directly |

Dependency-key/package aliasing is allowed only when it preserves import paths during migration and is recorded in the relevant manifest with tests and removal criteria.

## Representative consumers and re-exporters

- `aspen-cluster`
- root `aspen` node runtime
- `aspen-core-essentials-handler`
- `aspen-rpc-handlers`
- `aspen-cli`
- dogfood and integration test crates

## Dependency exceptions

| candidate | feature_set | dependency_path | owner | reason |
| --- | --- | --- | --- | --- |
| `aspen-raft-compat` | default | `aspen-raft -> aspen-core-shell` | Aspen Raft compatibility maintainers | Compatibility shell for existing Aspen integration. |
| `aspen-raft-compat` | default | `aspen-raft -> iroh` | Aspen Raft compatibility maintainers | Existing Aspen runtime networking until adapter split completes. |
| `aspen-raft-compat` | trust/secrets/sql/coordination | app feature dependencies | Aspen Raft compatibility maintainers | Explicit Aspen runtime features; not reusable defaults. |

## Verification rails

- compile Aspen compatibility consumers after each path migration
- compile old-path and new-path imports for every compatibility re-export
- dependency-boundary checker proving downstream reusable fixture does not use `aspen-raft` as primary API when canonical crates exist
- positive downstream example using canonical crates first, with `aspen-raft` only as a compatibility consumer
- negative boundary check proving reusable downstream examples do not require `aspen-raft` as their primary API
- node/cluster compatibility checks: `cargo check -p aspen --no-default-features --features node-runtime`, `cargo check -p aspen-cluster`, `cargo check -p aspen-rpc-handlers`
- CLI/dogfood/handler compatibility checks as relevant
- bridge/gateway/web/TUI compatibility checks as relevant

## First-slice status

Current status is `workspace-internal`. This crate is the Aspen app/runtime shell and compatibility bridge by design — it is not a reusable extraction candidate and is not expected to reach `extraction-ready-in-workspace`. Its purpose is to preserve existing import paths while reusable layers move to canonical crates.

Verified compatibility evidence:

- All compatibility re-exports compile (I11/I12: `aspen_raft::storage_shared::*`, `aspen_raft::types::*`, `aspen_raft::raft_kv`, `aspen_raft::raft_kv_types`)
- Node/cluster consumers compile (V7: `evidence/compat-node-cluster.md`)
- CLI/dogfood/handler consumers compile (V8: `evidence/compat-cli-dogfood-handlers.md`)
- Bridge/gateway/web/TUI consumers compile (V9: `evidence/compat-bridges-web-tui.md`)

Removal of compatibility re-exports tracked per entry in the compatibility table above.
