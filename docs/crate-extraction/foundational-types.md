# Extraction Manifest: Foundational Types and Helpers

## Candidate

- **Family**: `foundational-types`
- **Canonical class**: `leaf type/helper`
- **Crates**: `aspen-storage-types`, `aspen-traits`, `aspen-cluster-types`, `aspen-hlc`, `aspen-time`, `aspen-constants`
- **Intended audience**: Rust projects that need Aspen bounded constants, portable clocks/types, KV/storage contracts, cluster address types, or shared traits without Aspen runtime shells.
- **Public API owner**: owner needed
- **Readiness state**: `workspace-internal`

## Package metadata

- **Documentation entrypoint**: crate-level Rustdoc plus this family manifest.
- **License policy**: AGPL-3.0-or-later until human license strategy changes.
- **Repository/homepage policy**: Aspen monorepo path until publication policy is decided.
- **Semver policy**: internal compatibility only; no external semver guarantee yet.
- **Publication policy**: no publishable/repo-split state in this change.

## Feature contract

| Crate | Default contract | Optional adapter/runtime features | First action |
| --- | --- | --- | --- |
| `aspen-constants` | Leaf constants only. | none expected | Document semver and examples. |
| `aspen-hlc` | Portable HLC/timestamp types with `uhlc` default features disabled. | explicit std/runtime only if needed | Prove no rand/getrandom leak. |
| `aspen-storage-types` | Reusable data types only. | Redb definitions must move out. | Split `SM_KV_TABLE` / `redb::TableDefinition`. |
| `aspen-cluster-types` | Alloc-safe defaults. | `iroh` conversion helpers. | Keep `default = []` and prove consumer feature unification. |
| `aspen-traits` | Narrow reusable KV capability traits. | async/runtime blanket impls only behind documented shell/feature. | Split/prove KV capability traits. |
| `aspen-time` | Explicit wall-clock boundary helpers. | simulation helpers as documented. | Keep ambient time owned here. |

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

## First blocker

Move `SM_KV_TABLE` / `redb::TableDefinition` out of `aspen-storage-types`, then split/prove narrower `aspen-traits` KV capabilities.
