# Extraction Manifest: Foundational Types and Helpers

## Candidate

- **Family**: `foundational-types`
- **Canonical class**: `leaf type/helper`
- **Crates**: `aspen-storage-types`, `aspen-traits`, `aspen-cluster-types`, `aspen-hlc`, `aspen-time`, `aspen-constants`
- **Intended audience**: Rust projects that need Aspen bounded constants, portable clocks/types, KV/storage contracts, cluster address types, or shared traits without Aspen runtime shells.
- **Public API owner**: Aspen foundational type maintainers
- **Readiness state**: `extraction-ready-in-workspace`

## Package metadata

- **Documentation entrypoint**: crate-level Rustdoc plus this family manifest.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Aspen monorepo path until publication policy is decided.
- **Semver policy**: internal compatibility only; no external semver guarantee yet.
- **Publication policy**: no publishable/repo-split state in this change.

## Feature contract

| Crate | Public API classification | Canonical imports and compatibility | Readiness evidence |
| --- | --- | --- | --- |
| `aspen-constants` | Reusable API. Leaf Tiger Style constants only. | Canonical import is `aspen_constants::*`; no compatibility shell is required. | Downstream fixture imports bounded API constants without root `aspen`; forbidden-boundary evidence shows no Redb/Iroh/runtime dependency. |
| `aspen-hlc` | Reusable API. HLC/timestamp helpers with `uhlc` defaults disabled for no-default consumers. | Canonical import is `aspen_hlc::{create_hlc, new_timestamp, SerializableTimestamp}`; no compatibility shell is required. | Downstream fixture uses HLC timestamp construction; no-std boundary evidence covers the live `blake3`/`uhlc` graph. |
| `aspen-storage-types` | Reusable API. Portable storage records only. | Canonical import is `aspen_storage_types::KvEntry`; shell-facing Redb table definitions remain in `aspen-core-shell::storage`. | Downstream fixture imports `KvEntry`; no-std boundary and forbidden-boundary evidence confirm `SM_KV_TABLE`/Redb stay out of portable defaults. |
| `aspen-cluster-types` | Reusable API with optional runtime adapter helpers. | Canonical import is `aspen_cluster_types::{NodeId, NodeAddress, NodeTransportAddr, ClusterNode}`; Iroh conversions require the explicit `iroh` feature. | Downstream fixture uses alloc-safe address parts with `default-features = false`; forbidden-boundary evidence confirms no default Iroh/runtime leak. |
| `aspen-traits` | Reusable API plus compatibility type re-exports. | Canonical imports are narrow capability traits and re-exported KV/cluster request types from `aspen_traits`; async trait definitions are behind the `async` feature. | Downstream fixture consumes re-exported `ReadRequest`/`WriteRequest` with `default-features = false`; compatibility evidence covers representative consumers. |
| `aspen-time` | Reusable API for explicit wall-clock boundary helpers. | Canonical imports are `aspen_time::{TimeProvider, current_time_ms, current_time_secs}`; simulation helpers are feature-gated. | Downstream fixture implements `TimeProvider`; compatibility evidence confirms current shell consumers still compile. |

## Dependency decisions

- `redb::TableDefinition` is storage/backend surface, not foundational portable type surface.
- `aspen-traits` must avoid pulling std/runtime shells through blanket impls or default-feature unification.
- `aspen-cluster-types` may expose Iroh address conversion only through explicit opt-in features.
- `aspen-time` owns wall-clock access; lower crates should take explicit time inputs or a provider.

## Compatibility plan

- Keep existing public paths until consumers migrate.
- Any moved table definition or trait blanket impl must record old path, new path, owner, tests, and removal criteria.
- Representative consumers: `aspen-core`, `aspen-core-shell`, `aspen-coordination`, `aspen-commit-dag`, `aspen-kv-branch`, `aspen-jobs`, `aspen-testing-core`.

## Downstream fixture plan

- Fixture depends directly on selected foundational crates, not root `aspen`.
- Fixture demonstrates KV entry/type imports, cluster address type without Iroh defaults, HLC timestamp usage, constants, and trait bounds.
- Negative fixture proves Redb table definitions and runtime Iroh helpers are unavailable without explicit adapter crates/features.

## Verification rails

- Positive downstream: `cargo check -p aspen-core --no-default-features`, `cargo check -p aspen-core-no-std-smoke`, family fixture `cargo metadata` and `cargo check`.
- Negative boundary: dependency-boundary checker mutations for forbidden Redb/runtime/Iroh defaults and representative-consumer feature unification.
- Compatibility: compile consumers named above after any moved path.

## Readiness decision

The foundational family is `extraction-ready-in-workspace`: public API ownership is assigned, canonical imports and compatibility shims are recorded, downstream fixture and negative-boundary evidence pass, and the live no-std checker confirms the current dependency graph. Publishable/repo-split states remain blocked on the global license/publication decision.
